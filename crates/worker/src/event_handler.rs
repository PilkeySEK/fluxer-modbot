use std::{collections::HashMap, sync::Arc, time::SystemTime};

use api_types::is_guild_manager_permissions;
use fluxer_neptunium::{
    async_trait,
    cache::{Cached, CachedGuildMember, CachedMessage, Guard},
    cached_payload::{
        CachedGuildCreate, CachedMessageCreate, CachedMessageReactionAdd, CachedReady,
    },
    events::{EventError, EventHandler, context::Context},
    exts::{ChannelExt, GuildExt, GuildMemberExt},
    http::endpoints::guild::SearchGuildMembersBody,
    model::{
        gateway::payload::incoming::GuildMemberRemove,
        guild::permissions::Permissions,
        id::{
            Id,
            marker::{GuildMarker, UserMarker},
        },
    },
};

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
        ctx: Context,
        member: Cached<CachedGuildMember>,
    ) -> Result<(), EventError> {
        update_guild_member_in_db(&self.db_manager, &ctx, &member).await?;
        Ok(())
    }

    async fn on_guild_member_update(
        &self,
        ctx: Context,
        member: Cached<CachedGuildMember>,
    ) -> Result<(), EventError> {
        update_guild_member_in_db(&self.db_manager, &ctx, &member).await?;

        Ok(())
    }

    async fn on_guild_member_remove(
        &self,
        _ctx: Context,
        event: Arc<GuildMemberRemove>,
    ) -> Result<(), EventError> {
        if let Err(e) = self
            .db_manager
            .remove_guild_member(event.guild_id, event.user.id)
            .await
        {
            tracing::error!(
                "DB Error removing guild member {} in guild {}: {}",
                event.user.id,
                event.guild_id,
                e
            );
        }

        Ok(())
    }

    // When the bot is added to a guild a guild create event is sent by the gateway
    async fn on_guild_create(
        &self,
        ctx: Context,
        event: Arc<CachedGuildCreate>,
    ) -> Result<(), EventError> {
        let me = event.guild.get_current_member(&ctx).await?;
        if !me.has_permissions(&ctx, Permissions::MANAGE_GUILD).await? {
            tracing::warn!(
                "I do not have MANAGE_GUILD in {} (\"{}\"), so I can't search members.",
                event.guild.id,
                event.guild.name
            );
            return Ok(());
        }
        let removed_members = match self
            .db_manager
            .remove_all_guild_members(event.guild.id)
            .await
        {
            Err(e) => {
                tracing::error!("Error removing all guild members in DB: {e}");
                return Ok(());
            }
            Ok(query_result) => query_result.rows_affected(),
        };
        // for member in &event.members {
        //     update_guild_member_in_db(&self.db_manager, &ctx, member).await?;
        // }

        // Complicated way of getting users which have a manager role and adding those users to the DB

        let guild_owner = event.guild.get_member(&ctx, event.guild.owner_id).await?;
        update_guild_member_in_db(&self.db_manager, &ctx, &guild_owner).await?;

        let manager_roles = &event
            .guild
            .list_roles(&ctx)
            .await?
            .into_iter()
            .filter_map(|role| {
                if is_guild_manager_permissions(role.permissions) {
                    Some(role.id)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let members = event
            .guild
            .search_members_all(
                &ctx,
                SearchGuildMembersBody::builder()
                    .role_ids(manager_roles.clone())
                    .build(),
            )
            .await?;
        let members_len = members.len();
        for member in members {
            let member = event.guild.get_member(&ctx, member.user_id).await?;
            update_guild_member_in_db(&self.db_manager, &ctx, &member).await?;
        }
        tracing::debug!(
            "Synced {members_len} guild managers with the database (removed {removed_members} before)."
        );

        Ok(())
    }

    // async fn on_guild_member_update(
    //     &self,
    //     ctx: Context,
    //     member: Cached<CachedGuildMember>,
    // ) -> Result<(), EventError> {
    //     member.refresh();
    //
    //     let mute_case = self
    //         .db_manager
    //         .close_and_get_latest_open_moderation_case_by_kind_and_user(
    //             member.guild_id,
    //             member.id,
    //             ModerationKind::Mute,
    //             reason,
    //             closed_by,
    //         );
    //
    //     Ok(())
    // }
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

async fn update_guild_member_in_db(
    db_manager: &DatabaseManager,
    ctx: &Context,
    member: &CachedGuildMember,
) -> Result<(), EventError> {
    let permissions = member.get_permissions(ctx).await?;

    // Only fetches each guild once because of caching
    let guild_owner = member.guild_id.fetch(ctx).await?.owner_id;

    if let Err(e) = db_manager
        .update_or_insert_guild_member(
            member.guild_id,
            member.id,
            permissions,
            member.id == guild_owner,
        )
        .await
    {
        tracing::error!(
            "DB Error updating or inserting guild member {} in guild {}: {}",
            member.id,
            member.guild_id,
            e
        );
    }

    Ok(())
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
