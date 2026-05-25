use fluxer_neptunium::{
    create_embed,
    events::context::Context,
    http::endpoints::{
        channel::CreateMessageBody,
        webhooks::{ExecuteWebhook, WebhookMessage},
    },
    model::{
        id::{Id, marker::GuildMarker},
        time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
    },
};
use pretty_duration::pretty_duration;
use zeroize::Zeroizing;

use crate::db::{DatabaseManager, schema::GuildModerationCase};

pub enum ModLogEntry {
    CaseCreated(GuildModerationCase),
}

pub struct Logger {
    ctx: Context,
}

impl Logger {
    pub fn new(ctx: Context) -> Self {
        Self { ctx }
    }

    pub async fn create_modlog_entry(
        &self,
        db: &DatabaseManager,
        guild_id: Id<GuildMarker>,
        entry: ModLogEntry,
    ) {
        let modlog_webhook = match db.get_guild_modlog_webhook(guild_id).await {
            Ok(webhook_url) => webhook_url,
            Err(e) => {
                tracing::error!("Database error getting modlog channel for guild {guild_id}: {e}");
                return;
            }
        };

        let Some((webhook_id, webhook_token, _)) = modlog_webhook else {
            return;
        };

        if let Err(e) = self
            .ctx
            .get_http_client()
            .execute(ExecuteWebhook {
                webhook_id,
                token: Zeroizing::new(webhook_token),
                message: WebhookMessage {
                    base: entry.into_message(),
                    username: None,
                    avatar_url: None,
                },
                wait: false,
            })
            .await
        {
            tracing::error!("Error executing modlog webhook for guild {guild_id}: {e:?}");
        }
    }
}

impl ModLogEntry {
    fn into_message(self) -> CreateMessageBody {
        match self {
            ModLogEntry::CaseCreated(case) => {
                let description = format!(
                    "> **Type:** {}\n> **Target:** <@{}> ({})\n> **Reason:** {}\n> **Moderator:** {}\n> **Duration:** {}",
                    case.moderation_kind,
                    case.target_id,
                    case.target_id,
                    case.reason
                        .unwrap_or_else(|| "*No reason provided.*".to_string()),
                    case.moderator_id.map_or_else(
                        || "*Automated action.*".to_string(),
                        |id| format!("<@{id}>")
                    ),
                    case.expiry.map_or_else(
                        || "Permanent".to_string(),
                        |(expires_at, duration)| format!(
                            "{} (expires {})",
                            pretty_duration(&duration, crate::PRETTY_DURATION_OPTIONS),
                            Timestamp::<UnixMillis>::from(expires_at)
                                .time_string(TimestampDisplayType::Relative)
                        )
                    ),
                );
                create_embed!(
                    title: format!("Case `{}`", case.case_id),
                    description: description,
                )
                .into()
            }
        }
    }
}
