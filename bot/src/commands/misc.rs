use std::time::{Duration, SystemTime};

use fluxer_neptunium::exts::MessageExt;
use pretty_duration::pretty_duration;

use crate::{
    commands::{CommandContext, CommandError},
    macros::embed_default_footer,
};

pub async fn ping(ctx: CommandContext<'_>, _args: &str) -> Result<(), CommandError> {
    ctx.message.reply(
        ctx.ctx,
        embed_default_footer!(
            ctx,
            {
                title: "Pong!",
                description: format!(
                    "> **Uptime:** {}\n> **Version:** {}+{}\n> {}",
                    pretty_duration(
                        &SystemTime::now().duration_since(*ctx.started_at).unwrap_or(Duration::ZERO),
                        crate::PRETTY_DURATION_OPTIONS,
                    ),
                    crate::VERSION,
                    crate::GIT_HASH,
                    if cfg!(debug_assertions) {
                        "*This is a debug build.*"
                    } else {
                        "*This is a release build.*"
                    }
                )
            }
        )
    ).await?;
    Ok(())
}
