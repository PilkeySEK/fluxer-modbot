use std::{sync::Arc, time::Duration};

use api_types::{
    DashboardUserSettings,
    db::{DbError, SharedDatabaseManager},
    ws::ApiToWorkerMessage,
};
use fluxer_neptunium::model::id::{
    Id,
    marker::{GuildMarker, UserMarker},
};
use sqlx::{FromRow, PgPool, QueryBuilder};
use tokio::{
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
};

use crate::db::schema::{GuildConfig, GuildUpdates, SessionJsonData};

pub struct DbManager {
    pool: sqlx::PgPool,
    session_expiry_thread_stop_tx: UnboundedSender<()>,
    session_expiry_thread_handle: JoinHandle<()>,
    default_command_prefix: String,
    api_to_worker_tx: Arc<tokio::sync::RwLock<Option<UnboundedSender<ApiToWorkerMessage>>>>,
    shared_manager: SharedDatabaseManager,
}

impl DbManager {
    pub async fn new(
        url: &str,
        default_command_prefix: String,
        api_to_worker_tx: Arc<tokio::sync::RwLock<Option<UnboundedSender<ApiToWorkerMessage>>>>,
    ) -> Result<Self, sqlx::Error> {
        let pool = PgPool::connect(url).await?;

        let (session_expiry_thread_stop_tx, session_expiry_thread_stop_rx) = unbounded_channel();

        let session_expiry_thread_handle = tokio::spawn(session_expiry_thread(
            pool.clone(),
            session_expiry_thread_stop_rx,
        ));

        Ok(Self {
            pool: pool.clone(),
            session_expiry_thread_stop_tx,
            session_expiry_thread_handle,
            default_command_prefix,
            api_to_worker_tx,
            shared_manager: SharedDatabaseManager::new(pool),
        })
    }

