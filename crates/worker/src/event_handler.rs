use std::{sync::Arc, time::SystemTime};

use anyhow::Context as _;
use fluxer_neptunium::{
    async_trait,
    cache::{Cached, CachedGuildMember, CachedGuildRole, CachedMessage},
    cached_payload::{
        CachedGuildRoleUpdateBulk, CachedMessageCreate, CachedMessageReactionAdd, CachedReady,
    },
    create_embed,
    events::{EventError, EventHandler, context::Context},
    exts::{ChannelExt, MessageExt},
    model::{
        gateway::payload::incoming::{GuildMemberRemove, GuildRoleDelete},
        id::{
            Id,
            marker::{GuildMarker, UserMarker},
        },
    },
};
use rand::distr::{Alphanumeric, SampleString};
use rust_shared::ws::WorkerToApiMessage;
use tokio::sync::mpsc::UnboundedSender;
use tracing::instrument;

use crate::{
    commands::{BotCommandError, CommandContext, Dispatcher},
    db::DatabaseManager,
    event_handler::reactions::ReactionsEventHandler,
    logging::Logger,
};

pub mod reactions;

pub struct BotEventHandler {
    dispatcher: Dispatcher,
    bot_name: String,
    started_at: SystemTime,
    db_manager: Arc<DatabaseManager>,
    max_command_prefix_len: usize,
    max_command_prefixes: usize,
    reactions_event_handler: ReactionsEventHandler,
    logger: Arc<Logger>,
    webhook_avatar_b64: Option<String>,
    bot_id: Id<UserMarker>,
    api_connection_tx: UnboundedSender<WorkerToApiMessage>,
}

impl BotEventHandler {
    #[expect(clippy::too_many_arguments)]
    pub fn new(
        dispatcher: Dispatcher,
        bot_name: String,
        db_manager: Arc<DatabaseManager>,
        max_command_prefix_len: usize,
        max_command_prefixes: usize,
        logger: Arc<Logger>,
        webhook_avatar_b64: Option<String>,
        bot_id: Id<UserMarker>,
        api_connection_tx: UnboundedSender<WorkerToApiMessage>,
    ) -> Self {
        Self {
            dispatcher,
            bot_name,
            started_at: SystemTime::now(),
            db_manager,
            max_command_prefix_len,
            max_command_prefixes,
            reactions_event_handler: ReactionsEventHandler::new(),
            logger,
            webhook_avatar_b64,
            bot_id,
            api_connection_tx,
        }
    }
}

#[async_trait]
impl EventHandler for BotEventHandler {
    async fn on_ready(&self, _ctx: Context, event: Arc<CachedReady>) -> Result<(), EventError> {
        let me = event.user.load();
        tracing::info!("Logged in as {}#{}!", me.username, me.discriminator);
        Ok(())
    }

    async fn on_message_create(
        &self,
        ctx: Context,
        event: Arc<CachedMessageCreate>,
    ) -> Result<(), EventError> {
        let message = event.message.load();
        let author = message.author.load();
        if author.bot {
            return Ok(());
        }
        let guild_id = {
            let channel = message.channel_id.get(&ctx).await?;
            let Some(guild_id) = channel.load().guild_id else {
                return Ok(());
            };
            guild_id
        };

        let guild_prefixes: Arc<Vec<String>> =
            match self.db_manager.get_guild_command_prefixes(guild_id).await {
                Ok(prefixes) => prefixes,
                Err(e) => {
                    tracing::error!("Error getting command prefixes for guild {guild_id}: {e}");
                    return Ok(());
                }
            };

        if let Some(content) = message.content.strip_prefix(&format!("<@{}>", self.bot_id)) {
            self.execute_command(
                ctx,
                event.message.clone(),
                guild_id,
                content,
                guild_prefixes,
            )
            .await;
            return Ok(());
        }

        for prefix in guild_prefixes.iter() {
            if let Some(content) = message.content.strip_prefix(prefix) {
                self.execute_command(
                    ctx,
                    event.message.clone(),
                    guild_id,
                    content,
                    guild_prefixes,
                )
                .await;
                break;
            }
        }

        Ok(())
    }

