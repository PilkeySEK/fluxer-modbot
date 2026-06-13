//! Shared database logic which is used by both api and worker, to avoid duplicating functions.

use std::{num::ParseIntError, str::FromStr};

use fluxer_neptunium::model::id::{
    Id,
    marker::{GuildMarker, UserMarker},
};
use sqlx::{PgPool, Postgres, QueryBuilder, postgres::PgQueryResult};

use crate::db::{CaseId, GuildModerationCase, UserInfo};

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

    pub async fn list_guild_moderation_cases_by_query(
        &self,
        guild_id: Id<GuildMarker>,
        limit: i64,
        offset: Option<i64>,
        query: Option<&str>,
    ) -> Result<Vec<GuildModerationCase>, DbError> {
        let mut qb = QueryBuilder::new("SELECT * FROM guild_moderation_cases WHERE guild_id = ");
        qb.push_bind(guild_id.into_inner().cast_signed());
        if let Some(query) = query {
            qb.push(" AND ");
            add_query_to_builder(&mut qb, query);
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

    pub async fn list_guild_moderation_cases_by_involving_user(
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

    pub async fn count_guild_moderation_cases_by_involving_user(
        &self,
        guild_id: Id<GuildMarker>,
        involving_user: Option<Id<UserMarker>>,
    ) -> Result<i64, DbError> {
        Ok(if let Some(involving_user) = involving_user {
            sqlx::query_scalar!(
                "SELECT COUNT(case_id) FROM guild_moderation_cases
                WHERE guild_id = $1 AND (target_id = $2 OR moderator_id = $2)",
                guild_id.into_inner().cast_signed(),
                involving_user.into_inner().cast_signed(),
            )
            .fetch_one(&self.pool)
            .await?
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(case_id) FROM guild_moderation_cases
                WHERE guild_id = $1",
                guild_id.into_inner().cast_signed(),
            )
            .fetch_one(&self.pool)
            .await?
        }
        .unwrap_or(0))
    }

    pub async fn count_guild_moderation_cases_by_query(
        &self,
        guild_id: Id<GuildMarker>,
        query: Option<&str>,
    ) -> Result<i64, DbError> {
        Ok(if let Some(query) = query {
            let mut qb = QueryBuilder::new(
                "SELECT COUNT(case_id) FROM guild_moderation_cases WHERE guild_id = ",
            );
            qb.push_bind(guild_id.into_inner().cast_signed());
            qb.push(" AND ");

            add_query_to_builder(&mut qb, query);

            qb.build_query_scalar::<Option<i64>>()
                .fetch_one(&self.pool)
                .await?
                .unwrap_or(0)
        } else {
            sqlx::query_scalar!(
                "SELECT COUNT(case_id) FROM guild_moderation_cases
                WHERE guild_id = $1",
                guild_id.into_inner().cast_signed(),
            )
            .fetch_one(&self.pool)
            .await?
            .unwrap_or(0)
        })
    }

    pub async fn get_user_info(
        &self,
        user_id: Id<UserMarker>,
    ) -> Result<Option<UserInfo>, DbError> {
        let record = sqlx::query!(
            "SELECT * FROM user_info WHERE user_id = $1",
            user_id.into_inner().cast_signed(),
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record.map(|record| UserInfo {
            user_id,
            avatar: record.avatar,
            username: record.username,
            discriminator: record.discriminator,
            global_name: record.global_name,
        }))
    }

    /// Inserts new user info or updates the existing one.
    pub async fn set_user_info(&self, info: UserInfo) -> Result<PgQueryResult, DbError> {
        Ok(sqlx::query!(
            "INSERT INTO user_info (user_id, avatar, username, discriminator, global_name)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (user_id) DO UPDATE
            SET avatar=$2, username=$3, discriminator=$4, global_name=$5",
            info.user_id.into_inner().cast_signed(),
            info.avatar,
            info.username,
            info.discriminator,
            info.global_name,
        )
        .execute(&self.pool)
        .await?)
    }
}

/// pushes `(...)`
fn add_query_to_builder(qb: &mut QueryBuilder<Postgres>, query: &str) {
    qb.push("(FALSE");
    if let Ok(id) = Id::<UserMarker>::try_from(query) {
        let id = id.into_inner().cast_signed();
        qb.push(" OR target_id = ")
            .push_bind(id)
            .push(" OR moderator_id = ")
            .push_bind(id);
    }
    if let Ok(CaseId(case_id)) = CaseId::from_str(query) {
        qb.push(" OR case_id = ").push_bind(case_id);
    }

    qb.push(" OR (reason IS NOT NULL AND reason ILIKE ")
        .push_bind(format!("%{}%", escape_like(query)))
        .push(")");
    qb.push(")");
}

fn escape_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

#[cfg(test)]
mod tests {
    #[test]
    fn escape_like() {
        assert_eq!(super::escape_like("%abcdefg%"), r#"\%abcdefg\%"#);
        assert_eq!(super::escape_like("_abc%"), r#"\_abc\%"#);
        assert_eq!(super::escape_like("_____"), r#"\_\_\_\_\_"#);
        assert_eq!(super::escape_like(r#"\%"#), r#"\\\%"#);
        assert_eq!(super::escape_like(r#"\\%"#), r#"\\\\\%"#);
        assert_eq!(super::escape_like(r#"_\"#), r#"\_\\"#);
    }
}
