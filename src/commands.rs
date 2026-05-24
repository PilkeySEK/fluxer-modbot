use std::{collections::HashMap, pin::Pin, time::SystemTime};

use chrono::{TimeDelta, Utc};
use fluxer_neptunium::{
    cache::CachedMessage,
    client::error::ClientErrorKind,
    events::{EventError, EventErrorKind, context::Context},
    exts::{GuildExt, GuildMemberExt, MessageExt},
    http::error::error_code::ApiErrorCode,
    model::{
        guild::permissions::Permissions,
        id::{
            Id,
            marker::{GuildMarker, MessageMarker},
        },
    },
};
use tokio::sync::mpsc::UnboundedSender;

use crate::{
    db::{DatabaseError, DatabaseManager, schema::GuildCommandConfiguration},
    event_handler::reactions::{
        ReactionExpiryHandlerFn, ReactionHandler, ReactionsEventHandlerMessage,
    },
    macros::{debug_panic, embed_default_footer, embed_default_footer_raw},
};

mod cases;
mod guild_settings;
mod misc;
mod moderation;

pub trait CommandExecuteFn<'a>: Send + Sync + 'static {
    fn call(
        &self,
        ctx: CommandContext<'a>,
        args: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), CommandError>> + Send + 'a>>;
}

impl<'a, F, Fut> CommandExecuteFn<'a> for F
where
    F: Fn(CommandContext<'a>, &'a str) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), CommandError>> + Send + 'a,
{
    fn call(
        &self,
        ctx: CommandContext<'a>,
        args: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), CommandError>> + Send + 'a>> {
        Box::pin(self(ctx, args))
    }
}

pub struct CommandContext<'a> {
    pub ctx: &'a Context,
    pub message: &'a CachedMessage,
    pub bot_name: &'a str,
    pub started_at: &'a SystemTime,
    pub db: &'a DatabaseManager,
    pub guild_id: Id<GuildMarker>,
    pub default_command_configuration: &'a HashMap<String, Permissions>,
    pub max_command_prefix_len: usize,
    pub reaction_handler_tx: &'a UnboundedSender<ReactionsEventHandlerMessage>,
}

#[derive(Debug)]
pub enum CommandError {
    EventError(EventError),
    DatabaseError(DatabaseError),
    Ignore,
}

impl std::error::Error for CommandError {}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventError(e) => f.write_fmt(format_args!("Event error: {e}")),
            Self::DatabaseError(e) => f.write_fmt(format_args!("Database error: {e}")),
            Self::Ignore => f.write_str("Ignored error, already handled"),
        }
    }
}

impl<T> From<T> for CommandError
where
    T: Into<EventError>,
{
    fn from(value: T) -> Self {
        Self::EventError(value.into())
    }
}

impl From<DatabaseError> for CommandError {
    fn from(value: DatabaseError) -> Self {
        Self::DatabaseError(value)
    }
}

pub struct CommandDispatcher {
    /// (alias, primary).
    aliases: HashMap<&'static str, &'static str>,
    commands: HashMap<&'static str, Box<dyn for<'a> CommandExecuteFn<'a>>>,
}

impl CommandContext<'_> {
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
}

impl CommandDispatcher {
    pub fn new() -> Self {
        Self {
            aliases: HashMap::new(),
            commands: HashMap::new(),
        }
    }