    async fn on_message_reaction_add(
        &self,
        _ctx: Context,
        event: Arc<CachedMessageReactionAdd>,
    ) -> Result<(), EventError> {
        self.reactions_event_handler
            .handle_reaction_add(event)
            .await
    }

    async fn on_guild_member_add(
        &self,
        _ctx: Context,
        member: Cached<CachedGuildMember>,
    ) -> Result<(), EventError> {
        if self
            .api_connection_tx
            .send(WorkerToApiMessage::InvalidateCachedGuildPermissionsForUser(
                member.guild_id,
                member.id,
            ))
            .is_err()
        {
            tracing::error!("API connection sender closed");
        }

        Ok(())
    }

    async fn on_guild_member_update(
        &self,
        _ctx: Context,
        member: Cached<CachedGuildMember>,
    ) -> Result<(), EventError> {
        if self
            .api_connection_tx
            .send(WorkerToApiMessage::InvalidateCachedGuildPermissionsForUser(
                member.guild_id,
                member.id,
            ))
            .is_err()
        {
            tracing::error!("API connection sender closed");
        }

        Ok(())
    }

    async fn on_guild_member_remove(
        &self,
        _ctx: Context,
        event: Arc<GuildMemberRemove>,
    ) -> Result<(), EventError> {
        if self
            .api_connection_tx
            .send(WorkerToApiMessage::InvalidateCachedGuildPermissionsForUser(
                event.guild_id,
                event.user.id,
            ))
            .is_err()
        {
            tracing::error!("API connection sender closed");
        }

        Ok(())
    }

    async fn on_guild_role_create(
        &self,
        _ctx: Context,
        role: Cached<CachedGuildRole>,
    ) -> Result<(), EventError> {
        if self
            .api_connection_tx
            .send(WorkerToApiMessage::InvalidateCachedGuildPermissions(
                role.guild_id,
            ))
            .is_err()
        {
            tracing::error!("API connection sender closed");
        }

        Ok(())
    }

    async fn on_guild_role_update(
        &self,
        _ctx: Context,
        role: Cached<CachedGuildRole>,
    ) -> Result<(), EventError> {
        if self
            .api_connection_tx
            .send(WorkerToApiMessage::InvalidateCachedGuildPermissions(
                role.guild_id,
            ))
            .is_err()
        {
            tracing::error!("API connection sender closed");
        }

        Ok(())
    }

    async fn on_guild_role_update_bulk(
        &self,
        _ctx: Context,
        event: Arc<CachedGuildRoleUpdateBulk>,
    ) -> Result<(), EventError> {
        for role in &event.roles {
            if self
                .api_connection_tx
                .send(WorkerToApiMessage::InvalidateCachedGuildPermissions(
                    role.guild_id,
                ))
                .is_err()
            {
                tracing::error!("API connection sender closed");
            }
        }

        Ok(())
    }

    async fn on_guild_role_delete(
        &self,
        _ctx: Context,
        role: Arc<GuildRoleDelete>,
    ) -> Result<(), EventError> {
        if self
            .api_connection_tx
            .send(WorkerToApiMessage::InvalidateCachedGuildPermissions(
                role.guild_id,
            ))
            .is_err()
        {
            tracing::error!("API connection sender closed");
        }

        Ok(())
    }

