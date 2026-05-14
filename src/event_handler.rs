use std::{collections::HashMap, sync::Arc, time::SystemTime};

use fluxer_neptunium::{
    async_trait,
    cached_payload::{CachedMessageCreate, CachedReady},
    events::{EventError, EventHandler, context::Context},
    exts::ChannelExt,
    model::guild::permissions::Permissions,
};

use crate::{
    commands::{CommandContext, CommandDispatcher},
    db::DatabaseManager,
};

pub struct BotEventHandler {
    dispatcher: CommandDispatcher,
    bot_name: String,
    started_at: SystemTime,
    db_manager: DatabaseManager,
    default_command_prefix: String,
    default_command_configuration: HashMap<String, Permissions>,
}

impl BotEventHandler {
    pub fn new(
        dispatcher: CommandDispatcher,
        bot_name: String,
        db_manager: DatabaseManager,
        default_command_prefix: String,
        default_command_configuration: HashMap<String, Permissions>,
    ) -> Self {
        Self {
            dispatcher,
            bot_name,
            started_at: SystemTime::now(),
            db_manager,
            default_command_prefix,
            default_command_configuration,
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

        let guild_prefixes = match self.db_manager.get_guild_command_prefixes(guild_id).await {
            Ok(Some(prefixes)) => prefixes,
            Ok(None) => vec![self.default_command_prefix.clone()],
            Err(e) => {
                tracing::error!("Error getting command prefixes for guild {guild_id}: {e}");
                return Ok(());
            }
        };

        for prefix in guild_prefixes {
            if let Some(content) = message.content.strip_prefix(&prefix) {
                if let Err(e) = self
                    .dispatcher
                    .execute(
                        CommandContext {
                            ctx: &ctx,
                            message: &message,
                            bot_name: &self.bot_name,
                            started_at: &self.started_at,
                            db: &self.db_manager,
                            guild_id,
                            default_command_configuration: &self.default_command_configuration,
                        },
                        content,
                    )
                    .await
                {
                    tracing::error!("Error executing command: {e}");
                }
                break;
            }
        }

        Ok(())
    }
}
