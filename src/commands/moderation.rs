use std::time::Duration;

use chrono::{TimeDelta, Utc};
use fluxer_neptunium::{
    exts::MessageExt,
    model::time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
};

use crate::{
    commands::{CommandContext, CommandError},
    db::schema::{CreateGuildModerationCaseData, ModerationKind},
    macros::{embed_default_footer, get_user_arg},
    util::parse_duration,
};

pub async fn warn(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (target_id, rest) = get_user_arg!(ctx, args, required);

    let (maybe_duration, reason) = rest.split_once(' ').unwrap_or((rest, ""));

    let now = Utc::now();

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
            (
                Some((
                    if let Some(time) =
                        now.checked_add_signed(match TimeDelta::from_std(std_duration) {
                            Ok(delta) => delta,
                            Err(e) => {
                                tracing::error!("Failed to convert Duration to TimeDelta: {e}");
                                return Ok(());
                            }
                        })
                    {
                        time
                    } else {
                        tracing::error!(?std_duration, %now, "Duration add overflow!");
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

pub async fn unwarn(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (target_id, rest) = get_user_arg!(ctx, args, required);

    let rest = rest.trim();
    let reason = if rest.is_empty() { None } else { Some(rest) };

    let case_id = ctx
        .db
        .close_latest_guild_moderation_case_of_kind(
            ctx.guild_id,
            target_id,
            ModerationKind::Warn,
            reason,
            Some(ctx.message.author.id),
        )
        .await?;

    if let Some(case_id) = case_id {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: format!("Case `{case_id}` closed."),
                        color: 0xffffff,
                    }
                ),
            )
            .await?;
    } else {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "The user does not have any open warn cases.",
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
    }

    Ok(())
}
