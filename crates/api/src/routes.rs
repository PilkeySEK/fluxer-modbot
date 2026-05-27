use axum::{Router, routing::get};

use crate::state::AppState;

pub mod oauth2;
pub mod users;

pub fn router(state: AppState) -> Router {
    Router::new()
        .merge(oauth2::router())
        .merge(users::router(state.clone()))
        .route("/", get(|| async { "OK" }))
        .with_state(state)
}
