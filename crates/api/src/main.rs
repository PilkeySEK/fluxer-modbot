#![cfg_attr(feature = "openapi-gen", expect(unreachable_code))]

use std::{env, sync::Arc};

use anyhow::Context;
use axum_extra::extract::cookie::Key;
use base64::{Engine, engine::general_purpose::STANDARD};
use tracing::level_filters::LevelFilter;
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
async fn main() -> anyhow::Result<()> {
    #[cfg(feature = "openapi-gen")]
    {
        openapi::print_openapi();
        return Ok(());
    }

    const LOG_VAR_NAME: &str = "RUST_LOG";
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(
            match EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .parse(env::var(LOG_VAR_NAME).unwrap_or_else(|_| String::new()))
            {
                Ok(layer) => layer,
                Err(e) => {
                    anyhow::bail!("{LOG_VAR_NAME} environment variable is invalid: {e}");
                }
            },
        )
        .init();

    let config_file_path =
        env::var("API_CONFIG_PATH").unwrap_or_else(|_| String::from("../api-config.json5"));
    let config = match ApiConfig::load(config_file_path) {
        Ok(config) => config,
        Err(e) => match e {
            ConfigLoadError::IoError(e) => {
                anyhow::bail!("Failed to open or read config file: {e}");
            }
            ConfigLoadError::ParseError(e) => {
                anyhow::bail!("Failed to parse config file: {e}");
            }
        },
    };

    let cookie_secret = STANDARD
        .decode(config.cookie_secret)
        .context("Failed to decode cookie secret")?;
    let cookie_key = Key::try_from(cookie_secret.as_slice())
        .context("Failed to create key from cookie secret")?;

    let state = AppState(Arc::new(
        InnerAppState::new(
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
        .context("Failed to create state")?,
    ));

    let app = routes::router(state.clone());

    // TODO: Make port configurable
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .context("Failed to bind TCP Listener")?;

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

    Ok(())
}
