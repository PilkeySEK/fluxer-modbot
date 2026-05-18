use fluxer_neptunium::{events::EventError, exts::MessageExt};

use crate::{commands::CommandContext, embed_default_footer, try_db};

pub async fn add_prefix(ctx: CommandContext<'_>, args: &str) -> Result<(), EventError> {
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

    try_db!(
        ctx,
        ctx.db
            .add_guild_command_prefix_upsert(ctx.guild_id, prefix, ctx.default_command_prefix)
    );

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

pub async fn remove_prefix(ctx: CommandContext<'_>, args: &str) -> Result<(), EventError> {
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
    let new_prefixes = try_db!(
        ctx,
        ctx.db
            .remove_guild_command_prefix_upsert(ctx.guild_id, prefix, ctx.default_command_prefix)
    );

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

pub async fn list_prefixes(ctx: CommandContext<'_>, _args: &str) -> Result<(), EventError> {
    let prefixes = try_db!(ctx, ctx.db.get_guild_command_prefixes(ctx.guild_id))
        .unwrap_or_else(|| vec![ctx.default_command_prefix.to_owned()]);

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
                                .into_iter()
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
