use std::sync::Arc;

use api_types::Guild;
use axum::{Extension, Json, Router, extract::State, middleware, routing::get};

use crate::{db::schema::SessionData, error::ApiResult, state::AppState};

pub fn router(state: AppState) -> Router<AppState> {
    Router::new().route(
        "/users/@me/guilds",
        get(get_user_guilds).layer(middleware::from_fn_with_state(
            state,
            crate::middleware::session_required,
        )),
    )
}

#[utoipa::path(
    get,
    path = "/users/@me/guilds",
    responses((status = 200, body = Vec<api_types::Guild>)),
)]
pub async fn get_user_guilds(
    Extension(session_data): Extension<Arc<SessionData>>,
    State(state): State<AppState>,
) -> ApiResult<Json<Vec<Guild>>> {
    let guilds = <std::vec::Vec<fluxer_neptunium::model::guild::Guild> as fluxer_neptunium::http::endpoints::ResponseBody>::deserialize(
        state
            .http_client
            .get(format!("{}/users/@me/guilds", state.fluxer_api_base))
            .bearer_auth(&session_data.bearer_token)
            .send()
            .await?
            .bytes()
            .await?
            .to_vec(),
    )?;

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
