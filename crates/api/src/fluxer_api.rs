use std::sync::atomic::{AtomicBool, Ordering};

use fluxer_neptunium::{
    http::endpoints::{ExecuteEndpointRequestError, ResponseBody},
    model::{
        gateway::payload::incoming::UserPrivateResponse,
        guild::{Guild, permissions::Permissions},
        id::{
            Id,
            marker::{GuildMarker, UserMarker},
        },
    },
};
use moka::future::CacheBuilder;
use reqwest::Client;

pub struct FluxerApiManager {
    client: reqwest::Client,
    api_base: String,
    guild_member_permissions: moka::future::Cache<(Id<GuildMarker>, Id<UserMarker>), Permissions>,
    /// The cache will be disabled when there is no websocket connection to the worker.
    cache_enabled: AtomicBool,
}

impl FluxerApiManager {
    pub fn new(api_base: String) -> Self {
        Self {
            client: Client::new(),
            api_base,
            guild_member_permissions: CacheBuilder::new(10u64.pow(6) /* 10^6 = one million */)
                .support_invalidation_closures()
                .build(),
            cache_enabled: AtomicBool::new(false),
        }
    }

    pub async fn get_user_guilds(
        &self,
        user_id: Id<UserMarker>,
        bearer_token: &str,
    ) -> Result<Vec<Guild>, Box<ExecuteEndpointRequestError>> {
        let guilds =
            <std::vec::Vec<Guild> as fluxer_neptunium::http::endpoints::ResponseBody>::deserialize(
                self.client
                    .get(format!("{}/users/@me/guilds", self.api_base))
                    .bearer_auth(bearer_token)
                    .send()
                    .await?
                    .bytes()
                    .await?
                    .to_vec(),
            )?;

        if self.cache_enabled.load(Ordering::Acquire) {
            for guild in &guilds {
                let Some(current_user_permissions) = guild.current_user_permissions else {
                    tracing::warn!(
                        "The guild {} doesn't have permissions for user {} set.",
                        guild.id,
                        user_id
                    );
                    continue;
                };
                self.guild_member_permissions
                    .insert((guild.id, user_id), current_user_permissions)
                    .await;
            }
        }

        Ok(guilds)
    }

    pub fn set_cache_enabled(&self, enabled: bool) {
        if enabled {
            tracing::debug!("Enabling guild member permissions cache");
        } else {
            tracing::debug!("Disabling guild member permissions cache");
            self.guild_member_permissions.invalidate_all();
        }
        self.cache_enabled.store(enabled, Ordering::Release);
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

    pub async fn get_user_guild_permissions(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
        bearer_token: &str,
    ) -> Result<Option<Permissions>, Box<ExecuteEndpointRequestError>> {
        if let Some(permissions) = self
            .guild_member_permissions
            .get(&(guild_id, user_id))
            .await
        {
            return Ok(Some(permissions));
        }
        let guilds = self.get_user_guilds(user_id, bearer_token).await?;
        Ok(guilds
            .iter()
            .find(|guild| guild.id == guild_id)
            .and_then(|guild| {
                if guild.current_user_permissions.is_none() {
                    tracing::warn!(
                        "Current user permissions for guild {} is not set for user {}",
                        guild.id,
                        user_id
                    );
                }
                guild.current_user_permissions
            }))
    }

    pub fn invalidate_cached_guild_permissions(&self, guild_id: Id<GuildMarker>) {
        if let Err(e) = self
            .guild_member_permissions
            .invalidate_entries_if(move |k, _v| k.0 == guild_id)
        {
            tracing::error!("invalidate_entries_if returned error: {e}");
        }
    }

    pub async fn invalidate_cached_guild_permissions_for_user(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
    ) {
        self.guild_member_permissions
            .invalidate(&(guild_id, user_id))
            .await;
    }
}
