use std::{sync::Arc, time::SystemTime};

use anyhow::Context as _;
use api_types::db::{CommandId, GuildCommandConfig, GuildCommandPermissionConfig};
use async_brigadier::{CommandError, arg::CommandArgument, parse_arg_recursively};
use chrono::{TimeDelta, Utc};
use enum_map::{EnumMap, enum_map};
use fluxer_neptunium::{
    cache::{Cached, CachedMessage},
    events::context::Context,
    exts::{GuildExt, GuildMemberExt, MessageExt},
    http::endpoints::channel::CreateMessageBody,
    model::{
        channel::message::embed::{MessageEmbed, MessageEmbedBase},
        guild::permissions::Permissions,
        id::{
            Id,
            marker::{GuildMarker, MessageMarker, UserMarker},
        },
        misc::HexColor,
    },
};
use mini_moka::sync::Cache;
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    commands::{
        cases::{case_info, list_cases},
        guild_settings::{
            add_prefix, clear_modlog_webhook, list_prefixes, modlog_channel, remove_prefix,
            set_modlog_webhook,
        },
        misc::ping,
        moderation::{ban, kick, mute, unban, unmute, unwarn, warn},
    },
    config::DefaultCommandConfig,
    db::DatabaseManager,
    event_handler::reactions::{
        ReactionExpiryHandlerFn, ReactionHandler, ReactionsEventHandlerMessage,
    },
    logging::Logger,
};

mod args;
mod cases;
mod guild_settings;
mod misc;
mod moderation;

// pub type Dispatcher = CommandDispatcher<CommandContext, anyhow::Result<()>>;
// pub type Dispatcher = BotDispatcher<CommandContext, anyhow::Result<()>>;
pub type Ctx = async_brigadier::CommandContext<CommandContext>;
type Arg = CommandArgument<CommandContext, anyhow::Result<()>>;

pub enum BotCommandError {
    MissingPermissions,
    UnknownCommand,
    #[expect(unused)]
    Command(CommandError),
    Any(anyhow::Error),
}

impl From<anyhow::Error> for BotCommandError {
    fn from(value: anyhow::Error) -> Self {
        Self::Any(value)
    }
}
impl From<CommandError> for BotCommandError {
    fn from(value: CommandError) -> Self {
        Self::Command(value)
    }
}

pub struct CommandContext {
    pub ctx: Context,
    pub message: Cached<CachedMessage>,
    pub bot_name: String,
    pub started_at: SystemTime,
    pub db: Arc<DatabaseManager>,
    pub guild_id: Id<GuildMarker>,
    pub max_command_prefix_len: usize,
    pub max_command_prefixes: usize,
    pub reaction_handler_tx: UnboundedSender<ReactionsEventHandlerMessage>,
    pub logger: Arc<Logger>,
    pub webhook_avatar_b64: Option<String>,
    pub bot_id: Id<UserMarker>,
    pub guild_command_prefixes: Arc<Vec<String>>,
}

pub struct Dispatcher {
    commands: EnumMap<CommandId, CommandArgument<CommandContext, anyhow::Result<()>>>,
    config_cache_by_name:
        mini_moka::sync::Cache<(Id<GuildMarker>, String), Arc<GuildCommandConfig>>,
    config_cache_by_id:
        mini_moka::sync::Cache<(Id<GuildMarker>, CommandId), Arc<GuildCommandConfig>>,
    db: Arc<DatabaseManager>,
    default_command_config: DefaultCommandConfig,
}

impl Dispatcher {
    #[must_use]
    pub fn new(db: Arc<DatabaseManager>, default_command_config: DefaultCommandConfig) -> Self {
        Self {
            commands: enum_map! {
                CommandId::ListCases => list_cases(),
                CommandId::CaseInfo => case_info(),
                CommandId::AddPrefix => add_prefix(),
                CommandId::RemovePrefix => remove_prefix(),
                CommandId::ListPrefixes => list_prefixes(),
                CommandId::SetModlogWebhook => set_modlog_webhook(),
                CommandId::ClearModlogWebhook => clear_modlog_webhook(),
                CommandId::SetModlogChannel => modlog_channel(),
                CommandId::Ping => ping(),
                CommandId::Warn => warn(),
                CommandId::Unwarn => unwarn(),
                CommandId::Mute => mute(),
                CommandId::Unmute => unmute(),
                CommandId::Kick => kick(),
                CommandId::Ban => ban(),
                CommandId::Unban => unban(),
            },
            config_cache_by_name: Cache::new(4096),
            config_cache_by_id: Cache::new(4096),
            db,
            default_command_config,
        }
    }

