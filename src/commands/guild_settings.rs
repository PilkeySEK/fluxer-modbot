use fluxer_neptunium::{exts::MessageExt, http::endpoints::webhooks::GetWebhookWithToken};

use crate::{
    commands::{CommandContext, CommandError},
    macros::embed_default_footer,
    util::parse_webhook_url,
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
            token: webhook_token.to_string().into(),
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
        .set_guild_modlog_webhook_upsert(ctx.guild_id, Some(webhook_url))
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
