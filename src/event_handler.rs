use std::{collections::HashMap, sync::Arc, time::SystemTime};

use fluxer_neptunium::{
    async_trait,
    cache::{CachedMessage, Guard},
    cached_payload::{CachedMessageCreate, CachedMessageReactionAdd, CachedReady},
    events::{EventError, EventHandler, context::Context},
    exts::ChannelExt,
    model::{
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
};

pub mod reactions;

pub struct BotEventHandler {
    dispatcher: CommandDispatcher,
    bot_name: String,
    started_at: SystemTime,
    db_manager: DatabaseManager,
    default_command_configuration: HashMap<String, Permissions>,
    max_command_prefix_len: usize,
    my_id: Id<UserMarker>,
    reactions_event_handler: ReactionsEventHandler,
}

impl BotEventHandler {
    pub fn new(
        dispatcher: CommandDispatcher,
        bot_name: String,
        db_manager: DatabaseManager,
        default_command_configuration: HashMap<String, Permissions>,
        max_command_prefix_len: usize,
        my_id: Id<UserMarker>,
    ) -> Self {
        Self {
            dispatcher,
            bot_name,
            started_at: SystemTime::now(),
            db_manager,
            default_command_configuration,
            max_command_prefix_len,
            my_id,
            reactions_event_handler: ReactionsEventHandler::new(),
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

        if let Some(content) = message.content.strip_prefix(&format!("<@{}>", self.my_id)) {
            self.execute_command(ctx, &message, guild_id, content).await;
            return Ok(());
        }

        for prefix in guild_prefixes.iter() {
            if let Some(content) = message.content.strip_prefix(prefix) {
                self.execute_command(ctx, &message, guild_id, content).await;
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
}

impl BotEventHandler {
    async fn execute_command(
        &self,
        ctx: Context,
        message: &Guard<Arc<CachedMessage>>,
        guild_id: Id<GuildMarker>,
        content: &str,
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
                    reaction_handler_tx: &self.reactions_event_handler.tx,
                },
                content.trim_start(),
            )
            .await
        {
            tracing::error!("Error executing command: {e}");
        }
    }
}
