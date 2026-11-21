use std::{collections::HashMap, path::Path};

use color_eyre::eyre::Context;
use enum_map::EnumMap;
use fluxer_neptunium::model::guild::permissions::Permissions;
use secrecy::SecretString;

use crate::db::CommandId;

#[derive(serde::Deserialize)]
pub struct InternalConfig {
    pub database_url: SecretString,
    pub api_worker_ws_url: String,
    pub bot_fluxer_token: SecretString,
    pub cookie_secret: SecretString,
    pub dashboard_base: String,
    pub fluxer_instance_root: String,
    pub worker_api_token: SecretString,
    pub oauth2: OAuth2Config,
}

#[derive(serde::Deserialize)]
pub struct OAuth2Config {
    pub client_id: String,
    pub client_secret: SecretString,
    pub auth_uri: String,
    pub redirect_uri: String,
    pub token_uri: String,
    pub scopes: Vec<String>,
}

#[derive(serde::Deserialize)]
pub struct PublicConfig {
    pub bot_name: String,
    pub default_command_prefix: String,
    pub max_command_prefix_len: usize,
    pub max_command_prefixes: usize,
    pub webhook_avatar: Option<String>,
}

#[derive(serde::Deserialize)]
pub struct DefaultCommandsConfig {
    pub names: HashMap<String, CommandId>,
    pub required_permissions: EnumMap<CommandId, Permissions>,
}

#[derive(serde::Deserialize)]
pub struct Config {
    pub internal: InternalConfig,
    pub public: PublicConfig,
    pub default_commands: DefaultCommandsConfig,
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> color_eyre::Result<Self> {
        let file_string = tokio::fs::read_to_string(path)
            .await
            .wrap_err("Failed to open file")?;
        toml::from_str(&file_string).wrap_err("Failed to deserialize file contents")
    }
}
