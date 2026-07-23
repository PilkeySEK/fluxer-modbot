use std::time::{Duration, SystemTime};

use async_brigadier::arg::literal;
use fluxer_neptunium::create_embed;

use crate::commands::{Ctx, Dispatcher};

pub fn register(dispatcher: &mut Dispatcher) {
    dispatcher.register(literal("ping").executes(async |ctx: Ctx| {
        ctx.reply(create_embed!(
            title: "Pong!",
            description: format!(
                "> **Uptime:** {}\n> **Version:** {}+{}\n> {}",
                pretty_duration::pretty_duration(
                    &SystemTime::now().duration_since(ctx.started_at).unwrap_or(Duration::ZERO),
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
        ))
        .await?;
        Ok(())
    }));
}