    pub async fn create_dash_session(
        &self,
        session_token: &str,
        user_id: Id<UserMarker>,
        expires_at: chrono::DateTime<chrono::Utc>,
        data: SessionJsonData,
    ) -> Result<(), sqlx::Error> {
        #[expect(clippy::unwrap_used)]
        sqlx::query!(
            "INSERT INTO dash_sessions (session_token, user_id, data, expires_at)
            VALUES ($1, $2, $3, $4)",
            session_token,
            user_id.into_inner().cast_signed(),
            serde_json::to_value(&data).unwrap(),
            expires_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_dash_session(&self, session_token: &str) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM dash_sessions WHERE session_token = $1",
            session_token,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_session_data_by_session_token(
        &self,
        session_token: &str,
    ) -> Result<Option<SessionJsonData>, DbError> {
        let json = sqlx::query_scalar!(
            "SELECT data FROM dash_sessions
            WHERE session_token = $1 AND expires_at > NOW()",
            session_token,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(DbError::Sqlx)?;

        let Some(json) = json else {
            return Ok(None);
        };

        Ok(Some(
            serde_json::from_value(json).map_err(DbError::JsonParse)?,
        ))
    }

    pub async fn update_guild(
        &self,
        guild_id: Id<GuildMarker>,
        updates: GuildUpdates,
    ) -> Result<GuildConfig, DbError> {
        let mut query_builder = QueryBuilder::new("INSERT INTO guilds (guild_id");
        if updates.command_prefixes.is_some() {
            query_builder.push(", command_prefixes");
        }
        query_builder
            .push(") VALUES (")
            .push_bind(guild_id.into_inner().cast_signed());
        if let Some(command_prefixes) = &updates.command_prefixes {
            query_builder.push(", ").push_bind(command_prefixes);
        }
        query_builder.push(") ON CONFLICT (guild_id) DO UPDATE SET guild_id = $1");
        if let Some(command_prefixes) = &updates.command_prefixes {
            query_builder
                .push(", command_prefixes = ")
                .push_bind(command_prefixes);
        }
        query_builder.push(" RETURNING *");

        let row = query_builder
            .build()
            .fetch_one(&self.pool)
            .await
            .map_err(DbError::Sqlx)?;

        if let Some(tx) = &*self.api_to_worker_tx.read().await
            && updates.command_prefixes.is_some()
        {
            let _ = tx.send(ApiToWorkerMessage::InvalidateCachedGuildPrefixes(guild_id));
        }

        GuildConfig::from_row(&row).map_err(DbError::Sqlx)
    }

    pub async fn get_guild_config_upsert(
        &self,
        guild_id: Id<GuildMarker>,
    ) -> Result<GuildConfig, DbError> {
        let guild = sqlx::query!(
            "INSERT INTO guilds (guild_id, command_prefixes)
            VALUES ($1, ARRAY[$2]::TEXT[])
            ON CONFLICT (guild_id) DO UPDATE
            SET guild_id = EXCLUDED.guild_id
            RETURNING *",
            guild_id.into_inner().cast_signed(),
            self.default_command_prefix,
        )
        .fetch_one(&self.pool)
        .await
        .map_err(DbError::Sqlx)?;

        Ok(GuildConfig {
            command_prefixes: guild.command_prefixes,
        })
    }

    pub async fn get_dashboard_user_settings_upsert(
        &self,
        user_id: Id<UserMarker>,
    ) -> Result<DashboardUserSettings, DbError> {
        #[expect(clippy::unwrap_used)]
        let settings = sqlx::query_scalar!(
            "INSERT INTO dashboard_user_settings (user_id, settings)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE
            SET user_id = EXCLUDED.user_id
            RETURNING settings",
            user_id.into_inner().cast_signed(),
            serde_json::to_value(DashboardUserSettings::default()).unwrap(),
        )
        .fetch_one(&self.pool)
        .await
        .map_err(DbError::Sqlx)?;

        serde_json::from_value(settings).map_err(DbError::JsonParse)
    }

    pub async fn set_dashboard_user_settings(
        &self,
        user_id: Id<UserMarker>,
        settings: DashboardUserSettings,
    ) -> Result<(), DbError> {
        #[expect(clippy::unwrap_used)]
        sqlx::query!(
            "INSERT INTO dashboard_user_settings (user_id, settings)
            VALUES ($1, $2)
            ON CONFLICT (user_id) DO UPDATE
            SET settings = $2",
            user_id.into_inner().cast_signed(),
            serde_json::to_value(settings).unwrap(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn stop(self) {
        if self.session_expiry_thread_stop_tx.send(()).is_err() {
            return;
        }
        let _ = self.session_expiry_thread_handle.await;
    }
}

impl std::ops::Deref for DbManager {
    type Target = SharedDatabaseManager;

    fn deref(&self) -> &Self::Target {
        &self.shared_manager
    }
}

async fn session_expiry_thread(
    pool: PgPool,
    mut session_expiry_thread_stop_rx: UnboundedReceiver<()>,
) {
    async fn expire_sessions(pool: &PgPool) -> Result<(), sqlx::Error> {
        sqlx::query!(
            "DELETE FROM dash_sessions
            WHERE expires_at < NOW()"
        )
        .execute(pool)
        .await?;
        Ok(())
    }

    let mut interval = tokio::time::interval(Duration::from_hours(3));
    // Commented so that we immediately invalidate expired sessions when the thread initially starts
    // interval.tick().await;
    loop {
        tokio::select! {
            _ = interval.tick() => {
                if let Err(e) = expire_sessions(&pool).await {
                    tracing::error!("Error expiring sessions: {e}");
                }
            },
            _ = session_expiry_thread_stop_rx.recv() => {
                break;
            }
        }
    }
}

pub mod schema {
    use fluxer_neptunium::model::id::{Id, marker::UserMarker};
    use serde::{Deserialize, Serialize};
    use sqlx::prelude::FromRow;

    #[derive(Serialize, Deserialize)]
    pub struct SessionJsonData {
        pub bearer_token: String,
        pub user_id: Id<UserMarker>,
    }

    pub struct SessionData {
        pub data: SessionJsonData,
        pub session_token: zeroize::Zeroizing<String>,
    }

    #[derive(Serialize, utoipa::ToSchema, FromRow)]
    pub struct GuildConfig {
        pub command_prefixes: Vec<String>,
    }

    #[derive(Deserialize, utoipa::ToSchema)]
    pub struct GuildUpdates {
        pub command_prefixes: Option<Vec<String>>,
    }
}
