use std::{env, str::FromStr, sync::Arc};

use async_brigadier::CommandDispatcher;
use fluxer_neptunium::{
    client::{Client, ClientConfig},
    http::endpoints::channel::AllowedMentions,
};
use pretty_duration::{PrettyDurationOptions, PrettyDurationOutputFormat};
use tokio::sync::mpsc::unbounded_channel;
use tracing::Level;

use crate::{
    api_connection::api_connection,
    case_expiration::case_expiry_listener,
    commands::register_commands,
    config::{Config, ConfigLoadError},
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
async fn main() {
    let config_file_path =
        env::var("CONFIG_FILE_PATH").unwrap_or_else(|_| String::from("../worker-config.json5"));

    let config = match Config::load(&config_file_path) {
        Ok(config) => config,
        Err(e) => {
            match e {
                ConfigLoadError::Io(e) => {
                    println!("I/O error loading config at {config_file_path}: {e}");
                }
                ConfigLoadError::Parse(e) => {
                    println!("Failed to parse config at {config_file_path}: {e}");
                }
            }
            return;
        }
    };

    let log_level = match Level::from_str(&config.log_level) {
        Ok(level) => level,
        Err(e) => {
            println!("Failed to parse log level from config: {e}");
            return;
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

    tracing_subscriber::fmt().with_max_level(log_level).init();

    let (db_manager, expired_cases_rx) = match create_db_manager_and_case_expiration_actor(
        &config.database_url,
        config.prefix_cache_capacity,
        config.default_command_prefix,
        Arc::clone(&logger),
    )
    .await
    {
        Ok(value) => value,
        Err(e) => {
            tracing::error!("Error connecting to database: {e}");
            return;
        }
    };

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

    let mut dispatcher = CommandDispatcher::new();
    register_commands(&mut dispatcher);

    let event_handler = BotEventHandler::new(
        dispatcher,
        config.bot_name,
        db_manager,
        config.default_command_configuration,
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
}
