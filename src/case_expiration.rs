// TODO: Optimize code here and also it should be able to handle very large amounts of cases by not having all of those in memory at the same time

use std::{sync::Arc, time::Duration};

use chrono::Utc;
use tokio::sync::{
    Mutex,
    mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
};

use crate::db::{DatabaseManager, schema::CaseId};

/// Setting `None` for the time means that the case should be silently dropped (as it was manually closed).
pub type ExpiringCase = (Option<chrono::DateTime<chrono::Utc>>, CaseId);
type StoredExpiringCase = (chrono::DateTime<chrono::Utc>, CaseId);
type Cases = Arc<Mutex<Vec<StoredExpiringCase>>>;

pub fn start_case_expiration_actor(
    existing_cases: Vec<StoredExpiringCase>,
) -> (UnboundedReceiver<CaseId>, UnboundedSender<ExpiringCase>) {
    let (expiring_cases_tx, expiring_cases_rx) = unbounded_channel();
    let (expired_cases_tx, expired_cases_rx) = unbounded_channel();

    let (new_case_added_tx, new_case_added_rx) = unbounded_channel();

    let cases: Cases = Arc::new(Mutex::new(existing_cases));

    tokio::spawn(case_expiration_task(
        Arc::clone(&cases),
        expired_cases_tx,
        new_case_added_rx,
    ));

    tokio::spawn(case_creation_task(
        cases,
        new_case_added_tx,
        expiring_cases_rx,
    ));

    (expired_cases_rx, expiring_cases_tx)
}

async fn case_creation_task(
    cases: Cases,
    new_case_added_tx: UnboundedSender<()>,
    mut expiring_cases_rx: UnboundedReceiver<ExpiringCase>,
) {
    while let Some(new_case) = expiring_cases_rx.recv().await {
        if let (Some(expires_at), case_id) = new_case {
            cases.lock().await.push((expires_at, case_id));
            if new_case_added_tx.send(()).is_err() {
                break;
            }
        } else {
            cases.lock().await.retain(|case| case.1 != new_case.1);
        }
    }
}

async fn case_expiration_task(
    cases: Cases,
    expired_cases_tx: UnboundedSender<CaseId>,
    mut new_case_added_rx: UnboundedReceiver<()>,
) {
    'l: loop {
        let next_expiration = {
            cases
                .lock()
                .await
                .iter()
                .fold(None, |acc, (expires_at, _)| {
                    if let Some(acc) = acc {
                        if expires_at < acc {
                            Some(expires_at)
                        } else {
                            Some(acc)
                        }
                    } else {
                        Some(expires_at)
                    }
                })
                .copied()
        };

        if let Some(next_expiration) = next_expiration {
            let delta_until = next_expiration.signed_duration_since(Utc::now());
            let duration_until = delta_until.to_std().unwrap_or(Duration::ZERO);

            tokio::select! {
                new_case_added_result = new_case_added_rx.recv() => {
                    if new_case_added_result.is_none() {
                        break;
                    }
                    // continue from the start of the loop
                },
                () = tokio::time::sleep(duration_until) => {
                    let expired_cases = get_expired_cases(&cases).await;
                    for (_, case_id) in expired_cases {
                        if expired_cases_tx.send(case_id).is_err() {
                            break 'l;
                        }
                    }
                    // continue from the start of the loop
                }
            }
        } else if new_case_added_rx.recv().await.is_none() {
            break;
        }
    }
}

async fn get_expired_cases(cases: &Cases) -> Vec<StoredExpiringCase> {
    let now = Utc::now();
    cases
        .lock()
        .await
        .extract_if(.., |(expires_at, _)| *expires_at < now)
        .collect()
}

pub async fn case_expiry_listener(mut rx: UnboundedReceiver<CaseId>, db: Arc<DatabaseManager>) {
    while let Some(closed_case_id) = rx.recv().await {
        if let Err(e) = db
            .close_guild_moderation_case_by_id_silently(closed_case_id, Some("Expired."), None)
            .await
        {
            tracing::error!("Database error expiring case {closed_case_id}: {e}");
        }
    }
}
