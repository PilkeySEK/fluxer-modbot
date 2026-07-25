use std::sync::Arc;

use anyhow::{Context, bail};
use fluxer_neptunium::model::id::{
    Id,
    marker::{ChannelMarker, GuildMarker, UserMarker, WebhookMarker},
};
use rust_shared::db::{
    CaseId, DbError, GuildModerationCase, ModerationKind, RawGuildModerationCase,
    SharedDatabaseManager,
};
use sqlx::{
    PgPool,
    postgres::{PgPoolOptions, PgQueryResult},
    query, query_as, query_scalar,
};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender},
    task::JoinHandle,
};

use crate::{
    caches::PrefixCache,
    case_expiration::{ExpiringCase, start_case_expiration_actor},
    db::schema::CreateGuildModerationCaseData,
    logging::{Logger, ModLogEntry},
    macros::debug_panic,
};

pub struct DatabaseManager {
    pool: PgPool,
    pub cached_prefixes: PrefixCache,
    default_prefix: String,
    expiring_cases_tx: UnboundedSender<ExpiringCase>,
    logger: Arc<Logger>,
    shared_manager: SharedDatabaseManager,
}

impl DatabaseManager {
    pub async fn get_guild_command_prefixes(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Arc<Vec<String>>, DbError> {
        if let Some(cached_prefixes) = self.cached_prefixes.get_guild_prefixes(guild_id) {
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
            self.cached_prefixes
                .update_guild_prefixes(guild_id, prefixes)
        } else {
            self.cached_prefixes
                .update_guild_prefixes(guild_id, vec![self.default_prefix.clone()])
        })
    }

    pub async fn add_guild_command_prefix_upsert(
        &self,
        guild_id: Id<GuildMarker>,
        prefix: &str,
    ) -> Result<Arc<Vec<String>>, DbError> {
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
        Ok(self
            .cached_prefixes
            .update_guild_prefixes(guild_id, prefixes))
    }

    pub async fn remove_guild_command_prefix_upsert(
        &self,
        guild_id: Id<GuildMarker>,
        prefix: &str,
    ) -> Result<Arc<Vec<String>>, DbError> {
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
        Ok(self
            .cached_prefixes
            .update_guild_prefixes(guild_id, prefixes))
    }

    /// The last argument is only used to ensure that the caller will add the user info to the database.
    pub async fn create_moderation_case(
        &self,
        data: CreateGuildModerationCaseData<'_>,
        _user_info_fetcher_task: JoinHandle<()>,
    ) -> anyhow::Result<CaseId> {
        let raw = query_as!(
            RawGuildModerationCase,
            "INSERT INTO guild_moderation_cases (guild_id, target_id, moderator_id, moderation_kind, expires_at, reason, duration, created_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING *",
            data.guild_id.into_inner().cast_signed(),
            data.target_id.into_inner().cast_signed(),
            data.moderator_id.map(|id| id.into_inner().cast_signed()),
            data.moderation_kind.to_string(),
            data.expiry.map(|value| value.0),
            data.reason,
            data.expiry.map(|value| value.1),
            data.created_at,
        ).fetch_one(&self.pool).await.context("Failed to execute database query")?;
        let Some(case) = GuildModerationCase::from_raw(raw) else {
            bail!("Failed to parse database response");
        };

        let case_id = case.case_id;

        if data.moderation_kind.manual_expiration()
            && let Some(expiry) = data.expiry
            && let Err(e) = self.expiring_cases_tx.send((Some(expiry.0), case_id))
        {
            tracing::error!("{e}");
        }

        self.logger
            .create_modlog_entry(self, case.guild_id, ModLogEntry::CaseCreated(case))
            .await;

        Ok(case_id)
    }

