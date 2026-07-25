use std::{collections::HashMap, path::Path};

use anyhow::Context;
use api_types::db::CommandId;
use enum_map::EnumMap;
use fluxer_neptunium::model::{
    guild::permissions::Permissions,
    id::{Id, marker::UserMarker},
};
use serde::{Deserialize, de::DeserializeOwned};
use zeroize::Zeroizing;

/*
pub struct Config {
    pub token: Zeroizing<String>,
    pub bot_name: String,
    pub database_url: Zeroizing<String>,
    pub default_command_prefix: String,
}

impl Config {
    pub fn load() -> Result<Self, dotenvy::Error> {
        dotenvy::dotenv()?;
        Ok(Self {
            token: Zeroizing::new(dotenvy::var("TOKEN")?),
            bot_name: dotenvy::var("BOT_NAME")?,
            database_url: Zeroizing::new(dotenvy::var("DATABASE_URL")?),
            default_command_prefix: dotenvy::var("DEFAULT_COMMAND_PREFIX")?,
        })
    }
}
*/

pub trait ConfigExt: DeserializeOwned {
    async fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self>;
}

#[derive(Deserialize)]
pub struct Config {
    pub database_url: Zeroizing<String>,
    pub default_command_prefix: String,
    pub token: Zeroizing<String>,
    pub bot_name: String,
    pub max_command_prefix_len: usize,
    pub prefix_cache_capacity: u64,
    pub webhook_avatar_b64: Option<String>,
    pub bot_id: Id<UserMarker>,
    pub worker_api_token: String,
    pub api_worker_ws_url: String,
    pub max_command_prefixes: usize,
}

#[derive(Deserialize)]
pub struct DefaultCommandConfig {
    pub names: HashMap<String, CommandId>,
    pub permissions: EnumMap<CommandId, Permissions>,
}

impl<T> ConfigExt for T
where
    T: DeserializeOwned,
{
    async fn load_from_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let file_string = tokio::fs::read_to_string(path)
            .await
            .context("Failed to open file")?;
        json5::from_str(&file_string).context("Failed to deserialize")
    }
}