    pub fn register<const N: usize, F: for<'a> CommandExecuteFn<'a>>(
        &mut self,
        primary_name: &'static str,
        aliases: [&'static str; N],
        execute_fn: F,
    ) {
        if self
            .commands
            .insert(primary_name, Box::new(execute_fn))
            .is_some()
        {
            debug_panic!("Command \"{primary_name}\" registered twice.");
        }
        for alias in aliases {
            if self.aliases.insert(alias, primary_name).is_some() {
                debug_panic!("Alias \"{alias}\" registered twice.");
            }
        }
    }

    #[expect(clippy::too_many_lines)]
    pub async fn execute(&self, ctx: CommandContext<'_>, input: &str) -> Result<(), EventError> {
        let (command_name, args) = input.split_once(' ').unwrap_or((input, ""));
        let command_name = if let Some(primary_name) = self.aliases.get(command_name) {
            primary_name
        } else {
            command_name
        };
        let command_configuration = match ctx
            .db
            .get_guild_command_configuration(ctx.guild_id, command_name)
            .await
        {
            Ok(configuration) => match configuration {
                Some(configuration) => configuration,
                None => {
                    if let Some(default_configuration) =
                        ctx.default_command_configuration.get(command_name)
                    {
                        GuildCommandConfiguration {
                            guild_id: ctx.guild_id,
                            command_name: String::from(command_name),
                            roles: Vec::new(),
                            permissions: *default_configuration,
                            channels: Vec::new(),
                        }
                    } else {
                        // There is no configuration for this command meaning it probably doesn't exist
                        return Ok(());
                    }
                }
            },
            Err(e) => {
                tracing::error!(
                    "Database error getting command configuration for guild {}: {}",
                    ctx.guild_id,
                    e
                );
                return Ok(());
            }
        };

        let guild_member = ctx
            .guild_id
            .get_member(ctx.ctx, ctx.message.author.id)
            .await?;

        if !command_configuration
            .channels
            .contains(&ctx.message.channel_id)
            && !guild_member
                .has_permissions(ctx.ctx, command_configuration.permissions)
                .await?
            && !{
                let mut has_role = false;
                for role in command_configuration.roles {
                    if guild_member.roles.contains(&role) {
                        has_role = true;
                        break;
                    }
                }
                has_role
            }
        {
            ctx.message.reply(ctx.ctx, embed_default_footer!(
                ctx,
                {
                    description: "You do not have the permissions required to execute this command.",
                    color: 0xff0000,
                }
            )).await?;
            return Ok(());
        }

        if let Some(execute_fn) = self.commands.get(command_name) {
            let ctx_backup = ctx.ctx;
            let bot_name_backup = ctx.bot_name;
            let message_backup = ctx.message;
            match execute_fn.call(ctx, args.trim_start()).await {
                Err(CommandError::DatabaseError(e)) => {
                    tracing::error!("Database error: {e}");
                    message_backup
                        .reply(
                            ctx_backup,
                            embed_default_footer_raw!(
                                bot_name_backup,
                                {
                                    description: "Database error.",
                                    color: 0xff0000,
                                }
                            ),
                        )
                        .await?;
                }
                Err(CommandError::EventError(e)) => {
                    #[expect(
                        irrefutable_let_patterns,
                        reason = "There might be other EventErrorKinds added."
                    )]
                    if let EventErrorKind::ClientError(e) = &e.kind
                        && let ClientErrorKind::HttpForbidden(res) = e.kind()
                        && res.code == ApiErrorCode::MissingPermissions
                    {
                        let _ = message_backup.reply(ctx_backup, embed_default_footer_raw!(
                            bot_name_backup,
                            {
                                description: "I did not have the required permissions for that.",
                                color: 0xff0000
                            }
                        )).await;
                    }
                    return Err(e);
                }
                Ok(()) | Err(CommandError::Ignore) => {}
            }
        }
        Ok(())
    }
}

pub fn register_commands(dispatcher: &mut CommandDispatcher) {
    dispatcher.register("ping", [], misc::ping);
    dispatcher.register("add-prefix", [], guild_settings::add_prefix);
    dispatcher.register(
        "remove-prefix",
        ["delete-prefix"],
        guild_settings::remove_prefix,
    );
    dispatcher.register(
        "list-prefixes",
        ["prefixes", "listprefixes", "prefixlist"],
        guild_settings::list_prefixes,
    );
    dispatcher.register("warn", ["add-warn", "create-warn"], moderation::warn);
    dispatcher.register(
        "unwarn",
        ["remove-warn", "delwarn", "rmwarn"],
        moderation::unwarn,
    );
    dispatcher.register(
        "list-cases",
        ["cases", "caselist", "listcases"],
        cases::list_cases,
    );
    dispatcher.register("case-info", ["case"], cases::case_info);
    dispatcher.register("mute", ["timeout"], moderation::mute);
    dispatcher.register("unmute", ["untimeout"], moderation::unmute);
}
