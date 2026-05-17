use std::env;

use fluxer_neptunium::client::Client;
use pretty_duration::{PrettyDurationOptions, PrettyDurationOutputFormat};

use crate::{
    commands::{CommandDispatcher, register_commands},
    config::{Config, ConfigLoadError},
    db::DatabaseManager,
    event_handler::BotEventHandler,
};

mod commands;
mod config;
mod db;
mod event_handler;
mod macros;

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
    tracing_subscriber::fmt().init();

    let config_file_path = env::var("CONFIG_FILE_PATH").unwrap_or(String::from("config.json5"));

    let config = match Config::load(config_file_path) {
        Ok(config) => config,
        Err(e) => {
            match e {
                ConfigLoadError::Io(e) => tracing::error!("I/O error loading config: {e}"),
                ConfigLoadError::Parse(e) => tracing::error!("Failed to parse config: {e}"),
            }
            return;
        }
    };

    let db_manager = match DatabaseManager::connect(&config.database_url).await {
        Ok(db_manager) => db_manager,
        Err(e) => {
            tracing::error!("Error connecting to database: {e}");
            return;
        }
    };

    let mut dispatcher = CommandDispatcher::new();
    register_commands(&mut dispatcher);
    let event_handler = BotEventHandler::new(
        dispatcher,
        config.bot_name,
        db_manager,
        config.default_command_prefix,
        config.default_command_configuration,
        config.max_command_prefix_len,
    );
    let mut client = Client::new(config.token);
    client.register_event_handler(event_handler);

    if let Err(e) = client.start().await {
        tracing::error!("Fatal client error: {e}");
    }
}
