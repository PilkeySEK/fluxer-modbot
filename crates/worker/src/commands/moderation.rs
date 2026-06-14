use std::time::Duration;

use api_types::db::ModerationKind;
use chrono::{TimeDelta, Utc};
use fluxer_neptunium::{
    cache::Cached,
    client::error::ClientErrorKind,
    exts::{GuildExt, MessageExt},
    model::{
        id::{Id, marker::UserMarker},
        time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
    },
};

use crate::{
    commands::{CommandContext, CommandError},
    db::schema::CreateGuildModerationCaseData,
    macros::{debug_panic, embed_default_footer, get_user_arg},
    util::{parse_duration, user_fetcher::fetch_and_add_users_to_db},
};

pub async fn warn(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (target_id, expiry, reason, now) = moderation_common(&ctx, args).await?;

    ctx.db
        .create_moderation_case(
            CreateGuildModerationCaseData {
                guild_id: ctx.guild_id,
                target_id,
                moderator_id: Some(ctx.message.author.id),
                moderation_kind: ModerationKind::Warn,
                expiry: match expiry {
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
            },
            fetch_and_add_users_to_db(&ctx, [target_id, ctx.message.author.id]),
        )
        .await?;

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: format!("**Warned** <@{target_id}>{}\n{}", if let Some((expires_at, _)) = expiry {
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

pub async fn mute(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (target_id, expiry, reason, now) = moderation_common_time_required(&ctx, args).await?;

    let member = ctx
        .guild_id
        .timeout_member_with_reason(
            ctx.ctx,
            target_id,
            expiry.0,
            format!(
                "Mute by {}#{}{}",
                ctx.message.author.username,
                ctx.message.author.discriminator,
                if let Some(reason) = reason {
                    format!(": {reason}")
                } else {
                    String::new()
                }
            ),
        )
        .await?;

    let case_id = ctx
        .db
        .create_moderation_case(
            CreateGuildModerationCaseData {
                guild_id: ctx.guild_id,
                target_id,
                moderator_id: Some(ctx.message.author.id),
                moderation_kind: ModerationKind::Mute,
                reason,
                expiry: Some((
                    expiry.0,
                    if let Ok(value) = i64::try_from(expiry.1.as_secs()) {
                        value
                    } else {
                        let duration = expiry.1;
                        tracing::error!(?duration, "Failed to convert seconds to i64.");
                        return Err(CommandError::Ignore);
                    },
                )),
                created_at: now,
            },
            fetch_and_add_users_to_db(&ctx, [target_id, ctx.message.author.id]),
        )
        .await?;
    let close_reason = format!("Mute updated by case {case_id}");
    let closed_cases = ctx
        .db
        .close_existing_cases_by_kind_and_user(
            ctx.guild_id,
            target_id,
            ModerationKind::Mute,
            Some(&close_reason),
            case_id,
        )
        .await?;

    if closed_cases.len() > 1 {
        debug_panic!("There should only be one open mute case at a time.");
    }

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: format!(
                        "Muted <@{}> until {}. (Case `{}`)",
                        member.id,
                        Timestamp::<UnixMillis>::from(expiry.0).time_string(TimestampDisplayType::VerboseDateWithShortTime),
                        case_id
                    ),
                    color: 0xffffff,
                }
            ),
        )
        .await?;

    Ok(())
}

#[expect(clippy::too_many_lines)]
pub async fn unmute(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (target_id, rest) = get_user_arg!(ctx, args, required);
    let reason = rest.trim();

    let mut member = match ctx.guild_id.get_member(ctx.ctx, target_id).await {
        Ok(member) => member,
        Err(e) => {
            if let ClientErrorKind::HttpNotFound(_) = e.kind() {
                ctx.message
                    .reply(
                        ctx.ctx,
                        embed_default_footer!(
                            ctx,
                            {
                                description: "That user is not a member of this community.",
                                color: 0xff0000
                            }
                        ),
                    )
                    .await?;
                return Ok(());
            }
            return Err(e.into());
        }
    };

    // Refresh the cached value
    Cached::refresh(&mut member);

    let mut member_is_timed_out = true;

    if let Some(communication_disabled_until) = member.communication_disabled_until
        && chrono::DateTime::from(communication_disabled_until) < Utc::now()
    {
        member_is_timed_out = false;
    } else if member.communication_disabled_until.is_none() {
        member_is_timed_out = false;
    }

    if !member_is_timed_out {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "The member is not timed out.",
                        color: 0xff0000
                    }
                ),
            )
            .await?;
        return Ok(());
    }

    let _member = ctx
        .guild_id
        .untimeout_member_with_reason(
            ctx.ctx,
            target_id,
            format!(
                "Unmuted by {}#{}{}",
                ctx.message.author.username,
                ctx.message.author.discriminator,
                if reason.is_empty() {
                    String::new()
                } else {
                    format!(": {reason}")
                }
            ),
        )
        .await?;

    let maybe_case = ctx
        .db
        .close_and_get_latest_open_moderation_case_by_kind_and_user(
            ctx.guild_id,
            target_id,
            ModerationKind::Mute,
            if reason.is_empty() {
                None
            } else {
                Some(reason)
            },
            Some(ctx.message.author.id),
        )
        .await?;

    if let Some(case_id) = maybe_case {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: format!("Unmuted <@{target_id}>, and closed case `{case_id}`"),
                        color: 0xffffff,
                    }
                ),
            )
            .await?;
    } else {
        ctx.message.reply(ctx.ctx, embed_default_footer!(
            ctx,
            {
                description: format!("Unmuted <@{target_id}>, but no case associated with the mute could be found."),
                color: 0xffffff,
            }
        )).await?;
    }

    Ok(())
}

