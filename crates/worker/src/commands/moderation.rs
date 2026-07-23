use std::sync::Arc;

use anyhow::Context;
use api_types::db::ModerationKind;
use async_brigadier::arg::{greedy_string, literal, optional};
use chrono::Utc;
use fluxer_neptunium::{
    create_embed,
    exts::GuildExt,
    http::endpoints::guild::BanGuildMemberBody,
    model::{
        guild::permissions::Permissions,
        id::{Id, marker::UserMarker},
        time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
    },
};
use tracing::instrument;

use crate::{
    commands::{
        CommandContext, Ctx, Dispatcher,
        args::{expiry, user_id_or_mention},
    },
    db::schema::CreateGuildModerationCaseData,
    util::{Expiry, has_permission, user_fetcher::fetch_and_add_users_to_db},
};

async fn warn(mut ctx: Ctx) -> anyhow::Result<()> {
    let expiry: Option<Expiry> = ctx.try_take_argument_downcast("duration");
    let target_id: Id<UserMarker> = ctx.take_argument("target_id");
    let reason: Option<String> = ctx.try_take_argument_downcast("reason");

    create_moderation_case(
        &ctx,
        target_id,
        expiry,
        reason.as_deref(),
        ModerationKind::Warn,
    )
    .await?;

    ctx.reply(create_embed!(
        description: format!(
            "**Warned** <@{target_id}>{}\n{}",
            if let Some((expires_at, _)) = expiry {
                format!(
                    " until {}.",
                    Timestamp::<UnixMillis>::from(expires_at).time_string(TimestampDisplayType::VerboseDateWithDayOfWeekAndShortTime),
                )
            } else { ".".to_owned() },
            if let Some(reason) = reason {
                format!("**Reason:** {reason}")
            } else {
                "*No reason.*".to_owned()
            }
        ),
        color: 0xffffff,
    ))
    .await?;

    Ok(())
}

async fn mute(mut ctx: Ctx) -> anyhow::Result<()> {
    let expiry: Expiry = ctx.take_argument("duration");
    let target_id: Id<UserMarker> = ctx.take_argument("target_id");
    let reason: Option<String> = ctx.try_take_argument_downcast("reason");

    ctx.guild_id
        .timeout_member(&ctx.ctx, target_id, expiry.0)
        .await?;

    create_moderation_case(
        &ctx,
        target_id,
        Some(expiry),
        reason.as_deref(),
        ModerationKind::Mute,
    )
    .await?;

    ctx.reply(create_embed!(
        description: format!(
            "**Muted** <@{target_id}>{}\n{}",
            format!(
                " until {}.",
                Timestamp::<UnixMillis>::from(expiry.0).time_string(TimestampDisplayType::VerboseDateWithDayOfWeekAndShortTime),
            ),
            if let Some(reason) = reason {
                format!("**Reason:** {reason}")
            } else {
                "*No reason.*".to_owned()
            }
        ),
        color: 0xffffff,
    ))
    .await?;

    Ok(())
}

async fn kick(mut ctx: Ctx) -> anyhow::Result<()> {
    let target_id: Id<UserMarker> = ctx.take_argument("target_id");
    let reason: Option<String> = ctx.try_take_argument_downcast("reason");

    ctx.guild_id.kick_member(&ctx.ctx, target_id).await?;

    create_moderation_case(
        &ctx,
        target_id,
        None,
        reason.as_deref(),
        ModerationKind::Kick,
    )
    .await?;

    ctx.reply(create_embed!(
        description: format!(
            "**Kicked** <@{target_id}>.\n{}",
            if let Some(reason) = reason {
                format!("**Reason:** {reason}")
            } else {
                "*No reason.*".to_owned()
            }
        ),
        color: 0xffffff,
    ))
    .await?;

    Ok(())
}

async fn ban(mut ctx: Ctx) -> anyhow::Result<()> {
    let expiry: Option<Expiry> = ctx.try_take_argument_downcast("duration");
    let target_id: Id<UserMarker> = ctx.take_argument("target_id");
    let reason: Option<String> = ctx.try_take_argument_downcast("reason");

    ctx.guild_id
        .ban_member(
            &ctx.ctx,
            target_id,
            BanGuildMemberBody {
                ban_duration: expiry.map(|expiry| expiry.1.into()),
                delete_message_days: None,
                reason: Some(format!(
                    "Kicked by {}#{} - {}",
                    ctx.message.author.username,
                    ctx.message.author.discriminator,
                    if let Some(reason) = &reason {
                        reason
                    } else {
                        "No reason provided."
                    }
                )),
            },
        )
        .await?;

    create_moderation_case(
        &ctx,
        target_id,
        expiry,
        reason.as_deref(),
        ModerationKind::Ban,
    )
    .await?;

    ctx.reply(create_embed!(
        description: format!(
            "**Banned** <@{target_id}>{}\n{}",
            if let Some((expires_at, _)) = expiry {
                format!(
                    " until {}.",
                    Timestamp::<UnixMillis>::from(expires_at).time_string(TimestampDisplayType::VerboseDateWithDayOfWeekAndShortTime),
                )
            } else { ".".to_owned() },
            if let Some(reason) = reason {
                format!("**Reason:** {reason}")
            } else {
                "*No reason.*".to_owned()
            }
        ),
        color: 0xffffff,
    ))
    .await?;

    Ok(())
}

#[instrument(skip(ctx, expiry, reason, moderation_kind), fields(target_id = target_id.into_inner(), guild_id = ctx.guild_id.into_inner()))]
async fn create_moderation_case(
    ctx: &Ctx,
    target_id: Id<UserMarker>,
    expiry: Option<Expiry>,
    reason: Option<&str>,
    moderation_kind: ModerationKind,
) -> anyhow::Result<()> {
    ctx.db
        .create_moderation_case(
            CreateGuildModerationCaseData {
                guild_id: ctx.guild_id,
                target_id,
                moderator_id: Some(ctx.message.author.id),
                moderation_kind,
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
                created_at: Utc::now(),
            },
            fetch_and_add_users_to_db(
                Arc::clone(&ctx.db),
                ctx.ctx.clone(),
                [target_id, ctx.message.author.id],
            ),
        )
        .await
        .context("Failed to create moderation case")?;
    Ok(())
}

pub fn register(dispatcher: &mut Dispatcher) {
    dispatcher.register(
        literal("warn")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::MODERATE_MEMBERS)
            })
            .then(user_id_or_mention("target_id").then(
                optional(expiry("duration")).then(optional(greedy_string("reason")).executes(warn)),
            )),
    );
    dispatcher.register(
        literal("mute")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::MODERATE_MEMBERS)
            })
            .then(
                user_id_or_mention("target_id").then(
                    expiry("duration").then(optional(greedy_string("reason")).executes(mute)),
                ),
            ),
    );
    dispatcher.register(
        literal("kick")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::KICK_MEMBERS)
            })
            .then(
                user_id_or_mention("target_id")
                    .then(optional(greedy_string("reason")).executes(kick)),
            ),
    );
    dispatcher.register(
        literal("ban")
            .requires(|ctx: &CommandContext| {
                has_permission(ctx.member_permissions, Permissions::BAN_MEMBERS)
            })
            .then(user_id_or_mention("target_id").then(
                optional(expiry("duration")).then(optional(greedy_string("reason")).executes(ban)),
            )),
    );
}
