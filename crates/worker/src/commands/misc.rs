use crate::commands::{Arg, Ctx, args::empty};
use std::time::{Duration, SystemTime};

pub fn ping() -> Arg {
    empty().executes(async |ctx: Ctx| {
        ctx.reply_embed(
            Some("Pong!"),
            format!(
                "> **Uptime:** {}\n> **Version:** {}+{}\n> {}",
                pretty_duration::pretty_duration(
                    &SystemTime::now()
                        .duration_since(ctx.started_at)
                        .unwrap_or(Duration::ZERO),
                    crate::PRETTY_DURATION_OPTIONS,
                ),
                crate::VERSION,
                crate::GIT_HASH,
                if cfg!(debug_assertions) {
                    "*This is a debug build.*"
                } else {
                    "*This is a release build.*"
                }
            ),
            None,
        )
        .await?;
        Ok(())
    })
}