    /*
        pub async fn close_existing_cases_by_kind_and_user(
            &self,
            guild_id: Id<GuildMarker>,
            target_id: Id<UserMarker>,
            kind: ModerationKind,
            reason: Option<&str>,
            exclude_id: CaseId,
        ) -> Result<Vec<CaseId>, DbError> {
            let case_ids = query_scalar!(
                "UPDATE guild_moderation_cases
            SET closed = true, close_reason = $1, closed_by = NULL
            WHERE guild_id = $2 AND target_id = $3 AND closed = false AND moderation_kind = $4 AND case_id != $5
            RETURNING case_id",
                reason,
                guild_id.into_inner().cast_signed(),
                target_id.into_inner().cast_signed(),
                kind.to_string(),
                exclude_id.0,
            )
            .fetch_all(&self.pool)
            .await?;
            let case_ids = case_ids.into_iter().map(CaseId).collect::<Vec<_>>();
            if kind.manual_expiration() {
                for &id in &case_ids {
                    if let Err(e) = self.expiring_cases_tx.send((None, id)) {
                        tracing::error!("{e}");
                    }
                }
            }

            Ok(case_ids)
        }
    */

    /*
        pub async fn get_latest_open_moderation_case_by_kind_and_user(
            &self,
            guild_id: Id<GuildMarker>,
            target_id: Id<UserMarker>,
            kind: ModerationKind,
        ) -> Result<Option<GuildModerationCase>, DbError> {
            let raw = query_as!(
                RawGuildModerationCase,
                "SELECT * FROM guild_moderation_cases
            WHERE guild_id = $1 AND target_id = $2 AND moderation_kind = $3 AND closed = false
            ORDER BY case_id DESC
            LIMIT 1",
                guild_id.into_inner().cast_signed(),
                target_id.into_inner().cast_signed(),
                kind.to_string(),
            )
            .fetch_optional(&self.pool)
            .await?;

            Ok(match raw {
                Some(raw) => match GuildModerationCase::from_raw(raw) {
                    Some(case) => Some(case),
                    None => return Err(DbError::ParseError),
                },
                None => None,
            })
        }
    */

    pub async fn close_and_get_latest_open_moderation_case_by_kind_and_user(
        &self,
        guild_id: Id<GuildMarker>,
        target_id: Id<UserMarker>,
        kind: ModerationKind,
        reason: Option<&str>,
        closed_by: Option<Id<UserMarker>>,
    ) -> Result<Option<CaseId>, DbError> {
        let case_id = query_scalar!(
            "UPDATE guild_moderation_cases
            SET closed = true, closed_by = $1, close_reason = $2
            WHERE case_id=(
                SELECT case_id FROM guild_moderation_cases
                WHERE guild_id = $3 AND target_id = $4 AND moderation_kind = $5 AND closed = false
                ORDER BY case_id DESC
                LIMIT 1
            )
            RETURNING case_id",
            closed_by.map(|id| id.into_inner().cast_signed()),
            reason,
            guild_id.into_inner().cast_signed(),
            target_id.into_inner().cast_signed(),
            kind.to_string(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(case_id.map(CaseId))
    }

    pub async fn get_guild_moderation_case(
        &self,
        guild_id: Id<GuildMarker>,
        case_id: CaseId,
    ) -> Result<Option<GuildModerationCase>, DbError> {
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
                None => Err(DbError::OtherParseError),
            },
            None => Ok(None),
        }
    }

