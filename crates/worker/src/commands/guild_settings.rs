use async_brigadier::arg::{greedy_string, optional};
use fluxer_neptunium::{
    exts::{ChannelExt, GuildExt, MessageExt},
    http::endpoints::{
        ExecuteEndpointRequestError,
        webhooks::{DeleteWebhookWithToken, GetWebhookWithToken},
    },
    model::id::{
        Id,
        marker::{ChannelMarker, WebhookMarker},
    },
};

use crate::{
    commands::{
        Arg, Ctx,
        args::{channel_id_or_mention_or_link, empty, webhook_url},
    },
    logging::ModLogEntry,
    util::{MaybeExpired, confirmation::confirmation},
};

pub fn add_prefix() -> Arg {
    async fn add_prefix(mut ctx: Ctx) -> anyhow::Result<()> {
        let prefix: String = ctx.take_argument("prefix");

        if prefix.len() > ctx.max_command_prefix_len {
            ctx.reply_embed(
                None,
                format!(
                    "The prefix must not be longer than {} characters.",
                    ctx.max_command_prefix_len
                ),
                Some(0xff0000),
            )
            .await?;
            return Ok(());
        }
        if ctx.guild_command_prefixes.contains(&prefix) {
            ctx.reply_embed(
                None,
                "That prefix already exists in this community.",
                Some(0xff0000),
            )
            .await?;
            return Ok(());
        }
        if ctx.guild_command_prefixes.len() >= ctx.max_command_prefixes {
            ctx.reply_embed(None,
                    format!("This community already has the maximum number of prefixes ({}). Remove a prefix first before adding another one.", ctx.max_command_prefixes),
                    Some(0xff0000),
                ).await?;
            return Ok(());
        }

        ctx.db
            .add_guild_command_prefix_upsert(ctx.guild_id, &prefix)
            .await?;

        ctx.reply_embed(
            None,
            format!("Added the command prefix `{prefix}`"),
            Some(0xffffff),
        )
        .await?;

        Ok(())
    }

    empty().then(greedy_string("prefix").executes(add_prefix))
}

pub fn remove_prefix() -> Arg {
    empty()
        .then(greedy_string("prefix").executes(async |mut ctx: Ctx| {
            let prefix: String = ctx.take_argument("prefix");

            let new_prefixes = ctx
                .db
                .remove_guild_command_prefix_upsert(ctx.guild_id, &prefix)
                .await?;

            ctx.reply_embed(
                None,
                format!(
                    "Removed `{}` as a prefix.{}",
                    prefix,
                    if new_prefixes.is_empty() {
                        "\n*Note: You have removed all prefixes. To run a command, use my mention (@ping) as the prefix.*"
                    } else {
                        ""
                    }
                ),
                None,
            )
            .await?;

            Ok(())
        }))
}

pub fn list_prefixes() -> Arg {
    empty().executes(async |ctx: Ctx| {
        let prefixes = ctx.db.get_guild_command_prefixes(ctx.guild_id).await?;

        ctx.reply_embed(
            None,
            if prefixes.is_empty() {
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
            None,
        )
        .await?;

        Ok(())
    })
}

pub fn set_modlog_webhook() -> Arg {
    empty().then(
        optional(webhook_url("webhook_url")).executes(async |mut ctx: Ctx| {
            let maybe_webhook_data: Option<(Id<WebhookMarker>, String)> =
                ctx.try_take_argument_downcast("webhook_url");

            let Some((webhook_id, webhook_token)) = maybe_webhook_data else {
                return clear_modlog_webhook_inner(ctx).await;
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
                ctx.reply_embed(
                    None,
                    "The webhook is invalid or could not be validated.",
                    Some(0xff0000),
                )
                .await?;
                return Ok(());
            }

            ctx.db
                .set_guild_modlog_webhook_upsert(
                    ctx.guild_id,
                    Some((webhook_id, &webhook_token, None)),
                )
                .await?;

            ctx.reply_embed(
                None,
                "Set the modlog webhook URL for this community.",
                Some(0xffffff),
            )
            .await?;

            Ok(())
        }),
    )
}

async fn clear_modlog_webhook_inner(ctx: Ctx) -> anyhow::Result<()> {
    ctx.db
        .set_guild_modlog_webhook_upsert(ctx.guild_id, None)
        .await?;

    ctx.reply_embed(None, "Cleared the modlog webhook URL.", Some(0xffffff))
        .await?;
    Ok(())
}

pub fn clear_modlog_webhook() -> Arg {
    empty().executes(clear_modlog_webhook_inner)
}

#[expect(clippy::too_many_lines)]
pub fn modlog_channel() -> Arg {
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
                        #[expect(clippy::disallowed_macros)]
                        let confirmation_message = ctx.message.reply(&ctx.ctx, fluxer_neptunium::create_embed!(
                                description: format!(
                                    "There is already a webhook named \"{}\" in <#{}> created by me, should it be reused?\n-# The ID of that webhook is `{}`.",
                                    webhook.name,
                                    channel_id,
                                    webhook.id
                                ),
                                color: 0xffffff,
                            )).await?;
                        match confirmation(&ctx, confirmation_message, ctx.message.author.id)
                            .await?
                        {
                            MaybeExpired::Expired => return Ok(()),
                            MaybeExpired::NotExpired(false) => break,
                            MaybeExpired::NotExpired(true) => {
                                ctx.db
                                    .set_guild_modlog_webhook_upsert(
                                        ctx.guild_id,
                                        Some((webhook.id, &webhook.token, Some(channel_id))),
                                    )
                                    .await?;
                                ctx.reply_embed(
                                    None,
                                    "Reused existing webhook for modlogs.",
                                    Some(0xffffff),
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

                ctx.reply_embed(
                        None,
                        format!(
                            "Added a webhook to <#{channel_id}> for modlogs. New modlogs will be sent there."
                        ),
                        Some(0xffffff)
                    ).await?;

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

        ctx.reply_embed(
            None,
            "It seems like that channel doesn't exist in this community.",
            Some(0xff0000),
        )
        .await?;

        Ok(())
    }

    empty().then(channel_id_or_mention_or_link("channel").executes(modlog_channel))
}
