use std::{env, sync::Arc};

use anyhow::Context;
use fluxer_neptunium::{
    client::{Client, ClientConfig},
    http::endpoints::channel::AllowedMentions,
};
use pretty_duration::{PrettyDurationOptions, PrettyDurationOutputFormat};
use tokio::sync::mpsc::unbounded_channel;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    api_connection::api_connection,
    case_expiration::case_expiry_listener,
    commands::Dispatcher,
    config::{Config, ConfigExt, DefaultCommandConfig},
    db::create_db_manager_and_case_expiration_actor,
    event_handler::BotEventHandler,
    logging::Logger,
};

mod api_connection;
mod caches;
mod case_expiration;
mod commands;
mod config;
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
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(EnvFilter::from_default_env())
        .init();

    let config_file_path =
        env::var("CONFIG_FILE_PATH").unwrap_or_else(|_| String::from("../worker-config.json5"));
    let default_command_config_file_path = env::var("DEFAULT_COMMAND_NAMES_FILE_PATH")
        .unwrap_or_else(|_| String::from("../default-command-names.json5"));

    let (config, default_command_config) = match tokio::try_join!(
        async {
            Config::load_from_file(&config_file_path)
                .await
                .with_context(|| format!("Failed to load config `{config_file_path}`"))
        },
        async {
            DefaultCommandConfig::load_from_file(&default_command_config_file_path)
                .await
                .with_context(|| {
                    format!(
                        "Failed to load default command config from `{default_command_config_file_path}`"
                    )
                })
        }
    ) {
        Ok(values) => values,
        Err(e) => {
            return Err(e);
        }
    };

    let mut client = Client::new_with_config(
        config.token,
        ClientConfig::builder()
            .default_allowed_mentions(AllowedMentions {
                parse: Some(Vec::new()),
                users: Some(Vec::new()),
                roles: Some(Vec::new()),
                replied_user: false,
            })
            .build(),
    );

    let logger = Arc::new(Logger::new(client.context().clone()));

    let (db_manager, expired_cases_rx) = create_db_manager_and_case_expiration_actor(
        &config.database_url,
        config.prefix_cache_capacity,
        config.default_command_prefix,
        Arc::clone(&logger),
    )
    .await
    .context("Error connecting to database")?;

    let db_manager = Arc::new(db_manager);

    let (api_connection_tx, api_connection_rx) = unbounded_channel();
    tokio::spawn(api_connection(
        config.api_worker_ws_url,
        config.worker_api_token,
        Arc::clone(&db_manager),
        api_connection_rx,
    ));

    tokio::spawn(case_expiry_listener(
        expired_cases_rx,
        Arc::clone(&db_manager),
    ));

    let event_handler = BotEventHandler::new(
        Dispatcher::new(Arc::clone(&db_manager), default_command_config),
        config.bot_name,
        db_manager,
        config.max_command_prefix_len,
        config.max_command_prefixes,
        logger,
        config.webhook_avatar_b64,
        config.bot_id,
        api_connection_tx,
    );

    client.register_event_handler(event_handler);

    if let Err(e) = client.start().await {
        tracing::error!("Fatal client error: {e}");
    }

    Ok(())
}
