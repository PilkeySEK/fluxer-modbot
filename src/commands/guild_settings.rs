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
                    color: 0xff0000,
                }
            ),
        )
        .await?;

    Ok(())
}
