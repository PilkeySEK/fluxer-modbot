use std::time::Duration;

use fluxer_neptunium::{
    exts::MessageExt,
    model::time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
};
use time::OffsetDateTime;

use crate::{
    commands::{CommandContext, CommandError},
    db::schema::{CreateGuildModerationCaseData, ModerationKind},
    macros::{embed_default_footer, try_parse_mention_or_id},
    util::{parse_duration, parse_mention_or_id, try_db},
};

pub async fn warn(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (user_mention_or_id_str, rest) = args.split_once(' ').unwrap_or((args, ""));

    let target_id = try_parse_mention_or_id!(ctx, user_mention_or_id_str);

    let (maybe_duration, reason) = rest.split_once(' ').unwrap_or((rest, ""));

    let now = OffsetDateTime::now_utc();

    let (expires_at_and_duration, rest) = match parse_duration(maybe_duration) {
        Some(std_duration) => {
            if std_duration > Duration::from_hours(24 * 356) {
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
            let duration = match std_duration.try_into() {
                Ok(duration) => duration,
                Err(e) => {
                    tracing::error!("{e}");
                    return Ok(());
                }
            };
            (
                Some((
                    if let Some(time) = now.checked_add(duration) {
                        time
                    } else {
                        tracing::error!(%duration, %now, "Duration add overflow!");
                        return Ok(());
                    },
                    std_duration,
                )),
                reason,
            )
        }
        None => (None, rest),
    };

    let rest = rest.trim();
    let reason = if rest.is_empty() { None } else { Some(rest) };

    try_db(
        &ctx,
        ctx.db
            .create_moderation_case(CreateGuildModerationCaseData {
                guild_id: ctx.guild_id,
                target_id,
                moderator_id: Some(ctx.message.author.id),
                moderation_kind: ModerationKind::Warn,
                expiry: match expires_at_and_duration {
                    Some((expires_at, duration)) => {
                        if let Ok(value) = i64::try_from(duration.as_secs()) {
                            Some((expires_at, value))
                        } else {
                            tracing::error!(?duration, "Failed to convert seconds to i64.");
                            return Ok(());
                        }
                    }
                    None => None,
                },
                reason,
                created_at: now,
            })
            .await,
    )
    .await?;

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: format!("**Warned** <@{target_id}>{}\n{}", if let Some((expires_at, _)) = expires_at_and_duration {
                        format!(
                            " until {}.",
                            Timestamp::<UnixMillis>::from(expires_at).time_string(TimestampDisplayType::VerboseDateWithDayOfWeekAndShortTime),
                        )
                    } else { ".".to_owned() },
                    if let Some(reason) = reason {
                        format!("**Reason:** {reason}")
                    } else {
                        "*No reason.*".to_owned()
                    }),
                    color: 0xffffff,
                }
            ),
        )
        .await?;

    Ok(())
}
