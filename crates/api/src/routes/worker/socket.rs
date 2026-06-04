use std::{ops::ControlFlow, time::Duration};

use api_types::ws::{
    ApiToWorkerMessage, WORKER_API_HEARTBEAT_INTERVAL, WorkerApiWsCloseCode, WorkerToApiMessage,
};
use axum::extract::ws::{CloseFrame, Message, WebSocket};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::state::AppState;

struct ApiWorkerSocket(WebSocket);

impl ApiWorkerSocket {
    async fn send(&mut self, message: ApiToWorkerMessage) -> ControlFlow<()> {
        #[expect(clippy::unwrap_used)]
        if let Err(e) = self
            .0
            .send(Message::text(serde_json::to_string(&message).unwrap()))
            .await
        {
            tracing::error!("Error sending websocket message to worker: {e}");
            ControlFlow::Break(())
        } else {
            ControlFlow::Continue(())
        }
    }

    async fn recv(&mut self) -> ControlFlow<(), WorkerToApiMessage> {
        let message = self
            .0
            .recv()
            .await
            .map_or(ControlFlow::Break(()), ControlFlow::Continue)?;
        match message {
            Ok(Message::Text(s)) => {
                let s = s.as_str();
                match serde_json::from_str(s) {
                    Ok(message) => ControlFlow::Continue(message),
                    Err(e) => {
                        tracing::error!("Failed to parse socket message: {e}");
                        ControlFlow::Break(())
                    }
                }
            }
            Ok(other) => {
                tracing::error!("Invalid message encoding: {other:?}");
                ControlFlow::Break(())
            }
            Err(e) => {
                tracing::error!("Error receiving socket message: {e}");
                ControlFlow::Break(())
            }
        }
    }

    async fn close(&mut self) {
        let _ = tokio::time::timeout(
            Duration::from_secs(30),
            self.0.send(Message::Close(Some(CloseFrame {
                code: WorkerApiWsCloseCode::HeartbeatTimeout as u16,
                reason: "Heartbeat timeout.".into(),
            }))),
        )
        .await;
    }
}

pub(super) async fn handle_ws(
    state: AppState,
    socket: WebSocket,
    mut rx: UnboundedReceiver<ApiToWorkerMessage>,
) {
    let mut socket = ApiWorkerSocket(socket);
    let mut interval = tokio::time::interval(WORKER_API_HEARTBEAT_INTERVAL);
    interval.tick().await; // The first tick completes immediately
    let mut expecting_heartbeat_res = false;
    let mut cache_enabled = false;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let ControlFlow::Continue(msg) = msg else {
                    tracing::debug!("handle_ws returning");
                    break;
                };
                if !cache_enabled {
                    cache_enabled = true;
                    state.fluxer_api.set_cache_enabled(true);
                }

                match msg {
                    WorkerToApiMessage::HeartbeatRes => {
                        expecting_heartbeat_res = false;
                    }
                    WorkerToApiMessage::Connected => {}
                    WorkerToApiMessage::HeartbeatReq => {
                        if socket.send(ApiToWorkerMessage::HeartbeatRes).await.is_break() {
                            break;
                        }
                    }
                    WorkerToApiMessage::InvalidateCachedGuildPermissions(guild_id) => {
                        state.fluxer_api.invalidate_cached_guild_permissions(guild_id);
                    }
                    WorkerToApiMessage::InvalidateCachedGuildPermissionsForUser(guild_id, user_id) => {
                        state.fluxer_api.invalidate_cached_guild_permissions_for_user(guild_id, user_id).await;
                    }
                }
            },
            _ = interval.tick() => {
                if expecting_heartbeat_res {
                    tracing::error!("Did not receive heartbeat response, returning from socket handler.");
                    socket.close().await;
                    break;
                }
                if socket.send(ApiToWorkerMessage::HeartbeatReq).await.is_break() {
                    break;
                }
                expecting_heartbeat_res = true;
            },
            msg = rx.recv() => {
                let Some(msg) = msg else {
                    tracing::debug!("handle_ws returning");
                    break;
                };
                if socket.send(msg).await.is_break() {
                    break;
                }
            }
        }
    }

    state.fluxer_api.set_cache_enabled(false);
}
