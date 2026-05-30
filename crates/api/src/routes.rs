use axum::{Router, routing::get};

use crate::state::AppState;

pub mod guild_config;
pub mod oauth2;
pub mod users;
pub mod worker;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(oauth2::router())
        .merge(users::router(state.clone()))
        .merge(guild_config::router(state.clone()))
        .merge(worker::router(state.clone()))
        .route("/", get(|| async { "OK" }))
        .with_state(state)
}
