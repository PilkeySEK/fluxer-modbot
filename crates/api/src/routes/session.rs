use std::sync::Arc;

use axum::{Extension, Router, extract::State, middleware, response::Redirect, routing::get};
use axum_extra::extract::{
    PrivateCookieJar,
    cookie::{Cookie, SameSite},
};

use crate::{SESSION_COOKIE_NAME, db::schema::SessionData, error::ApiResult, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().route(
        "/session/sign-out",
        get(sign_out).layer(middleware::from_fn_with_state(
            state,
            crate::middleware::session_required,
        )),
    )
}

#[utoipa::path(
    get,
    path = "/session/sign-out",
    responses((status = 200))
)]
pub async fn sign_out(
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> ApiResult<(PrivateCookieJar, Redirect)> {
    state
        .db
        .delete_dash_session(&session_data.session_token)
        .await?;
    let jar = jar.remove(
        Cookie::build((SESSION_COOKIE_NAME, (*session_data.session_token).clone()))
            .http_only(true)
            .same_site(SameSite::Lax)
            .path("/")
            .build(),
    );
    Ok((jar, Redirect::to(&state.dashboard_base)))
}
