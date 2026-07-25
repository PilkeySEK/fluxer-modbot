use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use axum::{
    Extension, Json, Router,
    extract::State,
    middleware,
    routing::{get, post},
};
use axum_extra::extract::PrivateCookieJar;
use fluxer_neptunium::model::id::{Id, marker::UserMarker};
use reqwest::StatusCode;
use rust_shared::{
    DashboardUser, DashboardUserSettings, Guild,
    db::{UserInfo, UserInfoSchema},
};

use crate::{SESSION_COOKIE_NAME, db::schema::SessionData, error::ApiResult, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/users/@me/guilds",
            get(get_user_guilds).layer(middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::session_required,
            )),
        )
        .route(
            "/users/@me",
            get(get_user_me).layer(middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::session_required,
            )),
        )
        .route(
            "/users/@maybe-me",
            get(get_maybe_user_me).layer(middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::session_optional,
            )),
        )
        .route(
            "/users/info-bulk",
            post(get_user_info).layer(middleware::from_fn_with_state(
                state.clone(),
                crate::middleware::session_required,
            )),
        )
        .route(
            "/users/@me/settings",
            post(set_user_me_settings).layer(middleware::from_fn_with_state(
                state,
                crate::middleware::session_required,
            )),
        )
}

#[utoipa::path(
    get,
    path = "/users/@me/guilds",
    responses((status = 200, body = Vec<rust_shared::Guild>)),
)]
pub async fn get_user_guilds(
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<Guild>>> {
    let guilds = state
        .fluxer_api
        .get_user_guilds(session_data.data.user_id, &session_data.data.bearer_token)
        .await?;

    Ok(Json(
        guilds
            .into_iter()
            .map(|guild| Guild {
                id: guild.id.to_string(),
                name: guild.name,
                icon: guild.icon,
                banner: guild.banner,
                splash: guild.splash,
                vanity_url_code: guild.vanity_url_code,
                owner_id: guild.owner_id.to_string(),
                member_count: guild.member_count,
                online_count: guild.online_count,
            })
            .collect::<Vec<_>>(),
    ))
}

#[utoipa::path(
    get,
    path = "/users/@me",
    responses((status = 200, body = DashboardUser))
)]
pub async fn get_user_me(
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
) -> ApiResult<Json<DashboardUser>> {
    let user = state
        .fluxer_api
        .get_user(&session_data.data.bearer_token)
        .await?;
    let user_settings = state
        .db
        .get_dashboard_user_settings_upsert(session_data.data.user_id)
        .await?;
    Ok(Json((user, user_settings).into()))
}

#[utoipa::path(
    get,
    path = "/users/@maybe-me",
    responses((status = 200, body = Option<DashboardUser>))
)]
pub async fn get_maybe_user_me(
    Extension(session_data): Extension<Arc<Option<SessionData>>>,
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> ApiResult<(PrivateCookieJar, Json<Option<DashboardUser>>)> {
    let Some(session_data) = &*session_data else {
        let jar = jar.remove(SESSION_COOKIE_NAME);
        return Ok((jar, Json(None)));
    };
    let user = state
        .fluxer_api
        .get_user(&session_data.data.bearer_token)
        .await?;
    let user_settings = state
        .db
        .get_dashboard_user_settings_upsert(session_data.data.user_id)
        .await?;
    Ok((jar, Json(Some((user, user_settings).into()))))
}

#[derive(utoipa::ToSchema, serde::Deserialize)]
pub struct UserInfoRequest {
    user_ids: Vec<String>,
}

#[utoipa::path(
    post,
    path = "/users/info-bulk",
    responses((status = 200, body = HashMap<String, UserInfoSchema>)),
)]
pub async fn get_user_info(
    Extension(_session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
    Json(body): Json<UserInfoRequest>,
) -> ApiResult<Json<HashMap<Id<UserMarker>, UserInfo>>> {
    let Some(user_ids) = body
        .user_ids
        .into_iter()
        .map(|s| Id::try_from(s).ok())
        .collect::<Option<HashSet<Id<UserMarker>>>>()
    else {
        return Err(crate::error::ApiErrorResponse::StatusCode(
            StatusCode::BAD_REQUEST,
        ));
    };

    let mut response = HashMap::new();

    for id in user_ids {
        let info = state.db.get_user_info(id).await?;
        if let Some(info) = info {
            response.insert(id, info);
        }
    }

    Ok(Json(response))
}

#[utoipa::path(
    post,
    path = "/users/@me/settings",
    responses((status = 204))
)]
pub async fn set_user_me_settings(
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
    Json(body): Json<DashboardUserSettings>,
) -> ApiResult<StatusCode> {
    state
        .db
        .set_dashboard_user_settings(session_data.data.user_id, body)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
