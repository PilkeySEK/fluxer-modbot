use std::sync::Arc;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::PrivateCookieJar;
use reqwest::StatusCode;

use crate::{SESSION_COOKIE_NAME, state::AppState};

pub async fn session_required(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let Some(session_token) = jar.get(SESSION_COOKIE_NAME) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    let session_token = session_token.value();

    let maybe_session_data = match state
        .db
        .get_session_data_by_session_token(session_token)
        .await
    {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Error getting session data from session token: {e}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    let Some(session_data) = maybe_session_data else {
        return Err(StatusCode::UNAUTHORIZED);
    };

    request.extensions_mut().insert(Arc::new(session_data));

    Ok(next.run(request).await)
}
