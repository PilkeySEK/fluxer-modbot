use std::{collections::HashMap, sync::Arc, time::SystemTime};

use api_types::ws::WorkerToApiMessage;
use fluxer_neptunium::{
    async_trait,
    cache::{Cached, CachedGuildMember, CachedGuildRole, CachedMessage, Guard},
    cached_payload::{
        CachedGuildRoleUpdateBulk, CachedMessageCreate, CachedMessageReactionAdd, CachedReady,
    },
    events::{EventError, EventHandler, context::Context},
    exts::ChannelExt,
    model::{
        gateway::payload::incoming::{GuildMemberRemove, GuildRoleDelete},
        guild::permissions::Permissions,
        id::{
            Id,
            marker::{GuildMarker, UserMarker},
        },
    },
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    commands::{CommandContext, CommandDispatcher},
    db::DatabaseManager,
    event_handler::reactions::ReactionsEventHandler,
    logging::Logger,
};

pub mod reactions;

pub struct BotEventHandler {
    dispatcher: CommandDispatcher,
    bot_name: String,
    started_at: SystemTime,
    db_manager: Arc<DatabaseManager>,
    default_command_configuration: HashMap<String, Permissions>,
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
        dispatcher: CommandDispatcher,
        bot_name: String,
        db_manager: Arc<DatabaseManager>,
        default_command_configuration: HashMap<String, Permissions>,
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
            default_command_configuration,
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
            self.execute_command(ctx, &message, guild_id, content, guild_prefixes)
                .await;
            return Ok(());
        }

        for prefix in guild_prefixes.iter() {
            if let Some(content) = message.content.strip_prefix(prefix) {
                self.execute_command(ctx, &message, guild_id, content, guild_prefixes)
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
    async fn execute_command(
        &self,
        ctx: Context,
        message: &Guard<Arc<CachedMessage>>,
        guild_id: Id<GuildMarker>,
        content: &str,
        guild_command_prefixes: Arc<Vec<String>>,
    ) {
        if let Err(e) = self
            .dispatcher
            .execute(
                CommandContext {
                    ctx: &ctx,
                    message,
                    bot_name: &self.bot_name,
                    started_at: &self.started_at,
                    db: &self.db_manager,
                    guild_id,
                    default_command_configuration: &self.default_command_configuration,
                    max_command_prefix_len: self.max_command_prefix_len,
                    max_command_prefixes: self.max_command_prefixes,
                    reaction_handler_tx: &self.reactions_event_handler.tx,
                    logger: &self.logger,
                    webhook_avatar_b64: match &self.webhook_avatar_b64 {
                        Some(avatar) => Some(avatar),
                        None => None,
                    },
                    bot_id: self.bot_id,
                    guild_command_prefixes: guild_command_prefixes
                        .iter()
                        .map(String::as_str)
                        .collect(),
                },
                content.trim_start(),
            )
            .await
        {
            tracing::error!("Error executing command: {e}");
        }
    }
}
