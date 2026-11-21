use std::{convert::Infallible, env, sync::Arc};

use color_eyre::eyre::Context;
use color_eyre::eyre::{self, ContextCompat};
use fluxer_neptunium::{
    client::{Client, ClientConfig},
    http::endpoints::channel::AllowedMentions,
};
use pretty_duration::{PrettyDurationOptions, PrettyDurationOutputFormat};
use secrecy::ExposeSecret;
use tokio::sync::mpsc::unbounded_channel;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    api_connection::api_connection, caches::PrefixCache, case_expiration::case_expiry_listener,
    commands::Dispatcher, db::create_db_manager_and_case_expiration_actor,
    event_handler::BotEventHandler, logging::Logger,
};

mod api_connection;
mod caches;
mod case_expiration;
mod commands;
// mod config;
mod db;
mod event_handler;
mod logging;
mod macros;
mod util;

// static PROD: LazyLock<bool> = LazyLock::new(|| {
//     const IS_PROD_ENV: Option<&str> = option_env!("IS_PROD");
//
//     matches!(IS_PROD_ENV, Some("true"))
// });
const GIT_HASH: &str = match option_env!("GIT_HASH") {
    Some(value) => value,
    None => "none", // idk
};
const PRETTY_DURATION_OPTIONS: Option<PrettyDurationOptions> = Some(PrettyDurationOptions {
    output_format: Some(PrettyDurationOutputFormat::Compact),
    singular_labels: None,
    plural_labels: None,
});
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> color_eyre::Result<Infallible> {
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
        env::var("CONFIG_FILE_PATH").unwrap_or_else(|_| String::from("../worker-config.json5"));

    let config = rust_shared::config::Config::load(&config_file_path)
        .await
        .wrap_err_with(|| format!("Failed to open config file at `{config_file_path}`"))?;

    let mut client = Client::create_with_config(
        config.internal.bot_fluxer_token.expose_secret(),
        ClientConfig::builder()
            .default_allowed_mentions(AllowedMentions {
                parse: None,
                users: None,
                roles: None,
                replied_user: None,
            })
            .build(),
    )
    .await?;

    let logger = Arc::new(Logger::new(client.context().clone()));

    let prefix_cache = Arc::new(PrefixCache::new(1024));

    let (db_manager, expired_cases_rx) = create_db_manager_and_case_expiration_actor(
        config.internal.database_url.expose_secret(),
        Arc::clone(&prefix_cache),
        config.public.default_command_prefix,
        Arc::clone(&logger),
    )
    .await
    .context("Error connecting to database")?;

    let db_manager = Arc::new(db_manager);

    let (api_connection_tx, api_connection_rx) = unbounded_channel();
    tokio::spawn(api_connection(
        config.internal.api_worker_ws_url,
        config.internal.worker_api_token,
        Arc::clone(&db_manager),
        api_connection_rx,
        prefix_cache,
    ));

    tokio::spawn(case_expiry_listener(
        expired_cases_rx,
        Arc::clone(&db_manager),
    ));

    let event_handler = BotEventHandler::new(
        Dispatcher::new(Arc::clone(&db_manager), config.default_commands),
        config.public.bot_name,
        db_manager,
        config.public.max_command_prefix_len,
        config.public.max_command_prefixes,
        logger,
        config.public.webhook_avatar,
        client
            .get_user_id_from_token()
            .wrap_err("Failed to get user ID from token")?,
        api_connection_tx,
    );

    client.register_event_handler(event_handler);

    client.start().await.wrap_err("Fatal client error")
}
