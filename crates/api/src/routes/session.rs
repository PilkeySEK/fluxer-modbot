use std::sync::Arc;

use axum::{Extension, Router, extract::State, middleware, response::NoContent, routing::post};

use crate::{db::schema::SessionData, error::ApiResult, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().route(
        "/session/sign-out",
        post(sign_out).layer(middleware::from_fn_with_state(
            state,
            crate::middleware::session_required,
        )),
    )
}

#[utoipa::path(
    post,
    path = "/session/sign-out",
    responses((status = 200))
)]
pub async fn sign_out(
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
) -> ApiResult<NoContent> {
    state
        .db
        .delete_dash_session(&session_data.session_token)
        .await?;
    Ok(NoContent)
}
