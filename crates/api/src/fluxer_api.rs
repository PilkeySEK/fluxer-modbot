use fluxer_neptunium::{
    http::endpoints::{ExecuteEndpointRequestError, ResponseBody},
    model::{gateway::payload::incoming::UserPrivateResponse, guild::Guild},
};
use reqwest::Client;

pub struct FluxerApiManager {
    client: reqwest::Client,
    api_base: String,
}

impl FluxerApiManager {
    pub fn new(api_base: String) -> Self {
        Self {
            client: Client::new(),
            api_base,
        }
    }

    pub async fn get_user_guilds(
        &self,
        bearer_token: &str,
    ) -> Result<Vec<Guild>, Box<ExecuteEndpointRequestError>> {
        <std::vec::Vec<Guild> as fluxer_neptunium::http::endpoints::ResponseBody>::deserialize(
            self.client
                .get(format!("{}/users/@me/guilds", self.api_base))
                .bearer_auth(bearer_token)
                .send()
                .await?
                .bytes()
                .await?
                .to_vec(),
        )
    }

    pub async fn get_user(
        &self,
        bearer_token: &str,
    ) -> Result<UserPrivateResponse, Box<ExecuteEndpointRequestError>> {
        ResponseBody::deserialize(
            self.client
                .get(format!("{}/users/@me", self.api_base))
                .bearer_auth(bearer_token)
                .send()
                .await?
                .bytes()
                .await?
                .to_vec(),
        )
    }
}
