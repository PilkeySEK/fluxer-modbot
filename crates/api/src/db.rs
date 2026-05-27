use std::time::Duration;

use fluxer_neptunium::model::id::{Id, marker::UserMarker};
use sqlx::PgPool;
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
};

use crate::db::schema::SessionData;

pub struct DbManager {
    pool: sqlx::PgPool,
    session_expiry_thread_stop_tx: UnboundedSender<()>,
    session_expiry_thread_handle: JoinHandle<()>,
}

#[derive(Debug)]
pub enum DbError {
    SqlxError(sqlx::Error),
    ParseError(serde_json::Error),
}

impl std::error::Error for DbError {}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SqlxError(e) => f.write_fmt(format_args!("PostgreSQL error: {e}")),
            Self::ParseError(e) => f.write_fmt(format_args!("Parse error: {e}")),
        }
    }
}

/*
#[derive(Debug, Clone)]
pub struct PgSessionStore {
    pool: sqlx::PgPool,
}
*/
impl DbManager {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;

        let (session_expiry_thread_stop_tx, session_expiry_thread_stop_rx) = unbounded_channel();

        let session_expiry_thread_handle = tokio::spawn(session_expiry_thread(
            pool.clone(),
            session_expiry_thread_stop_rx,
        ));

        Ok(Self {
            pool,
            session_expiry_thread_stop_tx,
            session_expiry_thread_handle,
        })
    }

    pub async fn create_dash_session(
        &self,
        session_token: &str,
        user_id: Id<UserMarker>,
        expires_at: chrono::DateTime<chrono::Utc>,
        data: SessionData,
    ) -> Result<(), sqlx::Error> {
        #[expect(clippy::unwrap_used, reason = "This will never fail")]
        sqlx::query!(
            "INSERT INTO dash_sessions (session_token, user_id, data, expires_at)
            VALUES ($1, $2, $3, $4)",
            session_token,
            user_id.into_inner().cast_signed(),
            serde_json::to_value(&data).unwrap(),
            expires_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_session_data_by_session_token(
        &self,
        session_token: &str,
    ) -> Result<Option<SessionData>, DbError> {
        let json = sqlx::query_scalar!(
            "SELECT data FROM dash_sessions
            WHERE session_token = $1 AND expires_at > NOW()",
            session_token,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DbError::SqlxError)?;

        let Some(json) = json else {
            return Ok(None);
        };

        Ok(Some(
            serde_json::from_value(json).map_err(DbError::ParseError)?,
        ))
    }

    pub async fn stop(self) {
        if self.session_expiry_thread_stop_tx.send(()).is_err() {
            return;
        }
        let _ = self.session_expiry_thread_handle.await;
    }
}

async fn session_expiry_thread(
    pool: PgPool,
    mut session_expiry_thread_stop_rx: UnboundedReceiver<()>,
) {
    async fn expire_sessions(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM dash_sessions
            WHERE expires_at < NOW()"
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    let mut interval = tokio::time::interval(Duration::from_secs(60));
    interval.tick().await;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = expire_sessions(&pool).await {
                    tracing::error!("Error expiring sessions: {e}");
                }
            },
            _ = session_expiry_thread_stop_rx.recv() => {
                break;
            }
        }
    }
}

pub mod schema {
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct SessionData {
        pub bearer_token: String,
    }
}
