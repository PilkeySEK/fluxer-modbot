use std::sync::Arc;

use api_types::pg_notifications::{GuildPrefixesUpdate, NOTIFICATION_GUILD_PREFIXES_UPDATE};
use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use mini_moka::sync::Cache;
use sqlx::postgres::PgListener;

#[derive(Clone)]
pub struct PrefixCache(Arc<mini_moka::sync::Cache<Id<GuildMarker>, std::sync::Arc<Vec<String>>>>);

impl PrefixCache {
    pub async fn new(database_url: &str, cache_capacity: u64) -> Result<Self, sqlx::Error> {
        let cache = Arc::new(Cache::new(cache_capacity));

        let mut listener = PgListener::connect(database_url).await?;
        listener.listen(NOTIFICATION_GUILD_PREFIXES_UPDATE).await?;

        tokio::spawn(Self(Arc::clone(&cache)).listener(listener));

        Ok(Self(cache))
    }

    async fn listener(self, mut listener: PgListener) {
        loop {
            match listener.recv().await {
                Ok(notification) => {
                    let payload: GuildPrefixesUpdate =
                        match serde_json::from_str(notification.payload()) {
                            Ok(value) => value,
                            Err(e) => {
                                tracing::error!(
                                    "Failed to deserialize payload for {}: {}",
                                    notification.channel(),
                                    e
                                );
                                continue;
                            }
                        };

                    self.0.invalidate(&payload.0);
                }
                Err(e) => {
                    tracing::error!("Error in PostgreSQL listener: {e}");
                }
            }
        }
    }

    pub fn get_guild_prefixes(&self, guild_id: Id<GuildMarker>) -> Option<Arc<Vec<String>>> {
        self.0.get(&guild_id)
    }

    pub fn update_guild_prefixes(
        &self,
        guild_id: Id<GuildMarker>,
        prefixes: Vec<String>,
    ) -> Arc<Vec<String>> {
        let prefixes = Arc::new(prefixes);
        self.0.insert(guild_id, Arc::clone(&prefixes));
        prefixes
    }
}
