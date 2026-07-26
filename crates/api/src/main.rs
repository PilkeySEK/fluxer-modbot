use std::sync::Arc;

use axum_extra::extract::cookie::Key;
use base64::{Engine, engine::general_purpose::STANDARD};
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    config::{ApiConfig, ConfigLoadError},
    state::{AppState, InnerAppState},
};

mod config;
mod db;
mod error;
mod fluxer_api;
mod middleware;
#[cfg(feature = "openapi-gen")]
mod openapi;
mod routes;
mod state;

const SESSION_COOKIE_NAME: &str = "session";

#[tokio::main]
async fn main() {
    #[cfg(feature = "openapi-gen")]
    {
        openapi::print_openapi();
        return;
    }
    #[cfg_attr(feature = "openapi-gen", expect(unreachable_code))]
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let config_file_path =
        std::env::var("API_CONFIG_PATH").unwrap_or_else(|_| String::from("../api-config.json5"));
    let config = match ApiConfig::load(config_file_path) {
        Ok(config) => config,
        Err(e) => {
            match e {
                ConfigLoadError::IoError(e) => {
                    tracing::error!("Failed to open or read config file: {e}");
                }
                ConfigLoadError::ParseError(e) => {
                    tracing::error!("Failed to parse config file: {e}");
                }
            }
            return;
        }
    };

    let cookie_secret = match STANDARD.decode(config.cookie_secret) {
        Ok(secret) => secret,
        Err(e) => {
            tracing::error!("Failed to decode cookie secret: {e}");
            return;
        }
    };
    let cookie_key = match Key::try_from(cookie_secret.as_slice()) {
        Ok(key) => key,
        Err(e) => {
            tracing::error!("Failed to create key from cookie secret: {e}");
            return;
        }
    };

    let state = AppState(
        match InnerAppState::new(
            config.oauth2,
            &config.database_url,
            config.fluxer_api_base,
            cookie_key,
            config.dashboard_uri,
            config.default_command_prefix,
            config.worker_api_token,
            config.dashboard_base,
            config.max_command_prefix_len,
            config.max_command_prefixes,
        )
        .await
        {
            Ok(value) => Arc::new(value),
            Err(e) => {
                tracing::error!("Error creating state: {e}");
                return;
            }
        },
    );

    let app = routes::router(state.clone());

    // TODO: Make port configurable
    let listener = match tokio::net::TcpListener::bind("0.0.0.0:3000").await {
        Ok(listener) => listener,
        Err(e) => {
            tracing::error!("Failed to bind: {e}");
            return;
        }
    };

    tracing::info!("Now serving API");

    if let Err(e) = axum::serve(listener, app).await {
        if let Some(inner_state) = Arc::into_inner(state.0)
            && let Some(db) = Arc::into_inner(inner_state.db)
        {
            db.stop().await;
            tracing::debug!("Shutdown database manager");
        }
        tracing::error!("{e}");
    }
}
