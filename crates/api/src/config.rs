use std::{cmp::min, fs::File, io::Read, path::Path};

pub enum ConfigLoadError {
    IoError(std::io::Error),
    ParseError(json5::Error),
}

#[derive(serde::Deserialize)]
pub struct ApiOauth2Config {
    pub client_id: String,
    pub scopes: Vec<String>,
    pub redirect_uri: String,
    pub auth_uri: String,
    pub client_secret: String,
    pub token_uri: String,
}

#[derive(serde::Deserialize)]
pub struct ApiConfig {
    pub database_url: String,
    pub oauth2: ApiOauth2Config,
    pub fluxer_api_base: String,
    pub cookie_secret: String,
    pub dashboard_uri: String,
}

impl ApiConfig {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigLoadError> {
        let mut file = File::open(path).map_err(ConfigLoadError::IoError)?;
        let metadata = file.metadata().map_err(ConfigLoadError::IoError)?;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "If the config file is not multiple GB this is totally fine, and even if the value is truncated the String will just be reallocated if it is too small."
        )]
        let mut file_string = String::with_capacity(min(metadata.len() as usize, 4096));
        file.read_to_string(&mut file_string)
            .map_err(ConfigLoadError::IoError)?;
        json5::from_str(&file_string).map_err(ConfigLoadError::ParseError)
    }
}
