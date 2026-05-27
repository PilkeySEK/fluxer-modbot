use std::{ops::Deref, sync::Arc};

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, EmptyExtraTokenFields, EndpointNotSet, EndpointSet,
    RedirectUrl, RevocationErrorResponseType, Scope, StandardErrorResponse, StandardRevocableToken,
    StandardTokenIntrospectionResponse, StandardTokenResponse, TokenUrl,
    basic::{BasicClient, BasicErrorResponseType, BasicTokenType},
    reqwest::redirect::Policy,
};
use rand::rngs::ChaCha20Rng;
use tokio::sync::Mutex;

use crate::{config::ApiOauth2Config, db::DbManager, error::ApiError};

#[derive(Clone)]
pub struct AppState(pub Arc<InnerAppState>);

impl Deref for AppState {
    type Target = InnerAppState;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRef<AppState> for Key {
    fn from_ref(input: &AppState) -> Self {
        input.0.cookie_key.clone()
    }
}

type OAuthClient = oauth2::Client<
    StandardErrorResponse<BasicErrorResponseType>,
    StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardTokenIntrospectionResponse<EmptyExtraTokenFields, BasicTokenType>,
    StandardRevocableToken,
    StandardErrorResponse<RevocationErrorResponseType>,
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

pub struct InnerAppState {
    pub db: DbManager,
    pub oauth: OAuthClient,
    pub oauth_scopes: Vec<oauth2::Scope>,
    pub oauth_http_client: oauth2::reqwest::Client,
    pub fluxer_api_base: String,
    pub cookie_key: Key,
    pub rng: Mutex<ChaCha20Rng>,
    pub dashboard_uri: String,
    pub http_client: reqwest::Client,
}

impl InnerAppState {
    pub async fn new(
        oauth_config: ApiOauth2Config,
        database_url: &str,
        fluxer_api_base: String,
        cookie_key: Key,
        dashboard_uri: String,
    ) -> Result<Self, ApiError> {
        let oauth = BasicClient::new(ClientId::new(oauth_config.client_id))
            .set_client_secret(ClientSecret::new(oauth_config.client_secret))
            .set_auth_uri(AuthUrl::new(oauth_config.auth_uri)?)
            .set_token_uri(TokenUrl::new(oauth_config.token_uri)?)
            .set_redirect_uri(RedirectUrl::new(oauth_config.redirect_uri)?);

        let db = DbManager::new(database_url).await?;

        Ok(Self {
            oauth,
            db,
            oauth_scopes: oauth_config.scopes.into_iter().map(Scope::new).collect(),
            oauth_http_client: oauth2::reqwest::Client::builder()
                .redirect(Policy::none())
                .build()?,
            fluxer_api_base: if let Some(suffix_stripped) = fluxer_api_base.strip_suffix("/") {
                suffix_stripped.to_string()
            } else {
                fluxer_api_base
            },
            cookie_key,
            rng: Mutex::new(rand::make_rng()),
            dashboard_uri,
            http_client: reqwest::Client::new(),
        })
    }
}
