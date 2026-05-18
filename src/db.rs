use std::sync::Arc;

use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use mini_moka::sync::Cache;
use sqlx::{PgPool, postgres::PgPoolOptions, query_as, query_scalar};

use crate::db::schema::{GuildCommandConfiguration, RawGuildCommandConfiguration};

pub struct DatabaseManager {
    pool: PgPool,
    cached_prefixes: mini_moka::sync::Cache<Id<GuildMarker>, std::sync::Arc<Vec<String>>>,
    default_prefix: String,
}

#[derive(Debug)]
pub enum DatabaseError {
    SqlxError(sqlx::Error),
    ParseError,
}

impl std::error::Error for DatabaseError {}

impl std::fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SqlxError(e) => f.write_fmt(format_args!("SQLX error: {e}")),
            Self::ParseError => f.write_str("Parse error"),
        }
    }
}

impl From<sqlx::Error> for DatabaseError {
    fn from(value: sqlx::Error) -> Self {
        Self::SqlxError(value)
    }
}

impl DatabaseManager {
    pub async fn connect(
        url: &str,
        prefix_cache_capacity: u64,
        default_prefix: String,
    ) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new().connect(url).await?,
            cached_prefixes: Cache::new(prefix_cache_capacity),
            default_prefix,
        })
    }

    pub async fn get_guild_command_prefixes(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Arc<Vec<String>>, sqlx::Error> {
        if let Some(cached_prefixes) = self.cached_prefixes.get(&guild_id) {
            return Ok(cached_prefixes);
        }
        let prefixes: Option<Vec<String>> = query_scalar!(
            "SELECT command_prefixes FROM guilds
            WHERE guild_id = $1",
            guild_id.into_inner().cast_signed(),
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(if let Some(prefixes) = prefixes.clone() {
            let prefixes = Arc::new(prefixes);
            self.cached_prefixes.insert(guild_id, Arc::clone(&prefixes));
            prefixes
        } else {
            let prefixes = Arc::new(vec![self.default_prefix.clone()]);
            self.cached_prefixes.insert(guild_id, Arc::clone(&prefixes));
            prefixes
        })
    }

    pub async fn get_guild_command_configuration(
        &self,
        guild_id: Id<GuildMarker>,
        command_name: &str,
    ) -> Result<Option<GuildCommandConfiguration>, DatabaseError> {
        let raw_configuration = query_as!(
            RawGuildCommandConfiguration,
            "SELECT * FROM guild_command_configuration
            WHERE guild_id = $1 AND command_name = $2",
            guild_id.into_inner().cast_signed(),
            command_name
        )
        .fetch_optional(&self.pool)
        .await?;

        match raw_configuration {
            Some(raw_configuration) => match GuildCommandConfiguration::from_raw(raw_configuration)
            {
                Some(configuration) => Ok(Some(configuration)),
                None => Err(DatabaseError::ParseError),
            },
            None => Ok(None),
        }
    }

    pub async fn add_guild_command_prefix_upsert(
        &self,
        guild_id: Id<GuildMarker>,
        prefix: &str,
    ) -> Result<(), sqlx::Error> {
        let prefixes = query_scalar!(
            "INSERT INTO guilds (guild_id, command_prefixes)
            VALUES ($1, ARRAY[$2, $3])
            ON CONFLICT (guild_id) DO UPDATE
            SET command_prefixes = CASE
                WHEN $2=ANY(guilds.command_prefixes) THEN ARRAY[$2, '!']
                ELSE ARRAY_APPEND(guilds.command_prefixes, $2)
            END
            RETURNING command_prefixes",
            guild_id.into_inner().cast_signed(),
            prefix,
            self.default_prefix,
        )
        .fetch_one(&self.pool)
        .await?;
        self.cached_prefixes.insert(guild_id, Arc::new(prefixes));
        Ok(())
    }

    pub async fn remove_guild_command_prefix_upsert(
        &self,
        guild_id: Id<GuildMarker>,
        prefix: &str,
    ) -> Result<Arc<Vec<String>>, sqlx::Error> {
        let inserted_prefixes_vec = if prefix == self.default_prefix {
            Vec::new()
        } else {
            vec![self.default_prefix.clone()]
        };
        let prefixes = query_scalar!(
            "INSERT INTO guilds (guild_id, command_prefixes)
            VALUES ($1, $2)
            ON CONFLICT (guild_id) DO UPDATE
            SET command_prefixes = ARRAY_REMOVE(guilds.command_prefixes, $3)
            RETURNING command_prefixes",
            guild_id.into_inner().cast_signed(),
            inserted_prefixes_vec.as_slice(),
            prefix,
        )
        .fetch_one(&self.pool)
        .await?;
        let prefixes = Arc::new(prefixes);
        self.cached_prefixes.insert(guild_id, Arc::clone(&prefixes));
        Ok(prefixes)
    }
}

pub mod schema {
    use fluxer_neptunium::model::{
        guild::permissions::Permissions,
        id::{
            Id,
            marker::{ChannelMarker, GuildMarker, RoleMarker},
        },
    };

    pub struct GuildCommandConfiguration {
        #[expect(unused)]
        pub guild_id: Id<GuildMarker>,
        #[expect(unused)]
        pub command_name: String,
        pub roles: Vec<Id<RoleMarker>>,
        pub permissions: Permissions,
        pub channels: Vec<Id<ChannelMarker>>,
    }

    #[derive(sqlx::FromRow)]
    pub(super) struct RawGuildCommandConfiguration {
        pub guild_id: i64,
        pub command_name: String,
        pub roles: Vec<String>,
        pub permissions: String,
        pub channels: Vec<String>,
    }

    impl GuildCommandConfiguration {
        pub(super) fn from_raw(raw: RawGuildCommandConfiguration) -> Option<Self> {
            Some(Self {
                guild_id: raw.guild_id.cast_unsigned().into(),
                command_name: raw.command_name,
                roles: raw
                    .roles
                    .into_iter()
                    .map(|id_str| id_str.parse::<u64>().ok().map(Id::new))
                    .collect::<Option<_>>()?,
                permissions: Permissions::from_bits_truncate(raw.permissions.parse::<u64>().ok()?),
                channels: raw
                    .channels
                    .into_iter()
                    .map(|id_str| id_str.parse::<u64>().ok().map(Id::new))
                    .collect::<Option<_>>()?,
            })
        }
    }
}
