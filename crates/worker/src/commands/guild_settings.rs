use fluxer_neptunium::{
    exts::{ChannelExt, GuildExt, MessageExt},
    http::endpoints::{
        ExecuteEndpointRequestError,
        webhooks::{DeleteWebhookWithToken, GetWebhookWithToken},
    },
};

use crate::{
    commands::{CommandContext, CommandError},
    logging::ModLogEntry,
    macros::embed_default_footer,
    util::{
        MaybeExpired, confirmation::confirmation, parse_channel_mention_or_id_or_link,
        parse_webhook_url,
    },
};

pub async fn add_prefix(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let prefix = args.trim();
    if prefix.is_empty() {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "Provide a prefix.",
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
        return Ok(());
    }
    if prefix.len() > ctx.max_command_prefix_len {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: format!("The prefix must not be longer than {} characters.", ctx.max_command_prefix_len),
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
        return Ok(());
    }
    if ctx.guild_command_prefixes.contains(&prefix) {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "That prefix already exists in this community.",
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
        return Ok(());
    }
    if ctx.guild_command_prefixes.len() >= ctx.max_command_prefixes {
        ctx.message.reply(ctx.ctx, embed_default_footer!(
            ctx,
            {
                description: format!("This community already has the maximum number of prefixes ({}). Remove a prefix first before adding another one.", ctx.max_command_prefixes),
                color: 0xff0000,
            }
        )).await?;
        return Ok(());
    }

    ctx.db
        .add_guild_command_prefix_upsert(ctx.guild_id, prefix)
        .await?;

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: format!("Added the command prefix `{}`", prefix),
                    color: 0xffffff,
                }
            ),
        )
        .await?;

    Ok(())
}

pub async fn remove_prefix(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let prefix = args.trim();
    if prefix.is_empty() {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "Provide a prefix to remove.",
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
        return Ok(());
    }
    let new_prefixes = ctx
        .db
        .remove_guild_command_prefix_upsert(ctx.guild_id, prefix)
        .await?;

    ctx.message.reply(ctx.ctx, embed_default_footer!(
        ctx,
        {
            description: format!("Removed `{}` as a prefix.{}", prefix, if new_prefixes.is_empty() {
                "\n*Note: You have removed all prefixes. To run a command, use my mention (@ping) as the prefix.*"
            } else {
                ""
            }),
        }
    )).await?;

    Ok(())
}

pub async fn list_prefixes(ctx: CommandContext<'_>, _args: &str) -> Result<(), CommandError> {
    let prefixes = ctx.db.get_guild_command_prefixes(ctx.guild_id).await?;

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
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
                    }
                }
            ),
        )
        .await?;

    Ok(())
}

pub async fn set_modlog_webhook(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let webhook_url = args.trim();
    if webhook_url.is_empty() {
        return clear_modlog_webhook(ctx, "").await;
    }
    let Some((webhook_id, webhook_token)) = parse_webhook_url(webhook_url) else {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "That is not a valid webhook URL.",
                        color: 0xff0000
                    }
                ),
            )
            .await?;
        return Ok(());
    };

    let webhook_result = ctx
        .ctx
        .get_http_client()
        .execute(GetWebhookWithToken {
            webhook_id,
            token: webhook_token.to_owned().into(),
        })
        .await;

    if webhook_result.is_err() {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "The webhook is invalid or could not be validated.",
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
        return Ok(());
    }

    ctx.db
        .set_guild_modlog_webhook_upsert(ctx.guild_id, Some((webhook_id, webhook_token, None)))
        .await?;

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: "Set the modlog webhook URL for this community.",
                    color: 0xffffff,
                }
            ),
        )
        .await?;

    Ok(())
}

pub async fn clear_modlog_webhook(
    ctx: CommandContext<'_>,
    _args: &str,
) -> Result<(), CommandError> {
    ctx.db
        .set_guild_modlog_webhook_upsert(ctx.guild_id, None)
        .await?;

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: "Cleared the modlog webhook URL.",
                    color: 0xffffff,
                }
            ),
        )
        .await?;
    Ok(())
}

#[expect(clippy::too_many_lines)]
pub async fn modlog_channel(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (channel_mention_or_link, _rest) = args.split_once(' ').unwrap_or((args, ""));
    let channel_id = if channel_mention_or_link.is_empty() {
        ctx.message.channel_id
    } else {
        let Some((_, channel_id)) = parse_channel_mention_or_id_or_link(channel_mention_or_link)
        else {
            ctx.message
                .reply(
                    ctx.ctx,
                    embed_default_footer!(
                        ctx,
                        {
                            description: "Could not parse the channel link provided.",
                            color: 0xff0000,
                        }
                    ),
                )
                .await?;
            return Ok(());
        };
        channel_id
    };

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

    let guild_channels = ctx.guild_id.list_channels(ctx.ctx).await?;

    for channel in guild_channels {
        if channel.id == channel_id {
            // This should be the same permission as the creating a webhook so it will return the same
            // permission missing error if they are missing.
            let webhooks = channel.list_webhooks(ctx.ctx).await?;

            for webhook in webhooks {
                if webhook.creator.id == ctx.bot_id {
                    let confirmation_message = ctx.message.reply(ctx.ctx, embed_default_footer!(
                        ctx,
                        {
                            description: format!(
                                "There is already a webhook named \"{}\" in <#{}> created by me, should it be reused?\n-# The ID of that webhook is `{}`.",
                                webhook.name,
                                channel_id,
                                webhook.id
                            ),
                            color: 0xffffff,
                        }
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
                            ctx.message
                                .reply(
                                    ctx.ctx,
                                    embed_default_footer!(
                                        ctx,
                                        {
                                            description: "Reused existing webhook for modlogs.",
                                            color: 0xffffff,
                                        }
                                    ),
                                )
                                .await?;

                            ctx.logger
                                .create_modlog_entry(
                                    ctx.db,
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
                    ctx.ctx,
                    ctx.bot_name.to_owned(),
                    ctx.webhook_avatar_b64.map(String::from),
                )
                .await?;

            ctx.db
                .set_guild_modlog_webhook_upsert(
                    ctx.guild_id,
                    Some((webhook.id, &webhook.token, Some(channel_id))),
                )
                .await?;

            ctx.message
                .reply(
                    ctx.ctx,
                    embed_default_footer!(
                        ctx,
                        {
                            description: format!("Added a webhook to <#{channel_id}> for modlogs. New modlogs will be sent there."),
                            color: 0xffffff,
                        }
                    ),
                )
                .await?;

            ctx.logger
                .create_modlog_entry(
                    ctx.db,
                    ctx.guild_id,
                    ModLogEntry::ModLogChannelSet {
                        responsible: ctx.message.author.id,
                    },
                )
                .await;

            return Ok(());
        }
    }

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: "It seems like that channel doesn't exist in this community.",
                    color: 0xff0000,
                }
            ),
        )
        .await?;

    Ok(())
}
