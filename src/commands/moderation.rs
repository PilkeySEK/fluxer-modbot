use std::time::Duration;

use fluxer_neptunium::{events::EventError, exts::MessageExt};
use time::OffsetDateTime;

use crate::{
    commands::CommandContext,
    db::schema::{CreateModerationCaseData, ModerationKind},
    embed_default_footer, try_db,
    util::parse_mention_or_id,
};

pub async fn warn(ctx: CommandContext<'_>, args: &str) -> Result<(), EventError> {
    let (user_mention_or_id_str, rest) = args.split_once(' ').unwrap_or((args, ""));

    let Some(target_id) = parse_mention_or_id(user_mention_or_id_str) else {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "Provide a target user ID or mention.",
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
        return Ok(());
    };

    let (maybe_duration, reason) = rest.split_once(' ').unwrap_or((rest, ""));

    let (expires_at, rest) = match parse_duration::parse(maybe_duration) {
        Ok(duration) => {
            if duration > Duration::from_hours(24 * 356) {
                ctx.message
                    .reply(
                        ctx.ctx,
                        embed_default_footer!(
                            ctx,
                            {
                                description: "The duration can not be higher than 1 year.",
                                color: 0xff0000,
                            }
                        ),
                    )
                    .await?;
                return Ok(());
            }
            let duration = match duration.try_into() {
                Ok(duration) => duration,
                Err(e) => {
                    tracing::error!("{e}");
                    return Ok(());
                }
            };
            let now = OffsetDateTime::now_utc();
            (
                Some(if let Some(time) = now.checked_add(duration) {
                    time
                } else {
                    tracing::error!(%duration, %now, "Duration add overflow!");
                    return Ok(());
                }),
                reason,
            )
        }
        Err(_) => (None, rest),
    };

    let rest = rest.trim();
    let reason = if rest.is_empty() { None } else { Some(rest) };

    try_db!(
        ctx,
        ctx.db.create_moderation_case(CreateModerationCaseData {
            guild_id: ctx.guild_id,
            target_id,
            moderator_id: Some(ctx.message.author.id),
            moderation_kind: ModerationKind::Warn,
            expires_at,
            reason,
        })
    );

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: format!("Warned <@{target_id}>."),
                    color: 0xffffff,
                }
            ),
        )
        .await?;

    Ok(())
}
