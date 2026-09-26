use std::{ops::ControlFlow, sync::Arc, time::Duration};

use futures::{SinkExt, TryStreamExt};
use rust_shared::ws::{ApiToWorkerMessage, WorkerToApiMessage};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest, http::HeaderValue},
};

use crate::{caches::PrefixCache, db::DatabaseManager};

const RETRY_WAIT_TIME: Duration = Duration::from_mins(1);

pub async fn api_connection(
    url: String,
    worker_token: String,
    db: Arc<DatabaseManager>,
    mut rx: tokio::sync::mpsc::UnboundedReceiver<WorkerToApiMessage>,
    prefix_cache: Arc<PrefixCache>,
) {
    // Wait for API to start up, probably
    tokio::time::sleep(Duration::from_secs(10)).await;

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
        prefix_cache.set_enabled(false);
        tracing::debug!("Connecting to API...");
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
        #[expect(clippy::unwrap_used)]
        if let Err(e) = stream
            .send(Message::Text(
                serde_json::to_string(&WorkerToApiMessage::Connected)
                    .unwrap()
                    .into(),
            ))
            .await
        {
            tracing::error!(
                "Error sending first socket message, reconnecting in {} seconds: {}",
                RETRY_WAIT_TIME.as_secs(),
                e
            );
            tokio::time::sleep(RETRY_WAIT_TIME).await;
            continue 'conn_loop;
        }
        tracing::info!("Connected to API");
        prefix_cache.set_enabled(true);
        loop {
            tokio::select! {
                next = stream.try_next() => {
                    match next {
                        Ok(msg) => {
                            if on_stream_next(msg, &mut stream, &db).await.is_break() {
                                continue 'conn_loop;
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
                msg = rx.recv() => {
                    let Some(msg) = msg else {
                        tracing::debug!("API to worker message sender closed, returning.");
                        break 'conn_loop;
                    };
                    #[expect(clippy::unwrap_used)]
                    if let Err(e) = stream
                        .send(Message::Text(
                            serde_json::to_string(&msg)
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
            }
        }
    }
    prefix_cache.set_enabled(false);
}

async fn on_stream_next(
    msg: Option<Message>,
    stream: &mut WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>,
    db: &DatabaseManager,
) -> ControlFlow<()> {
    let Some(msg) = msg else {
        tracing::error!(
            "Stream ended, reconnecting in {} seconds.",
            RETRY_WAIT_TIME.as_secs()
        );
        tokio::time::sleep(RETRY_WAIT_TIME).await;
        return ControlFlow::Break(());
    };
    if let Message::Close(msg) = msg {
        tracing::error!(
            "Socket closed, reconnecting in {} seconds: {msg:?}",
            RETRY_WAIT_TIME.as_secs()
        );
        tokio::time::sleep(RETRY_WAIT_TIME).await;
        return ControlFlow::Break(());
    }
    let Message::Text(msg) = msg else {
        tracing::error!(
            "Unexpected message encoding, reconnecting in {} seconds.",
            RETRY_WAIT_TIME.as_secs()
        );
        tokio::time::sleep(RETRY_WAIT_TIME).await;
        return ControlFlow::Break(());
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
                    return ControlFlow::Break(());
                }
            }
            ApiToWorkerMessage::HeartbeatRes => {
                tracing::warn!("Unused heartbeat res message received, ignoring it.");
            }
            ApiToWorkerMessage::InvalidateCachedGuildConfig(guild_id) => {
                db.cached_prefixes.invalidate_guild_prefixes(guild_id);
            }
        },
        Err(e) => {
            tracing::error!("Failed to decode message: {e}");
        }
    }

    ControlFlow::Continue(())
}
