use std::sync::Arc;

use api_types::is_guild_manager_permissions;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use axum_extra::extract::PrivateCookieJar;
use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use reqwest::StatusCode;

use crate::{SESSION_COOKIE_NAME, db::schema::SessionData, error::ApiResult, state::AppState};

async fn require_session(
    state: &AppState,
    jar: &PrivateCookieJar,
) -> Result<SessionData, StatusCode> {
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

    Ok(session_data)
}

pub async fn session_required(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
    mut request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let session_data = require_session(&state, &jar).await?;

    request.extensions_mut().insert(Arc::new(session_data));

    Ok(next.run(request).await)
}

pub async fn require_guild_manager(
    guild_id: Id<GuildMarker>,
    state: &AppState,
    session: &Arc<SessionData>,
) -> ApiResult<()> {
    let permissions = state
        .db
        .get_guild_member_permissions(guild_id, session.user_id)
        .await?;

    let Some((permissions, is_guild_owner)) = permissions else {
        return Err(StatusCode::FORBIDDEN.into());
    };

    if !is_guild_owner && !is_guild_manager_permissions(permissions) {
        return Err(StatusCode::FORBIDDEN.into());
    }

    Ok(())
}
