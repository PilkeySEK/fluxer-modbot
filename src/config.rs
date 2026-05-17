use std::{collections::HashMap, fs::File, io::Read, path::Path};

use fluxer_neptunium::model::guild::permissions::Permissions;
use serde::Deserialize;
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

#[derive(Deserialize)]
pub struct Config {
    pub database_url: Zeroizing<String>,
    pub default_command_prefix: String,
    pub token: Zeroizing<String>,
    pub bot_name: String,
    pub default_command_configuration: HashMap<String, Permissions>,
    pub max_command_prefix_len: usize,
}

pub enum ConfigLoadError {
    Io(std::io::Error),
    Parse(json5::Error),
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigLoadError> {
        let mut file_string = String::new();
        File::open(path)
            .map_err(ConfigLoadError::Io)?
            .read_to_string(&mut file_string)
            .map_err(ConfigLoadError::Io)?;
        json5::from_str(&file_string).map_err(ConfigLoadError::Parse)
    }
}