pub async fn kick(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (target_id, rest) = get_user_arg!(ctx, args, required);

    if target_id == ctx.message.author.id {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "You can't kick yourself.",
                        color: 0xff0000
                    }
                ),
            )
            .await?;
        return Ok(());
    }

    let reason = rest.trim();
    let reason = if reason.is_empty() {
        None
    } else {
        Some(reason)
    };

    if let Err(e) = ctx.guild_id.kick_member(ctx.ctx, target_id).await {
        if let ClientErrorKind::HttpNotFound(_) = e.kind() {
            ctx.message.reply(ctx.ctx, embed_default_footer!(
                ctx,
                {
                    description: format!("It seems like <@{target_id}> is not a member of this community."),
                    color: 0xff0000
                }
            )).await?;
            return Ok(());
        }
        return Err(e.into());
    }

    let case_id = ctx
        .db
        .create_moderation_case(
            CreateGuildModerationCaseData {
                guild_id: ctx.guild_id,
                target_id,
                moderator_id: Some(ctx.message.author.id),
                moderation_kind: ModerationKind::Kick,
                reason,
                expiry: None,
                created_at: Utc::now(),
            },
            fetch_and_add_users_to_db(&ctx, [target_id, ctx.message.author.id]),
        )
        .await?;

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: format!("Kicked <@{target_id}>.\n**Case ID:** {case_id}"),
                    color: 0xffffff,
                }
            ),
        )
        .await?;

    Ok(())
}

async fn moderation_common_time_required<'a>(
    ctx: &CommandContext<'_>,
    args: &'a str,
) -> Result<
    (
        Id<UserMarker>,
        (chrono::DateTime<Utc>, Duration),
        Option<&'a str>,
        chrono::DateTime<chrono::Utc>,
    ),
    CommandError,
> {
    let (target_id, rest) = get_user_arg!(ctx, args, required);

    let (maybe_duration, reason) = rest.split_once(' ').unwrap_or((rest, ""));

    let now = Utc::now();

    let (expires_at_and_duration, rest) = if let Some(std_duration) = parse_duration(maybe_duration)
    {
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
            return Err(CommandError::Ignore);
        }
        (
            (
                if let Some(time) =
                    now.checked_add_signed(match TimeDelta::from_std(std_duration) {
                        Ok(delta) => delta,
                        Err(e) => {
                            tracing::error!("Failed to convert Duration to TimeDelta: {e}");
                            return Err(CommandError::Ignore);
                        }
                    })
                {
                    time
                } else {
                    tracing::error!(?std_duration, %now, "Duration add overflow!");
                    return Err(CommandError::Ignore);
                },
                std_duration,
            ),
            reason,
        )
    } else {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "Provide a duration.",
                        color: 0xff0000
                    }
                ),
            )
            .await?;
        return Err(CommandError::Ignore);
    };

    let rest = rest.trim();
    let reason = if rest.is_empty() { None } else { Some(rest) };

    Ok((target_id, expires_at_and_duration, reason, now))
}

/// Get the target ID, expiry, and reason.
/// Also returns `now` as the last tuple element.
async fn moderation_common<'a>(
    ctx: &CommandContext<'_>,
    args: &'a str,
) -> Result<
    (
        Id<UserMarker>,
        Option<(chrono::DateTime<Utc>, Duration)>,
        Option<&'a str>,
        chrono::DateTime<chrono::Utc>,
    ),
    CommandError,
> {
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
                return Err(CommandError::Ignore);
            }
            (
                Some((
                    if let Some(time) =
                        now.checked_add_signed(match TimeDelta::from_std(std_duration) {
                            Ok(delta) => delta,
                            Err(e) => {
                                tracing::error!("Failed to convert Duration to TimeDelta: {e}");
                                return Err(CommandError::Ignore);
                            }
                        })
                    {
                        time
                    } else {
                        tracing::error!(?std_duration, %now, "Duration add overflow!");
                        return Err(CommandError::Ignore);
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

    Ok((target_id, expires_at_and_duration, reason, now))
}
