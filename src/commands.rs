use std::{collections::HashMap, pin::Pin, sync::Arc, time::SystemTime};

use fluxer_neptunium::{
    cache::CachedMessage,
    create_embed,
    events::{EventError, context::Context},
    exts::{GuildExt, GuildMemberExt, MessageExt},
    model::{
        guild::permissions::Permissions,
        id::{Id, marker::GuildMarker},
    },
};

use crate::db::{DatabaseManager, schema::GuildCommandConfiguration};

mod guild_settings;
mod misc;

pub trait CommandExecuteFn<'a>: Send + Sync + 'static {
    fn call(
        &self,
        ctx: CommandContext<'a>,
        args: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), EventError>> + Send + 'a>>;
}

impl<'a, F, Fut> CommandExecuteFn<'a> for F
where
    F: Fn(CommandContext<'a>, &'a str) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<(), EventError>> + Send + 'a,
{
    fn call(
        &self,
        ctx: CommandContext<'a>,
        args: &'a str,
    ) -> Pin<Box<dyn Future<Output = Result<(), EventError>> + Send + 'a>> {
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
}

pub struct CommandDispatcher {
    commands: HashMap<&'static str, Arc<dyn for<'a> CommandExecuteFn<'a>>>,
}

impl CommandDispatcher {
    pub fn new() -> Self {
        Self {
            commands: HashMap::new(),
        }
    }

    pub fn register<const N: usize, F: for<'a> CommandExecuteFn<'a>>(
        &mut self,
        command_names: [&'static str; N],
        execute_fn: F,
    ) {
        let execute_fn: Arc<dyn for<'a> CommandExecuteFn<'a>> = Arc::new(execute_fn);
        for name in command_names {
            self.commands.insert(name, Arc::clone(&execute_fn));
        }
    }

    pub async fn execute(&self, ctx: CommandContext<'_>, input: &str) -> Result<(), EventError> {
        let (command_name, args) = input.split_once(' ').unwrap_or((input, ""));
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
            ctx.message.reply(ctx.ctx, create_embed!(
                description: "You do not have the permissions required to execute this command.",
                color: 0xff0000,
            )).await?;
            return Ok(());
        }

        if let Some(execute_fn) = self.commands.get(command_name) {
            execute_fn.call(ctx, args).await
        } else {
            Ok(())
        }
    }
}

pub fn register_commands(dispatcher: &mut CommandDispatcher) {
    dispatcher.register(["ping"], misc::ping);
    dispatcher.register(["set-prefix"], guild_settings::set_prefix);
}
