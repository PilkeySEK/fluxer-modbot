use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use mini_moka::sync::Cache;
use std::sync::Arc;

#[derive(Clone)]
pub struct PrefixCache(Arc<mini_moka::sync::Cache<Id<GuildMarker>, std::sync::Arc<Vec<String>>>>);

impl PrefixCache {
    pub fn new(cache_capacity: u64) -> Self {
        let cache = Arc::new(Cache::new(cache_capacity));

        Self(cache)
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

    /// The next time the prefixes for this guild need to be fetched from the database again.
    pub fn invalidate_guild_prefixes(&self, guild_id: Id<GuildMarker>) {
        self.0.invalidate(&guild_id);
    }
}
