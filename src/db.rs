use std::sync::Arc;

use fluxer_neptunium::model::id::{
    Id,
    marker::{GuildMarker, UserMarker},
};
use mini_moka::sync::Cache;
use sqlx::{
    PgPool, QueryBuilder,
    postgres::{PgPoolOptions, PgQueryResult},
    query, query_as, query_scalar,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    case_expiration::ExpiringCase,
    db::schema::{
        CaseId, CreateGuildModerationCaseData, GuildCommandConfiguration, GuildModerationCase,
        ModerationKind, RawGuildCommandConfiguration, RawGuildModerationCase,
    },
    macros::debug_panic,
};

pub struct DatabaseManager {
    pool: PgPool,
    cached_prefixes: mini_moka::sync::Cache<Id<GuildMarker>, std::sync::Arc<Vec<String>>>,
    default_prefix: String,
    pub(super) expiring_cases_tx: Option<UnboundedSender<ExpiringCase>>,
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
            expiring_cases_tx: None,
        })
    }

    fn send_expiring_case(
        &self,
        expires_at: Option<chrono::DateTime<chrono::Utc>>,
        case_id: CaseId,
    ) {
        if let Some(tx) = &self.expiring_cases_tx {
            let _ = tx.send((expires_at, case_id));
        }
    }

    pub async fn get_guild_command_prefixes(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Arc<Vec<String>>, DatabaseError> {
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
        Ok(if let Some(prefixes) = prefixes {
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
    ) -> Result<(), DatabaseError> {
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
    ) -> Result<Arc<Vec<String>>, DatabaseError> {
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

    pub async fn create_moderation_case(
        &self,
        data: CreateGuildModerationCaseData<'_>,
    ) -> Result<CaseId, DatabaseError> {
        let case_id = query_scalar!(
            "INSERT INTO guild_moderation_cases (guild_id, target_id, moderator_id, moderation_kind, expires_at, reason, duration, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING case_id",
            data.guild_id.into_inner().cast_signed(),
            data.target_id.into_inner().cast_signed(),
            data.moderator_id.map(|id| id.into_inner().cast_signed()),
            data.moderation_kind.to_string(),
            data.expiry.map(|value| value.0),
            data.reason,
            data.expiry.map(|value| value.1),
            data.created_at,
        ).fetch_one(&self.pool).await?;
        let case_id = CaseId(case_id);

        if let Some(expiry) = data.expiry {
            self.send_expiring_case(Some(expiry.0), case_id);
        }

        Ok(case_id)
    }

    pub async fn list_guild_moderation_cases(
        &self,
        guild_id: Id<GuildMarker>,
        limit: i64,
        offset: Option<i64>,
        involving_user: Option<Id<UserMarker>>,
    ) -> Result<Vec<GuildModerationCase>, DatabaseError> {
        let mut qb = QueryBuilder::new("SELECT * FROM guild_moderation_cases WHERE guild_id = ");
        qb.push_bind(guild_id.into_inner().cast_signed());
        if let Some(involving_user) = involving_user {
            let involving_user = involving_user.into_inner().cast_signed();
            qb.push(" AND (target_id = ").push_bind(involving_user);
            qb.push(" OR moderator_id = ").push_bind(involving_user);
            qb.push(")");
        }

        qb.push(" ORDER BY case_id DESC LIMIT ").push_bind(limit);
        if let Some(offset) = offset {
            qb.push(" OFFSET ").push_bind(offset);
        }

        let raw_cases = qb.build_query_as().fetch_all(&self.pool).await?;

        match raw_cases
            .into_iter()
            .map(GuildModerationCase::from_raw)
            .collect::<Option<Vec<GuildModerationCase>>>()
        {
            Some(cases) => Ok(cases),
            None => Err(DatabaseError::ParseError),
        }
    }

    pub async fn count_guild_moderation_cases(
        &self,
        guild_id: Id<GuildMarker>,
        involving_user: Option<Id<UserMarker>>,
    ) -> Result<i64, DatabaseError> {
        Ok(if let Some(involving_user) = involving_user {
            query_scalar!(
                "SELECT COUNT(case_id) FROM guild_moderation_cases
                WHERE guild_id = $1 AND (target_id = $2 OR moderator_id = $2)",
                guild_id.into_inner().cast_signed(),
                involving_user.into_inner().cast_signed(),
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            query_scalar!(
                "SELECT COUNT(case_id) FROM guild_moderation_cases
                WHERE guild_id = $1",
                guild_id.into_inner().cast_signed(),
            )
            .fetch_one(&self.pool)
            .await?
        }
        .unwrap_or(0))
    }

    pub async fn get_guild_moderation_case(
        &self,
        guild_id: Id<GuildMarker>,
        case_id: CaseId,
    ) -> Result<Option<GuildModerationCase>, DatabaseError> {
        let case = query_as!(
            RawGuildModerationCase,
            "SELECT * FROM guild_moderation_cases
            WHERE guild_id = $1 AND case_id = $2",
            guild_id.into_inner().cast_signed(),
            case_id.0,
        )
        .fetch_optional(&self.pool)
        .await?;

        match case {
            Some(case) => match GuildModerationCase::from_raw(case) {
                Some(case) => Ok(Some(case)),
                None => Err(DatabaseError::ParseError),
            },
            None => Ok(None),
        }
    }

    pub async fn get_last_guild_moderation_case_involving_user(
        &self,
        guild_id: Id<GuildMarker>,
        involved_user_id: Id<UserMarker>,
    ) -> Result<Option<GuildModerationCase>, DatabaseError> {
        let case = query_as!(
            RawGuildModerationCase,
            "SELECT * FROM guild_moderation_cases
            WHERE guild_id = $1 AND (moderator_id = $2 OR target_id = $2)
            ORDER BY case_id DESC
            LIMIT 1",
            guild_id.into_inner().cast_signed(),
            involved_user_id.into_inner().cast_signed(),
        )
        .fetch_optional(&self.pool)
        .await?;

        match case {
            Some(case) => match GuildModerationCase::from_raw(case) {
                Some(case) => Ok(Some(case)),
                None => Err(DatabaseError::ParseError),
            },
            None => Ok(None),
        }
    }

    pub async fn close_latest_guild_moderation_case_of_kind(
        &self,
        guild_id: Id<GuildMarker>,
        user_id: Id<UserMarker>,
        moderation_kind: ModerationKind,
        reason: Option<&str>,
        closed_by: Option<Id<UserMarker>>,
    ) -> Result<Option<CaseId>, DatabaseError> {
        let case_id = query_scalar!(
            "UPDATE guild_moderation_cases
            SET closed = true, close_reason = $1, closed_by = $2
            WHERE case_id=(
              SELECT case_id FROM guild_moderation_cases
              WHERE guild_id = $3 AND target_id = $4 AND closed = false AND (expires_at IS NULL OR expires_at > NOW()) AND moderation_kind = $5
              ORDER BY case_id DESC
              LIMIT 1
            )
            RETURNING case_id",
            reason,
            closed_by.map(|id| id.into_inner().cast_signed()),
            guild_id.into_inner().cast_signed(),
            user_id.into_inner().cast_signed(),
            moderation_kind.to_string(),
        ).fetch_optional(&self.pool).await?;
        if let Some(case_id) = case_id {
            let case_id = CaseId(case_id);

            self.send_expiring_case(None, case_id);

            Ok(Some(case_id))
        } else {
            Ok(None)
        }
    }

    /// Does not notify the case expiration actor.
    pub async fn close_guild_moderation_case_by_id_silently(
        &self,
        case_id: CaseId,
        reason: Option<&str>,
        closed_by: Option<Id<UserMarker>>,
    ) -> Result<PgQueryResult, DatabaseError> {
        Ok(query!(
            "UPDATE guild_moderation_cases
            SET closed = true, close_reason = $1, closed_by = $2
            WHERE case_id = $3",
            reason,
            closed_by.map(|id| id.into_inner().cast_signed()),
            case_id.0,
        )
        .execute(&self.pool)
        .await?)
    }

    pub async fn get_all_expiring_case_ids(
        &self,
    ) -> Result<Vec<(chrono::DateTime<chrono::Utc>, CaseId)>, DatabaseError> {
        struct CaseExpiryInfo {
            expires_at: Option<chrono::DateTime<chrono::Utc>>,
            case_id: i64,
        }
        let cases = query_as!(
            CaseExpiryInfo,
            "SELECT expires_at, case_id FROM guild_moderation_cases
            WHERE expires_at IS NOT NULL"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(cases
            .into_iter()
            .filter_map(|info| {
                if let Some(expires_at) = info.expires_at {
                    Some((expires_at, CaseId(info.case_id)))
                } else {
                    debug_panic!("Database returned a case where expires_at is null");
                    None
                }
            })
            .collect())
    }
}

pub mod schema {
    use std::{str::FromStr, time::Duration};

    use fluxer_neptunium::model::{
        guild::permissions::Permissions,
        id::{
            Id,
            marker::{ChannelMarker, GuildMarker, RoleMarker, UserMarker},
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

    #[derive(strum::Display, strum::EnumString)]
    pub enum ModerationKind {
        Warn,
        Mute,
        Kick,
        Ban,
    }

    pub struct CreateGuildModerationCaseData<'a> {
        pub guild_id: Id<GuildMarker>,
        pub target_id: Id<UserMarker>,
        pub moderator_id: Option<Id<UserMarker>>,
        pub moderation_kind: ModerationKind,
        pub reason: Option<&'a str>,
        pub expiry: Option<(chrono::DateTime<chrono::Utc>, i64)>,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct CaseId(pub i64);

    #[derive(sqlx::FromRow)]
    pub(super) struct RawGuildModerationCase {
        pub case_id: i64,
        pub guild_id: i64,
        pub target_id: i64,
        pub moderator_id: Option<i64>,
        pub moderation_kind: String,
        pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
        pub reason: Option<String>,
        pub closed: bool,
        pub duration: Option<i64>,
        pub created_at: chrono::DateTime<chrono::Utc>,
        pub close_reason: Option<String>,
        pub closed_by: Option<i64>,
    }

    pub struct GuildModerationCaseCloseData {
        pub reason: Option<String>,
        pub closed_by: Option<Id<UserMarker>>,
    }

    pub struct GuildModerationCase {
        pub case_id: CaseId,
        #[expect(unused)]
        pub guild_id: Id<GuildMarker>,
        pub target_id: Id<UserMarker>,
        pub moderator_id: Option<Id<UserMarker>>,
        pub moderation_kind: ModerationKind,
        pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
        pub reason: Option<String>,
        pub duration: Option<Duration>,
        pub created_at: chrono::DateTime<chrono::Utc>,
        /// `None` if the case is not closed.
        pub close_data: Option<GuildModerationCaseCloseData>,
    }

    impl GuildModerationCase {
        pub(super) fn from_raw(raw: RawGuildModerationCase) -> Option<Self> {
            Some(Self {
                case_id: CaseId(raw.case_id),
                guild_id: raw.guild_id.cast_unsigned().into(),
                target_id: raw.target_id.cast_unsigned().into(),
                moderator_id: raw.moderator_id.map(|id| id.cast_unsigned().into()),
                moderation_kind: ModerationKind::from_str(&raw.moderation_kind).ok()?,
                expires_at: raw.expires_at,
                reason: raw.reason,
                duration: match raw.duration {
                    Some(duration_i64) => {
                        Some(Duration::from_secs(u64::try_from(duration_i64).ok()?))
                    }
                    None => None,
                },
                created_at: raw.created_at,
                close_data: if raw.closed {
                    Some(GuildModerationCaseCloseData {
                        reason: raw.close_reason,
                        closed_by: raw.closed_by.map(|id| id.cast_unsigned().into()),
                    })
                } else {
                    None
                },
            })
        }
    }

    impl std::fmt::Display for CaseId {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // Try to encode it using SQIDS with the blocklist, which is very unlikely to fail.
            // It only fails when it has reached the maximum number of tries for getting around the
            // blocklist, in which case we use sqids without a blocklist, which may contain a bad
            // word but this is probably fine in practice. Either way, it would be better than
            // panicking if sqids fails.
            let sqids_encoded = match crate::SQIDS.encode(&[self.0.cast_unsigned()]) {
                Ok(encoded) => encoded,
                #[expect(
                    clippy::unwrap_used,
                    reason = "There is no blocklist so this can't fail."
                )]
                Err(_) => crate::SQIDS_NO_BLOCKLIST
                    .encode(&[self.0.cast_unsigned()])
                    .unwrap(),
            };
            f.write_str(&sqids_encoded)
        }
    }

    impl CaseId {
        pub fn from_str(s: &str) -> Option<Self> {
            // I think sqids doesn't take any blocklists into account when decoding so this is fine
            // even if the original ID was generated using SQIDS_NO_BLOCKLISt
            let id = crate::SQIDS.decode(s);
            id.first().map(|id| Self(id.cast_signed()))
        }
    }
}
