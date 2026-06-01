use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    middleware,
    routing::{get, patch},
};
use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::{
    db::schema::{GuildConfig, SessionData},
    error::ApiResult,
    middleware::require_guild_manager,
    state::AppState,
};

#[derive(Deserialize, utoipa::ToSchema)]
pub struct GuildUpdates {
    command_prefixes: Option<Vec<String>>,
}

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/guilds/{guild_id}",
            patch(update_guild).layer(middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::session_required,
            )),
        )
        .route(
            "/guilds/{guild_id}",
            get(get_guild_config).layer(middleware::from_fn_with_state(
                state,
                crate::middleware::session_required,
            )),
        )
}

#[utoipa::path(
    patch,
    path = "/guilds/{guild_id}",
    params(
        ("guild_id", description = ""),
    ),
    responses(
        (status = 204),
    )
)]
async fn update_guild(
    Path(guild_id): Path<Id<GuildMarker>>,
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
    Json(guild_updates): Json<GuildUpdates>,
) -> ApiResult<StatusCode> {
    require_guild_manager(guild_id, &state, &session_data).await?;
    if let Some(command_prefixes) = guild_updates.command_prefixes {
        state
            .db
            .update_guild_prefixes(guild_id, &command_prefixes)
            .await?;
    }

    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    get,
    path = "/guilds/{guild_id}",
    params(
        ("guild_id", description = ""),
    ),
    responses(
        (status = 200, body = GuildConfig),
    ),
)]
async fn get_guild_config(
    Path(guild_id): Path<Id<GuildMarker>>,
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
) -> ApiResult<Json<GuildConfig>> {
    require_guild_manager(guild_id, &state, &session_data).await?;
    let guild_config = state.db.get_guild_config_upsert(guild_id).await?;
    Ok(Json(guild_config))
}
