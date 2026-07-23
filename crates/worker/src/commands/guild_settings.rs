use async_brigadier::arg::{greedy_string, literal, multi_literal, optional};
use fluxer_neptunium::{
    create_embed,
    exts::{ChannelExt, GuildExt, MessageExt},
    http::endpoints::{
        ExecuteEndpointRequestError,
        webhooks::{DeleteWebhookWithToken, GetWebhookWithToken},
    },
    model::{
        guild::permissions::Permissions,
        id::{
            Id,
            marker::{ChannelMarker, WebhookMarker},
        },
    },
};

use crate::{
    commands::{
        CommandContext, Ctx, Dispatcher,
        args::{channel_id_or_mention_or_link, webhook_url},
    },
    logging::ModLogEntry,
    util::{MaybeExpired, confirmation::confirmation, has_permission},
};

async fn add_prefix(mut ctx: Ctx) -> anyhow::Result<()> {
    let prefix: String = ctx.take_argument("prefix");

    if prefix.len() > ctx.max_command_prefix_len {
        ctx.reply(create_embed!(
            description: format!("The prefix must not be longer than {} characters.", ctx.max_command_prefix_len),
            color: 0xff0000,
        )).await?;
        return Ok(());
    }
    if ctx.guild_command_prefixes.contains(&prefix) {
        ctx.reply(create_embed!(
            description: "That prefix already exists in this community.",
            color: 0xff0000,
        ))
        .await?;
        return Ok(());
    }
    if ctx.guild_command_prefixes.len() >= ctx.max_command_prefixes {
        ctx.reply(create_embed!(
            description: format!("This community already has the maximum number of prefixes ({}). Remove a prefix first before adding another one.", ctx.max_command_prefixes),
            color: 0xff0000,
        )).await?;
        return Ok(());
    }

    ctx.db
        .add_guild_command_prefix_upsert(ctx.guild_id, &prefix)
        .await?;

    ctx.reply(create_embed!(
        description: format!("Added the command prefix `{}`", prefix),
        color: 0xffffff,
    ))
    .await?;

    Ok(())
}

async fn remove_prefix(mut ctx: Ctx) -> anyhow::Result<()> {
    let prefix: String = ctx.take_argument("prefix");

    let new_prefixes = ctx
        .db
        .remove_guild_command_prefix_upsert(ctx.guild_id, &prefix)
        .await?;

    ctx.reply(create_embed!(
        description: format!("Removed `{}` as a prefix.{}", prefix, if new_prefixes.is_empty() {
            "\n*Note: You have removed all prefixes. To run a command, use my mention (@ping) as the prefix.*"
        } else {
            ""
        }),
    )).await?;

    Ok(())
}

async fn list_prefixes(ctx: Ctx) -> anyhow::Result<()> {
    let prefixes = ctx.db.get_guild_command_prefixes(ctx.guild_id).await?;

    ctx.reply(create_embed!(
        description: if prefixes.is_empty() {
            "*There are no prefixes set.*".to_owned()
        } else {
            format!(
                "The prefixes for this community are:\n{}",
                prefixes
                    .iter()
                    .map(|prefix| format!("- `{prefix}`"))
                    .collect::<Vec<String>>()
                    .join("\n")
            )
        },
    ))
    .await?;

    Ok(())
}

async fn set_modlog_webhook(mut ctx: Ctx) -> anyhow::Result<()> {
    let maybe_webhook_data: Option<(Id<WebhookMarker>, String)> =
        ctx.try_take_argument_downcast("webhook_url");

    let Some((webhook_id, webhook_token)) = maybe_webhook_data else {
        return clear_modlog_webhook(ctx).await;
    };

    let webhook_result = ctx
        .ctx
        .get_http_client()
        .execute(GetWebhookWithToken {
            webhook_id,
            token: webhook_token.clone().into(),
        })
        .await;

    if webhook_result.is_err() {
        ctx.reply(create_embed!(
            description: "The webhook is invalid or could not be validated.",
            color: 0xff0000,
        ))
        .await?;
        return Ok(());
    }

    ctx.db
        .set_guild_modlog_webhook_upsert(ctx.guild_id, Some((webhook_id, &webhook_token, None)))
        .await?;

    ctx.reply(create_embed!(
        description: "Set the modlog webhook URL for this community.",
        color: 0xffffff,
    ))
    .await?;

    Ok(())
}

async fn clear_modlog_webhook(ctx: Ctx) -> anyhow::Result<()> {
    ctx.db
        .set_guild_modlog_webhook_upsert(ctx.guild_id, None)
        .await?;

    ctx.reply(create_embed!(
        description: "Cleared the modlog webhook URL.",
        color: 0xffffff,
    ))
    .await?;
    Ok(())
}

