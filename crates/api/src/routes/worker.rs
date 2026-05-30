use axum::{
    Router,
    extract::{Request, State, WebSocketUpgrade},
    middleware::{Next, from_fn_with_state},
    response::Response,
    routing::any,
};
use reqwest::StatusCode;
use tokio::sync::mpsc::unbounded_channel;

use crate::state::AppState;

mod socket;

pub async fn worker_auth_required(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(authorization) = request.headers().get("Authorization") else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    let Ok(authorization) = authorization.to_str() else {
        return Err(StatusCode::BAD_REQUEST);
    };

    // No reason for this check honestly
    if authorization.len() > 2048 {
        return Err(StatusCode::BAD_REQUEST);
    }

    if authorization.trim() != state.worker_api_token {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/worker/ws", any(ws_upgrade_handler))
        .layer(from_fn_with_state(state, worker_auth_required))
}

async fn ws_upgrade_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> Response {
    let (tx, rx) = unbounded_channel();
    *state.api_to_worker_tx.write().await = Some(tx);
    ws.on_upgrade(|ws| socket::handle_ws(state, ws, rx))
}
