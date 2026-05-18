use fluxer_neptunium::model::id::{Id, marker::GuildMarker};
use sqlx::{
    PgPool,
    postgres::{PgPoolOptions, PgQueryResult},
    query, query_as, query_scalar,
};

use crate::db::schema::{GuildCommandConfiguration, RawGuildCommandConfiguration};

pub struct DatabaseManager {
    pool: PgPool,
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
    pub async fn connect(url: &str) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PgPoolOptions::new().connect(url).await?,
        })
    }

    pub async fn get_guild_command_prefixes(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Option<Vec<String>>, sqlx::Error> {
        query_scalar!(
            "SELECT command_prefixes FROM guilds
            WHERE guild_id = $1",
            guild_id.into_inner().cast_signed(),
        )
        .fetch_optional(&self.pool)
        .await
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
        default_prefix: &str,
    ) -> Result<PgQueryResult, sqlx::Error> {
        query!(
            "INSERT INTO guilds (guild_id, command_prefixes)
            VALUES ($1, ARRAY[$2, $3])
            ON CONFLICT (guild_id) DO UPDATE
            SET command_prefixes = CASE
                WHEN $2=ANY(guilds.command_prefixes) THEN ARRAY[$2, '!']
                ELSE ARRAY_APPEND(guilds.command_prefixes, $2)
            END",
            guild_id.into_inner().cast_signed(),
            prefix,
            default_prefix,
        )
        .execute(&self.pool)
        .await
    }

    pub async fn remove_guild_command_prefix_upsert(
        &self,
        guild_id: Id<GuildMarker>,
        prefix: &str,
        default_prefix: &str,
    ) -> Result<Vec<String>, sqlx::Error> {
        let inserted_prefixes_vec = if prefix == default_prefix {
            Vec::new()
        } else {
            vec![default_prefix.to_owned()]
        };
        query_scalar!(
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
        .await
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
