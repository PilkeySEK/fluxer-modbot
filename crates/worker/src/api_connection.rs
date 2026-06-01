use std::{sync::Arc, time::Duration};

use api_types::ws::{ApiToWorkerMessage, WorkerToApiMessage};
use futures::{SinkExt, TryStreamExt};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest, http::HeaderValue};

use crate::db::DatabaseManager;

#[expect(clippy::too_many_lines)]
pub async fn api_connection(url: String, worker_token: String, db: Arc<DatabaseManager>) {
    const RETRY_WAIT_TIME: Duration = Duration::from_mins(1);

    // Wait for API to start up, probably
    tokio::time::sleep(Duration::from_secs(15)).await;

    let mut request = match url.into_client_request() {
        Ok(request) => request,
        Err(e) => {
            tracing::error!("Failed to convert url to client request: {e}");
            return;
        }
    };
    request.headers_mut().insert(
        "Authorization",
        match HeaderValue::from_str(&worker_token) {
            Ok(value) => value,
            Err(e) => {
                tracing::error!("Invalid header value: {e}");
                return;
            }
        },
    );
    'conn_loop: loop {
        let mut stream = match tokio_tungstenite::connect_async(request.clone()).await {
            Ok((stream, _response)) => stream,
            Err(e) => {
                tracing::error!(
                    "Error connecting to `{}`, reconnecting in {} seconds: {}",
                    request.uri(),
                    RETRY_WAIT_TIME.as_secs(),
                    e
                );
                tokio::time::sleep(RETRY_WAIT_TIME).await;
                continue 'conn_loop;
            }
        };
        loop {
            match stream.try_next().await {
                Ok(msg) => {
                    let Some(msg) = msg else {
                        tracing::error!(
                            "Stream ended, reconnecting in {} seconds.",
                            RETRY_WAIT_TIME.as_secs()
                        );
                        tokio::time::sleep(RETRY_WAIT_TIME).await;
                        continue 'conn_loop;
                    };
                    if let Message::Close(msg) = msg {
                        tracing::error!(
                            "Socket closed, reconnecting in {} seconds: {msg:?}",
                            RETRY_WAIT_TIME.as_secs()
                        );
                        tokio::time::sleep(RETRY_WAIT_TIME).await;
                        continue 'conn_loop;
                    }
                    let Message::Text(msg) = msg else {
                        tracing::error!(
                            "Unexpected message encoding, reconnecting in {} seconds.",
                            RETRY_WAIT_TIME.as_secs()
                        );
                        tokio::time::sleep(RETRY_WAIT_TIME).await;
                        continue 'conn_loop;
                    };
                    match serde_json::from_str(msg.as_str()) {
                        Ok(msg) => match msg {
                            ApiToWorkerMessage::HeartbeatReq => {
                                #[expect(clippy::unwrap_used)]
                                if let Err(e) = stream
                                    .send(Message::Text(
                                        serde_json::to_string(&WorkerToApiMessage::HeartbeatRes)
                                            .unwrap()
                                            .into(),
                                    ))
                                    .await
                                {
                                    tracing::error!(
                                        "Error sending heartbeat res, reconnecting in {} seconds: {e}",
                                        RETRY_WAIT_TIME.as_secs()
                                    );
                                    tokio::time::sleep(RETRY_WAIT_TIME).await;
                                    continue 'conn_loop;
                                }
                            }
                            ApiToWorkerMessage::HeartbeatRes => {
                                tracing::warn!(
                                    "Unused heartbeat res message received, ignoring it."
                                );
                            }
                            ApiToWorkerMessage::InvalidateGuildPrefixes(guild_id) => {
                                db.cached_prefixes.invalidate_guild_prefixes(guild_id);
                            }
                        },
                        Err(e) => {
                            tracing::error!("Failed to decode message: {e}");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Error receiving next socket message, reconnecting in {} seconds: {}",
                        RETRY_WAIT_TIME.as_secs(),
                        e
                    );
                    tokio::time::sleep(RETRY_WAIT_TIME).await;
                    continue 'conn_loop;
                }
            }
        }
    }
}