    async fn get_command_config<'a>(
        &self,
        command: &'a str,
        guild_id: Id<GuildMarker>,
    ) -> anyhow::Result<Option<(&'a str, Arc<GuildCommandConfig>)>> {
        let command = command.trim_start();
        let (word, rest) = command.split_once(' ').unwrap_or((command, ""));
        let word = word.to_owned();

        if let Some(config) = self.config_cache_by_name.get(&(guild_id, word.clone())) {
            return Ok(Some((rest, config)));
        }
        let command_config = self
            .db
            .get_guild_command_config_from_command_name(guild_id, &word)
            .await?;
        let Some(command_config) = command_config else {
            return Ok(None);
        };
        let command_config = Arc::new(command_config);
        // We don't insert every possible name into the cache because probably only one of the names is actually being used
        self.config_cache_by_name
            .insert((guild_id, word), Arc::clone(&command_config));
        self.config_cache_by_id.insert(
            (guild_id, command_config.command_id),
            Arc::clone(&command_config),
        );
        Ok(Some((rest, command_config)))
    }

    async fn get_command_permissions_of_id(
        &self,
        guild_id: Id<GuildMarker>,
        command_id: CommandId,
    ) -> anyhow::Result<either::Either<Arc<GuildCommandPermissionConfig>, Permissions>> {
        if let Some(config) = self.config_cache_by_id.get(&(guild_id, command_id)) {
            return Ok(either::Left(Arc::clone(&config.perms)));
        }
        let command_config = self
            .db
            .get_guild_command_config(guild_id, command_id)
            .await?;
        let permissions = if let Some(command_config) = command_config {
            let permissions = Arc::clone(&command_config.perms);
            let command_config = Arc::new(command_config);
            self.config_cache_by_id
                .insert((guild_id, command_id), Arc::clone(&command_config));
            // Here, we don't insert any name into the cache because we don't know which is actually being used
            either::Left(permissions)
        } else {
            let permissions = self.default_command_config.permissions[command_id];
            either::Right(permissions)
        };

        Ok(permissions)
    }

    pub async fn execute(
        &self,
        command: &str,
        context: CommandContext,
    ) -> Result<anyhow::Result<()>, BotCommandError> {
        let (rest, command_id) =
            if let Some(v) = self.get_command_config(command, context.guild_id).await? {
                (v.0, v.1.command_id)
            } else {
                'blk: {
                    for (name, id) in &self.default_command_config.names {
                        if let Some(rest) = command.strip_prefix(name) {
                            break 'blk (rest, *id);
                        }
                    }
                    return Err(BotCommandError::UnknownCommand);
                }
            };
        let permissions = self
            .get_command_permissions_of_id(context.guild_id, command_id)
            .await?;

        let guild_member = context
            .guild_id
            .get_member(&context.ctx, context.message.author.id)
            .await
            .context("Failed to fetch guild member")?;

        let has_permission = match permissions {
            either::Left(permissions) => 'blk: {
                let channel_id = context.message.channel_id;
                let roles = &guild_member.roles;
                let user_permissions = guild_member
                    .get_permissions_in_channel(&context.ctx, context.message.channel_id)
                    .await
                    .context("Failed to determine permissions in channel")?;
                if user_permissions.contains(Permissions::ADMINISTRATOR) {
                    break 'blk true;
                }
                if !permissions.required_channels.is_empty()
                    && !permissions.required_channels.contains(&channel_id)
                {
                    break 'blk false;
                }
                if !permissions.required_roles.is_empty() {
                    for role in roles {
                        if permissions.required_roles.contains(role) {
                            break 'blk true;
                        }
                    }
                }
                permissions.required_permissions.contains(user_permissions)
            }
            either::Right(permissions) => guild_member
                .has_permissions_in_channel(&context.ctx, context.message.channel_id, permissions)
                .await
                .context("Failed to determine whether the user has permissions in the channel")?,
        };

        if !has_permission {
            return Err(BotCommandError::MissingPermissions);
        }

        let arg = &self.commands[command_id];

        let node = parse_arg_recursively(rest, arg, &context)?;
        let Some(executes) = node.executes else {
            return Err(CommandError::NotExecutable.into());
        };
        Ok(executes
            .call(async_brigadier::CommandContext {
                context,
                args: node.args,
            })
            .await)
    }
}

impl CommandContext {
    /// Helper for registering a reaction handler. For more control, use `reaction_handler_tx` on this struct.
    ///
    /// If an error occurs, it is logged but no panics will happen.
    pub fn register_reaction_handler(
        &self,
        message_id: Id<MessageMarker>,
        handler: impl ReactionHandler + 'static,
        expiry: Option<(ReactionExpiryHandlerFn, std::time::Duration)>,
    ) {
        let expiry = match expiry {
            Some((f, expires_in)) => {
                let expires_in: TimeDelta = match TimeDelta::from_std(expires_in) {
                    Ok(expires_in) => expires_in,
                    Err(e) => {
                        tracing::error!("Failed to convert Duration to TimeDelta: {e}");
                        return;
                    }
                };
                let now = Utc::now();
                let Some(expires_at) = now.checked_add_signed(expires_in) else {
                    tracing::error!("Overflow calculating expires_at.");
                    return;
                };
                Some((f, expires_at))
            }
            None => None,
        };
        if self
            .reaction_handler_tx
            .send((message_id, Box::new(handler), expiry))
            .is_err()
        {
            tracing::error!("The reaction handler is gone.");
        }
    }

    /// Intentionally does not return the created message because it might be
    /// deleted automatically after a certain time depending on the guild config (in the future).
    pub async fn reply(
        &self,
        content: impl Into<CreateMessageBody> + Send + Sync,
    ) -> anyhow::Result<()> {
        self.message.reply(&self.ctx, content).await?;
        Ok(())
    }

    /// Intentionally does not return the created message because it might be
    /// deleted automatically after a certain time depending on the guild config (in the future).
    pub async fn reply_embed(
        &self,
        title: Option<&str>,
        description: impl Into<String>,
        color: Option<u32>,
    ) -> anyhow::Result<()> {
        self.reply(
            MessageEmbed::builder()
                .base(
                    MessageEmbedBase::builder()
                        .maybe_title(title)
                        .description(description)
                        .maybe_color(color.map(HexColor::new))
                        .build(),
                )
                .build(),
        )
        .await
    }
}
