use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use mini_moka::sync::Cache;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub struct PrefixCache {
    cache: mini_moka::sync::Cache<Id<GuildMarker>, std::sync::Arc<Vec<String>>>,
    enabled: AtomicBool,
}

impl PrefixCache {
    pub fn new(cache_capacity: u64) -> Self {
        Self {
            cache: Cache::new(cache_capacity),
            enabled: AtomicBool::new(false),
        }
    }

    pub fn get_guild_prefixes(&self, guild_id: Id<GuildMarker>) -> Option<Arc<Vec<String>>> {
        if !self.enabled.load(Ordering::Acquire) {
            return None;
        }
        self.cache.get(&guild_id)
    }

    pub fn set_enabled(&self, enabled: bool) {
        if enabled {
            tracing::debug!("Enabling prefix cache");
        } else {
            tracing::debug!("Disabling prefix cache");
        }
        self.cache.invalidate_all();
        self.enabled.store(enabled, Ordering::Release);
    }

    pub fn update_guild_prefixes(
        &self,
        guild_id: Id<GuildMarker>,
        prefixes: Vec<String>,
    ) -> Arc<Vec<String>> {
        let prefixes = Arc::new(prefixes);
        if !self.enabled.load(Ordering::Acquire) {
            return prefixes;
        }
        self.cache.insert(guild_id, Arc::clone(&prefixes));
        prefixes
    }

    /// The next time the prefixes for this guild need to be fetched from the database again.
    pub fn invalidate_guild_prefixes(&self, guild_id: Id<GuildMarker>) {
        if !self.enabled.load(Ordering::Acquire) {
            return;
        }
        self.cache.invalidate(&guild_id);
    }
}
