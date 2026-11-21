#![cfg_attr(feature = "openapi-gen", expect(unreachable_code))]

use std::{env, sync::Arc};

use axum_extra::extract::cookie::Key;
use base64::{Engine, engine::general_purpose::STANDARD};
use color_eyre::eyre::{self, Context};
use secrecy::ExposeSecret;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::state::{AppState, InnerAppState};

// mod config;
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
async fn main() -> color_eyre::Result<()> {
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
                    eyre::bail!("{LOG_VAR_NAME} environment variable is invalid: {e}");
                }
            },
        )
        .init();

    let config_file_path =
        env::var("API_CONFIG_PATH").unwrap_or_else(|_| String::from("../api-config.json5"));
    let config = rust_shared::config::Config::load(&config_file_path)
        .await
        .wrap_err_with(|| format!("Failed to open config file at `{config_file_path}`"))?;

    let cookie_secret = STANDARD
        .decode(config.internal.cookie_secret.expose_secret())
        .context("Failed to decode cookie secret")?;
    let cookie_key = Key::try_from(cookie_secret.as_slice())
        .context("Failed to create key from cookie secret")?;

    let state = AppState(Arc::new(
        InnerAppState::new(
            config.internal.oauth2,
            config.internal.database_url.expose_secret(),
            config.internal.fluxer_instance_root,
            cookie_key,
            config.public.default_command_prefix,
            config.internal.worker_api_token,
            config.internal.dashboard_base,
            config.public.max_command_prefix_len,
            config.public.max_command_prefixes,
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