    /*
        async fn on_guild_audit_log_entry_create(
            &self,
            _ctx: Context,
            event: Arc<GuildAuditLogEntryCreate>,
        ) -> Result<(), EventError> {
            #[expect(clippy::single_match)]
            match event.audit_log_entry.action_type {
                AuditLogActionType::MemberUpdate => {
                    let Some((new, _old)) = event.audit_log_entry.changes.iter().find_map(|change| {
                        if let AuditLogChange::CommunicationDisabledUntil { new, old } = change {
                            Some((new, old))
                        } else {
                            None
                        }
                    }) else {
                        return Ok(());
                    };

                    let reason = if let Some(options) = &event.audit_log_entry.options
                        && let Some(reason) = options.get("timeout_reason")
                        && let serde_json::Value::String(s) = reason
                    {
                        Some(s)
                    } else {
                        None
                    };

                    match self
                        .db_manager
                        .maybe_create_case_for_external_timeout(
                            event.guild_id,
                            new.map(Into::into),
                            reason.map(String::as_str),
                        )
                        .await
                    {
                        Ok(None) => {}
                        Ok(Some(case)) => {
                            self.logger
                                .create_modlog_entry(
                                    &self.db_manager,
                                    case.guild_id,
                                    ModLogEntry::CaseCreated(case),
                                )
                                .await;
                        }
                        Err(e) => {
                            tracing::error!(
                                "Database error maybe creating case for external timeout in guild {}: {}",
                                event.guild_id,
                                e
                            );
                        }
                    }
                }
                _ => {}
            }

            Ok(())
        }
    */
}

impl BotEventHandler {
    #[instrument(skip(self, ctx, message, guild_command_prefixes), fields(user_id = message.author.id.into_inner(), guild_id = guild_id.into_inner()))]
    async fn execute_command(
        &self,
        ctx: Context,
        message: Cached<CachedMessage>,
        guild_id: Id<GuildMarker>,
        content: &str,
        guild_command_prefixes: Arc<Vec<String>>,
    ) {
        let result = self
            .dispatcher
            .execute(
                content,
                CommandContext {
                    ctx: ctx.clone(),
                    message: message.clone(),
                    bot_name: self.bot_name.clone(),
                    started_at: self.started_at,
                    db: Arc::clone(&self.db_manager),
                    guild_id,
                    max_command_prefix_len: self.max_command_prefix_len,
                    max_command_prefixes: self.max_command_prefixes,
                    reaction_handler_tx: self.reactions_event_handler.tx.clone(),
                    logger: Arc::clone(&self.logger),
                    webhook_avatar_b64: self.webhook_avatar_b64.clone(),
                    bot_id: self.bot_id,
                    guild_command_prefixes,
                },
            )
            .await;
        match result {
            Err(BotCommandError::MissingPermissions) => {
                #[expect(clippy::disallowed_macros)]
                if let Err(e) = message
                    .reply(
                        &ctx,
                        create_embed!(
                            description: "You do not have the permissions to execute this command.",
                            color: 0xff0000,
                        ),
                    )
                    .await
                    .context("Failed to reply with missing permissions message")
                {
                    tracing::error!("{e:?}");
                }
            }
            Err(BotCommandError::UnknownCommand) => {
                #[expect(clippy::disallowed_macros)]
                if let Err(e) = message
                    .reply(
                        &ctx,
                        create_embed!(
                            description: "Unknown command.",
                            color: 0xff0000,
                        ),
                    )
                    .await
                    .context("Failed to reply with unknown command message")
                {
                    tracing::error!("{e:?}");
                }
            }
            Err(BotCommandError::Command(_)) => {
                #[expect(clippy::disallowed_macros)]
                if let Err(e) = message
                    .reply(
                        &ctx,
                        create_embed!(
                            description: "Wrong usage.",
                            color: 0xff0000,
                        ),
                    )
                    .await
                    .context("Failed to reply with wrong usage message")
                {
                    tracing::error!("{e:?}");
                }
            }
            Err(BotCommandError::Any(e)) | Ok(Err(e)) => {
                let error_id = Alphanumeric.sample_string(&mut rand::rng(), 16);
                #[expect(clippy::disallowed_macros)]
                if let Err(e) = message
                    .reply(
                        &ctx,
                        create_embed!(
                            description: format!("Internal error [{error_id}]."),
                            color: 0xff0000,
                        ),
                    )
                    .await
                    .context("Failed to reply with internal error message")
                {
                    tracing::error!(%error_id, "{e:?}");
                }
                tracing::error!(%error_id, "{e:?}");
            }
            Ok(Ok(())) => {}
        }
    }
}