    pub async fn get_last_guild_moderation_case_involving_user(
        &self,
        guild_id: Id<GuildMarker>,
        involved_user_id: Id<UserMarker>,
    ) -> Result<Option<GuildModerationCase>, DbError> {
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
                None => Err(DbError::OtherParseError),
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
    ) -> Result<Option<CaseId>, DbError> {
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

            if moderation_kind.manual_expiration()
                && let Err(e) = self.expiring_cases_tx.send((None, case_id))
            {
                tracing::error!("{e}");
            }

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
    ) -> Result<PgQueryResult, DbError> {
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

    async fn get_all_expiring_case_ids(
        pool: &PgPool,
    ) -> Result<Vec<(chrono::DateTime<chrono::Utc>, CaseId)>, DbError> {
        struct CaseExpiryInfo {
            expires_at: Option<chrono::DateTime<chrono::Utc>>,
            case_id: i64,
        }
        let cases = query_as!(
            CaseExpiryInfo,
            "SELECT expires_at, case_id FROM guild_moderation_cases
            WHERE expires_at IS NOT NULL",
        )
        .fetch_all(pool)
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

    pub async fn get_guild_modlog_webhook(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<Option<(Id<WebhookMarker>, String, Option<Id<ChannelMarker>>)>, DbError> {
        let result = query!(
            "SELECT modlog_webhook_id, modlog_webhook_token, modlog_webhook_channel_id FROM guilds
            WHERE guild_id = $1",
            guild_id.into_inner().cast_signed(),
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(result.and_then(|result| {
            Some((
                result.modlog_webhook_id?.cast_unsigned().into(),
                result.modlog_webhook_token?,
                result
                    .modlog_webhook_channel_id
                    .map(|value| value.cast_unsigned().into()),
            ))
        }))
    }

    #[expect(clippy::type_complexity)]
    pub async fn set_guild_modlog_webhook_upsert(
        &self,
        guild_id: Id<GuildMarker>,
        value: Option<(Id<WebhookMarker>, &str, Option<Id<ChannelMarker>>)>,
    ) -> Result<PgQueryResult, DbError> {
        Ok(query!(
            "INSERT INTO guilds (guild_id, command_prefixes, modlog_webhook_id, modlog_webhook_token, modlog_webhook_channel_id)
            VALUES ($1, ARRAY[$2], $3, $4, $5)
            ON CONFLICT (guild_id) DO UPDATE
            SET modlog_webhook_id = $3, modlog_webhook_token = $4, modlog_webhook_channel_id = $5",
            guild_id.into_inner().cast_signed(),
            self.default_prefix,
            value.map(|value| value.0.into_inner().cast_signed()),
            value.map(|value| value.1),
            value.map(|value| value.2.map(|value| value.into_inner().cast_signed())).flatten(),
        )
        .execute(&self.pool)
        .await?)
    }

    /*
        pub async fn maybe_create_case_for_external_timeout(
            &self,
            data: CreateGuildModerationCaseData<'_>,
        ) -> Result<Option<GuildModerationCase>, DbError> {
            let maybe_existing_case_raw = query_as!(
                RawGuildModerationCase,
                "SELECT * FROM guild_moderation_cases
            WHERE guild_id = $1 AND target_id = $2 AND moderation_kind = $3 AND closed = false
            ORDER BY case_id DESC
            LIMIT 1",
                data.guild_id.into_inner().cast_signed(),
                data.target_id.into_inner().cast_signed(),
                data.moderation_kind.to_string(),
            ).fetch_optional(&self.pool).await?;

            let Some(existing_case_raw) = maybe_existing_case_raw else {
                return Ok(None);
            };

            if existing_case_raw.expires_at == data.expiry.map(|expiry| expiry.0) {
                return Ok(None);
            }



            todo!()
        }
    */
}

impl std::ops::Deref for DatabaseManager {
    type Target = SharedDatabaseManager;

    fn deref(&self) -> &Self::Target {
        &self.shared_manager
    }
}

/// Because both of these depend partially on each other they can only be created cleanly at the same time,
/// which this function handles.
pub async fn create_db_manager_and_case_expiration_actor(
    url: &str,
    prefix_cache_capacity: u64,
    default_prefix: String,
    logger: Arc<Logger>,
) -> Result<(DatabaseManager, UnboundedReceiver<CaseId>), DbError> {
    let pool = PgPoolOptions::new().connect(url).await?;
    let existing_cases = DatabaseManager::get_all_expiring_case_ids(&pool).await?;
    let (expired_cases_rx, expiring_cases_tx) = start_case_expiration_actor(existing_cases);
    Ok((
        DatabaseManager {
            pool: pool.clone(),
            expiring_cases_tx,
            cached_prefixes: PrefixCache::new(prefix_cache_capacity),
            default_prefix,
            logger,
            shared_manager: SharedDatabaseManager::new(pool),
        },
        expired_cases_rx,
    ))
}

pub mod schema {
    use fluxer_neptunium::model::id::{
        Id,
        marker::{GuildMarker, UserMarker},
    };
    use rust_shared::db::ModerationKind;

    pub struct CreateGuildModerationCaseData<'a> {
        pub guild_id: Id<GuildMarker>,
        pub target_id: Id<UserMarker>,
        pub moderator_id: Option<Id<UserMarker>>,
        pub moderation_kind: ModerationKind,
        pub reason: Option<&'a str>,
        pub expiry: Option<(chrono::DateTime<chrono::Utc>, i64)>,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }
}
