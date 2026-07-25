use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Path, Query, State},
    middleware,
    routing::{get, patch},
};
use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use reqwest::StatusCode;
use rust_shared::db::{GuildModerationCase, GuildModerationCaseResponse};
use serde::Deserialize;

use crate::{
    db::schema::{GuildConfig, GuildUpdates, SessionData},
    error::ApiResult,
    middleware::require_guild_manager,
    state::AppState,
};

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
                state.clone(),
                crate::middleware::session_required,
            )),
        )
        .route(
            "/guilds/{guild_id}/cases",
            get(get_guild_moderation_cases).layer(middleware::from_fn_with_state(
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
        (status = 200, body = GuildConfig),
    )
)]
async fn update_guild(
    Path(guild_id): Path<Id<GuildMarker>>,
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
    Json(mut guild_updates): Json<GuildUpdates>,
) -> ApiResult<Json<GuildConfig>> {
    require_guild_manager(guild_id, &state, &session_data).await?;

    if let Some(command_prefixes) = &mut guild_updates.command_prefixes {
        if command_prefixes.len() > state.max_command_prefix_len {
            return Err(crate::error::ApiErrorResponse::StatusCode(
                StatusCode::BAD_REQUEST,
            ));
        }
        if let Some(trimmed_command_prefixes) = command_prefixes
            .iter_mut()
            .map(|prefix| {
                let trimmed = prefix.trim();
                if trimmed.is_empty() || trimmed.len() > state.max_command_prefix_len {
                    None
                } else {
                    Some(trimmed.to_owned())
                }
            })
            .collect::<Option<Vec<String>>>()
        {
            *command_prefixes = trimmed_command_prefixes;
        } else {
            return Err(crate::error::ApiErrorResponse::StatusCode(
                StatusCode::BAD_REQUEST,
            ));
        }
    }

    Ok(Json(state.db.update_guild(guild_id, guild_updates).await?))
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

#[derive(Deserialize)]
pub struct ModerationCasesQuery {
    page: Option<i64>,
    search: Option<String>,
}

#[derive(utoipa::ToSchema)]
#[expect(unused)]
pub struct GuildModerationCasesResponseSchema {
    cases: Vec<GuildModerationCaseResponse>,
    total: i64,
    has_next: bool,
    per_page: i64,
}

#[derive(serde::Serialize)]
pub struct GuildModerationCasesResponse {
    cases: Vec<GuildModerationCase>,
    total: i64,
    has_next: bool,
    per_page: i64,
}

// TODO: Paging support
#[utoipa::path(
    get,
    path = "/guilds/{guild_id}/cases",
    params(
        ("guild_id", Path),
        ("search" = Option<String>, Query),
        ("page" = Option<i64>, Query),
    ),
    responses(
        (status = 200, body = GuildModerationCasesResponseSchema),
    ),
)]
async fn get_guild_moderation_cases(
    Path(guild_id): Path<Id<GuildMarker>>,
    Query(query): Query<ModerationCasesQuery>,
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
) -> ApiResult<Json<GuildModerationCasesResponse>> {
    const ENTRIES_PER_PAGE: i64 = 20;

    let page = query.page.unwrap_or(0);

    if page < 0 {
        return Err(StatusCode::BAD_REQUEST.into());
    }

    require_guild_manager(guild_id, &state, &session_data).await?;

    let count = state
        .db
        .count_guild_moderation_cases_by_query(guild_id, query.search.as_deref())
        .await?;

    let offset_num = ENTRIES_PER_PAGE * page;
    let offset = if offset_num == 0 {
        None
    } else {
        Some(offset_num)
    };
    let cases = state
        .db
        .list_guild_moderation_cases_by_query(
            guild_id,
            ENTRIES_PER_PAGE,
            offset,
            query.search.as_deref(),
        )
        .await?;

    Ok(Json(GuildModerationCasesResponse {
        cases,
        total: count,
        has_next: offset_num + ENTRIES_PER_PAGE < count,
        per_page: ENTRIES_PER_PAGE,
    }))
}