#[expect(clippy::too_many_lines)]
async fn modlog_channel(mut ctx: Ctx) -> anyhow::Result<()> {
    let channel_id: Id<ChannelMarker> = ctx.take_argument("channel");

    // Delete the existing webhook in the guild, if it exists and wasn't manually set
    if let Some((webhook_id, token, webhook_channel_id)) =
        ctx.db.get_guild_modlog_webhook(ctx.guild_id).await?
        && {
            if let Some(webhook_channel_id) = webhook_channel_id {
                webhook_channel_id != channel_id
            } else {
                false
            }
        }
        && let Err(e) = ctx
            .ctx
            .get_http_client()
            .execute(DeleteWebhookWithToken {
                webhook_id,
                token: token.into(),
            })
            .await
    {
        match *e {
            // The old webhook doesn't exist anymore, probably
            ExecuteEndpointRequestError::Forbidden(_)
            | ExecuteEndpointRequestError::NotFound(_) => {}
            other => return Err(fluxer_neptunium::client::error::Error::from(other).into()),
        }
    }

    let guild_channels = ctx.guild_id.list_channels(&ctx.ctx).await?;

    for channel in guild_channels {
        if channel.id == channel_id {
            // This should be the same permission as the creating a webhook so it will return the same
            // permission missing error if they are missing.
            let webhooks = channel.list_webhooks(&ctx.ctx).await?;

            for webhook in webhooks {
                if webhook.creator.id == ctx.bot_id {
                    let confirmation_message = ctx.message.reply(&ctx.ctx, create_embed!(
                        description: format!(
                            "There is already a webhook named \"{}\" in <#{}> created by me, should it be reused?\n-# The ID of that webhook is `{}`.",
                            webhook.name,
                            channel_id,
                            webhook.id
                        ),
                        color: 0xffffff,
                    )).await?;
                    match confirmation(&ctx, confirmation_message, ctx.message.author.id).await? {
                        MaybeExpired::Expired => return Ok(()),
                        MaybeExpired::NotExpired(false) => break,
                        MaybeExpired::NotExpired(true) => {
                            ctx.db
                                .set_guild_modlog_webhook_upsert(
                                    ctx.guild_id,
                                    Some((webhook.id, &webhook.token, Some(channel_id))),
                                )
                                .await?;
                            ctx.reply(create_embed!(
                                description: "Reused existing webhook for modlogs.",
                                color: 0xffffff,
                            ))
                            .await?;

                            ctx.logger
                                .create_modlog_entry(
                                    &ctx.db,
                                    ctx.guild_id,
                                    ModLogEntry::ModLogChannelSet {
                                        responsible: ctx.message.author.id,
                                    },
                                )
                                .await;

                            return Ok(());
                        }
                    }
                }
            }

            let webhook = channel
                .create_webhook(
                    &ctx.ctx,
                    ctx.bot_name.clone(),
                    ctx.webhook_avatar_b64.clone(),
                )
                .await?;

            ctx.db
                .set_guild_modlog_webhook_upsert(
                    ctx.guild_id,
                    Some((webhook.id, &webhook.token, Some(channel_id))),
                )
                .await?;

            ctx.reply(
                    create_embed!(
                        description: format!("Added a webhook to <#{channel_id}> for modlogs. New modlogs will be sent there."),
                        color: 0xffffff,
                    ),
                )
                .await?;

            ctx.logger
                .create_modlog_entry(
                    &ctx.db,
                    ctx.guild_id,
                    ModLogEntry::ModLogChannelSet {
                        responsible: ctx.message.author.id,
                    },
                )
                .await;

            return Ok(());
        }
    }

    ctx.reply(create_embed!(
        description: "It seems like that channel doesn't exist in this community.",
        color: 0xff0000,
    ))
    .await?;

    Ok(())
}

pub fn register(dispatcher: &mut Dispatcher) {
    dispatcher.register(
        literal("add-prefix")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::MANAGE_GUILD)
            })
            .then(greedy_string("prefix").executes(add_prefix)),
    );
    dispatcher.register(
        literal("remove-prefix")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::MANAGE_GUILD)
            })
            .then(greedy_string("prefix").executes(remove_prefix)),
    );
    dispatcher.register(
        literal("prefixes")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::MANAGE_GUILD)
            })
            .executes(list_prefixes),
    );
    dispatcher.register(
        multi_literal(vec!["set-modlog-webhook", "modlog-webhook"])
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::MANAGE_GUILD)
            })
            .then(optional(webhook_url("webhook_url")).executes(set_modlog_webhook)),
    );
    dispatcher.register(
        literal("clear-modlog-webhook")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::MANAGE_GUILD)
            })
            .executes(clear_modlog_webhook),
    );
    dispatcher.register(
        multi_literal(vec![
            "modlog-channel",
            "set-modlog-channel",
            "modlogs-channel",
            "set-modlogs-channel",
        ])
        .requires(|ctx: &CommandContext| {
            has_permission(ctx.member_permissions, Permissions::MANAGE_GUILD)
        })
        .then(channel_id_or_mention_or_link("channel").executes(modlog_channel)),
    );
}
