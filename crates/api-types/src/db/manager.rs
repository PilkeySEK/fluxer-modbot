//! Shared database logic which is used by both api and worker, to avoid duplicating functions.

use std::num::ParseIntError;

use fluxer_neptunium::model::id::{
    Id,
    marker::{GuildMarker, UserMarker},
};
use sqlx::{PgPool, QueryBuilder};

use crate::db::GuildModerationCase;

#[derive(Debug)]
pub enum DbError {
    Sqlx(sqlx::Error),
    JsonParse(serde_json::Error),
    ParseInt(ParseIntError),
    OtherParseError,
}

impl std::error::Error for DbError {}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sqlx(e) => f.write_fmt(format_args!("PostgreSQL error: {e}")),
            Self::JsonParse(e) => f.write_fmt(format_args!("Parse error: {e}")),
            Self::ParseInt(e) => f.write_fmt(format_args!("Error converting string to int: {e}")),
            Self::OtherParseError => f.write_str("Other pase error"),
        }
    }
}

impl From<sqlx::Error> for DbError {
    fn from(value: sqlx::Error) -> Self {
        Self::Sqlx(value)
    }
}

impl From<serde_json::Error> for DbError {
    fn from(value: serde_json::Error) -> Self {
        Self::JsonParse(value)
    }
}

impl From<ParseIntError> for DbError {
    fn from(value: ParseIntError) -> Self {
        Self::ParseInt(value)
    }
}

pub struct SharedDatabaseManager {
    pool: PgPool,
}

impl SharedDatabaseManager {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_guild_moderation_cases(
        &self,
        guild_id: Id<GuildMarker>,
        limit: i64,
        offset: Option<i64>,
        involving_user: Option<Id<UserMarker>>,
    ) -> Result<Vec<GuildModerationCase>, DbError> {
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
            None => Err(DbError::OtherParseError),
        }
    }

    pub async fn count_guild_moderation_cases(
        &self,
        guild_id: Id<GuildMarker>,
        involving_user: Option<Id<UserMarker>>,
    ) -> Result<i64, DbError> {
        if let Some(involving_user) = involving_user {
            Ok(sqlx::query_scalar!(
                "SELECT COUNT(case_id) FROM guild_moderation_cases
                WHERE guild_id = $1 AND (target_id = $2 OR moderator_id = $2)",
                guild_id.into_inner().cast_signed(),
                involving_user.into_inner().cast_signed(),
            )
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0))
        } else {
            Ok(sqlx::query_scalar!(
                "SELECT COUNT(case_id) FROM guild_moderation_cases
                WHERE guild_id = $1",
                guild_id.into_inner().cast_signed(),
            )
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0))
        }
    }
}
