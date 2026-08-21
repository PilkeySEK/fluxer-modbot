use async_brigadier::arg::optional;
use chrono::Utc;
use fluxer_neptunium::{
    cache::Cached,
    create_embed,
    exts::MessageExt,
    http::endpoints::channel::{CreateMessageBody, EditMessageBody},
    model::{
        id::{Id, marker::UserMarker},
        time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
    },
};
use rust_shared::db::{CaseId, GuildModerationCase, GuildModerationCaseExpiry, ModerationKind};

use crate::{
    commands::{
        Arg, Ctx,
        args::{case_id, empty, user_id_or_mention},
    },
    macros::debug_panic,
    util::{
        MaybeExpired, MaybeExpiringResult,
        confirmation::confirmation,
        pages::{PageAction, pages},
    },
};

const MAX_GUILD_MODERATION_CASES_PER_MESSAGE: i64 = 10;

pub fn list_cases() -> Arg {
    empty().then(
        optional(user_id_or_mention("involving_user")).executes(async |mut ctx: Ctx| {
            let involving_user: Option<Id<UserMarker>> =
                ctx.try_take_argument_downcast("involving_user");

            let (cases, case_count) = tokio::join!(
                ctx.db.list_guild_moderation_cases_by_involving_user(
                    ctx.guild_id,
                    MAX_GUILD_MODERATION_CASES_PER_MESSAGE,
                    None,
                    involving_user
                ),
                ctx.db
                    .count_guild_moderation_cases_by_involving_user(ctx.guild_id, involving_user)
            );
            let cases = cases?;
            let case_count = case_count?;

            let message = ctx
                .message
                .reply(
                    &ctx.ctx,
                    format_case_list(cases, case_count, involving_user, 0),
                )
                .await?;

            if case_count > MAX_GUILD_MODERATION_CASES_PER_MESSAGE {
                let mut current_offset: i64 = 0;
                loop {
                    let page_action = match pages(
                        &ctx,
                        Cached::clone(&message),
                        ctx.message.author.id,
                        if current_offset + MAX_GUILD_MODERATION_CASES_PER_MESSAGE >= case_count {
                            [PageAction::Back].into()
                        } else if current_offset <= 0 {
                            if current_offset < 0 {
                                debug_panic!("current_offset = {current_offset} < 0");
                            }
                            [PageAction::Continue].into()
                        } else {
                            [PageAction::Back, PageAction::Continue].into()
                        },
                    )
                    .await
                    {
                        MaybeExpiringResult::Err(e) => break Err(e),
                        MaybeExpiringResult::Ok(MaybeExpired::Expired) => break Ok(()),
                        MaybeExpiringResult::Ok(MaybeExpired::NotExpired(action)) => action,
                    };

                    match page_action {
                        PageAction::Back => {
                            current_offset -= 10;
                            if current_offset < 0 {
                                debug_panic!("Offset of {current_offset} > 0");
                            }
                        }
                        PageAction::Continue => {
                            current_offset += 10;
                            if current_offset >= case_count {
                                debug_panic!(
                                    "Offset of {current_offset} > case count {case_count}"
                                );
                            }
                        }
                    }

                    let cases = ctx
                        .db
                        .list_guild_moderation_cases_by_involving_user(
                            ctx.guild_id,
                            MAX_GUILD_MODERATION_CASES_PER_MESSAGE,
                            Some(current_offset),
                            involving_user,
                        )
                        .await?;

                    let result = message
                        .edit(
                            &ctx.ctx,
                            format_case_list(cases, case_count, involving_user, current_offset),
                        )
                        .await?;
                    tracing::info!(?result);
                }
            } else {
                Ok(())
            }
        }),
    )
}

async fn try_get_case_from_maybe_case_id_or_user_id(
    ctx: &Ctx,
    maybe_case_id_or_user_id: Option<either::Either<Id<UserMarker>, CaseId>>,
) -> anyhow::Result<Option<GuildModerationCase>> {
    let case = match maybe_case_id_or_user_id {
        Some(either::Right(case_id)) => {
            ctx.db
                .get_guild_moderation_case(ctx.guild_id, case_id)
                .await?
        }
        Some(either::Left(user_id)) => {
            ctx.db
                .get_last_guild_moderation_case_involving_user(ctx.guild_id, user_id)
                .await?
        }
        None => {
            ctx.db
                .get_last_guild_moderation_case_involving_user(ctx.guild_id, ctx.message.author.id)
                .await?
        }
    };
    Ok(case)
}

pub fn case_info() -> Arg {
    async fn case_info(ctx: Ctx) -> anyhow::Result<()> {
        let maybe_case_id_or_user_id = ctx.try_get_argument("case_id_or_user");

        let maybe_case_id_or_user_id = if let Some(case_id_or_user_id) = maybe_case_id_or_user_id {
            if let Some(case_id) = case_id_or_user_id.downcast_ref::<CaseId>() {
                Some(either::Right(*case_id))
            } else if let Some(user_id) = case_id_or_user_id.downcast_ref::<Id<UserMarker>>() {
                Some(either::Left(*user_id))
            } else {
                tracing::error!("case_id_or_user_id was neither");
                return Ok(());
            }
        } else {
            None
        };

        let Some(case) =
            try_get_case_from_maybe_case_id_or_user_id(&ctx, maybe_case_id_or_user_id).await?
        else {
            ctx.reply_embed(None, "No case found.", Some(0xff0000))
                .await?;
            return Ok(());
        };

        ctx.reply(format_case_info(case)).await?;

        Ok(())
    }

    empty()
        .then(case_id("case_id_or_user").executes(case_info))
        .then(user_id_or_mention("case_id_or_user").executes(case_info))
        .executes(case_info)
}

pub fn delete_case() -> Arg {
    async fn delete_case(ctx: Ctx) -> anyhow::Result<()> {
        let maybe_case_id_or_user_id = ctx.try_get_argument("case_id_or_user");

        let maybe_case_id_or_user_id = if let Some(case_id_or_user_id) = maybe_case_id_or_user_id {
            if let Some(case_id) = case_id_or_user_id.downcast_ref::<CaseId>() {
                Some(either::Right(*case_id))
            } else if let Some(user_id) = case_id_or_user_id.downcast_ref::<Id<UserMarker>>() {
                Some(either::Left(*user_id))
            } else {
                tracing::error!("case_id_or_user_id was neither");
                return Ok(());
            }
        } else {
            None
        };

        let Some(case) =
            try_get_case_from_maybe_case_id_or_user_id(&ctx, maybe_case_id_or_user_id).await?
        else {
            ctx.reply_embed(None, "No case found.", Some(0xff0000))
                .await?;
            return Ok(());
        };

        let confirmation_text = format!(
            "Are you sure you want to delete the case `{}`? The case will be permanently deleted. This cannot be undone.",
            case.case_id
        );
        let confirmation_message = ctx
            .message
            .reply(
                &ctx.ctx,
                create_embed!(
                    description: confirmation_text,
                    color: 0xffffff,
                ),
            )
            .await?;

        match confirmation(&ctx, confirmation_message, ctx.message.author.id).await? {
            MaybeExpired::Expired | MaybeExpired::NotExpired(false) => {}
            MaybeExpired::NotExpired(true) => {
                let query_result = ctx.db.delete_guild_moderation_case(case.case_id).await?;
                if query_result.rows_affected() == 0 {
                    ctx.reply_embed(None, "The case has already been deleted.", Some(0xff0000))
                        .await?;
                } else {
                    ctx.reply_embed(
                        None,
                        format!("Permanently deleted case `{}`", case.case_id),
                        Some(0xffffff),
                    )
                    .await?;
                }
            }
        }

        Ok(())
    }

    empty()
        .then(case_id("case_id_or_use").executes(delete_case))
        .then(user_id_or_mention("case_id_or_user").executes(delete_case))
        .executes(delete_case)
}

fn format_case_info(case: GuildModerationCase) -> CreateMessageBody {
    let case_string = format!(
        "> **Type:** `{}`\n> **User:** <@{}> ({})\n> **Reason:** {}\n> **Duration:** {}\n> **Moderator:** {}\n> **Closed:** {}",
        case.moderation_kind,
        case.target_id,
        case.target_id,
        case.reason
            .unwrap_or_else(|| "*No reason provided.*".to_owned()),
        if case.moderation_kind == ModerationKind::Kick {
            "/".to_owned()
        } else {
            case.expiry.map_or_else(
                || "Permanent".to_owned(),
                |GuildModerationCaseExpiry {
                     expires_at,
                     duration,
                 }| {
                    let now = Utc::now();
                    format!(
                        "{} ({} {})",
                        pretty_duration::pretty_duration(&duration, crate::PRETTY_DURATION_OPTIONS),
                        if expires_at > now {
                            "expires"
                        } else {
                            "expired"
                        },
                        Timestamp::<UnixMillis>::from(expires_at)
                            .time_string(TimestampDisplayType::Relative)
                    )
                },
            )
        },
        case.moderator_id.map_or_else(
            || "*Automated action.*".to_owned(),
            |moderator_id| format!("<@{moderator_id}>")
        ),
        if let Some(close_data) = case.close_data {
            format!(
                "yes{}\n> **Closed by:** {}",
                if let Some(close_reason) = close_data.reason {
                    format!("\n> **Close reason:** {close_reason}")
                } else {
                    String::new()
                },
                if let Some(closed_by) = close_data.closed_by {
                    format!("<@{closed_by}>")
                } else {
                    "*Automated action.*".to_owned()
                }
            )
        } else {
            "no".to_owned()
        },
    );

    #[expect(clippy::disallowed_macros)]
    create_embed!(
        title: format!("Case `{}`", case.case_id),
        description: case_string,
        color: 0xffffff,
    )
    .into()
}

fn format_case_list(
    cases: Vec<GuildModerationCase>,
    case_count: i64,
    involving_user: Option<Id<UserMarker>>,
    offset: i64,
) -> impl Into<CreateMessageBody> + Into<EditMessageBody> {
    fn format_case_oneline(case: GuildModerationCase) -> String {
        format!(
            "[{}] **{}** of <@{}> - `{}`{}{}: {}",
            Timestamp::<UnixMillis>::from(case.created_at)
                .time_string(TimestampDisplayType::ShortDate),
            case.moderation_kind,
            case.target_id,
            case.case_id,
            if case.moderation_kind == ModerationKind::Kick {
                String::new()
            } else if let Some(GuildModerationCaseExpiry {
                expires_at,
                duration: _,
            }) = case.expiry
            {
                format!(
                    " (expires {})",
                    Timestamp::<UnixMillis>::from(expires_at)
                        .time_string(TimestampDisplayType::Relative)
                )
            } else {
                " (permanent)".to_owned()
            },
            if case.close_data.is_some() {
                " (**closed**)"
            } else {
                ""
            },
            case.reason.unwrap_or_else(|| "*No reason.*".to_owned()),
        )
    }

    let cases_formatted = cases
        .into_iter()
        .map(format_case_oneline)
        .collect::<Vec<String>>();
    let cases_string = if cases_formatted.is_empty() {
        "*There are no cases.*".to_owned()
    } else {
        cases_formatted.join("\n")
    };

    #[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss, clippy::disallowed_macros)]
    EditMessageBody::builder().embeds(vec![create_embed!(
        description: format!(
            "Displaying `{}` out of `{}` cases{}:\n\n{}\n-# Page {}/{}",
            cases_formatted.len(),
            case_count,
            if let Some(involving_user) = involving_user {
                format!(" involving <@{involving_user}>")
            } else {
                String::new()
            },
            cases_string,
            ((offset as f64) / MAX_GUILD_MODERATION_CASES_PER_MESSAGE as f64).ceil() as i64 + 1,
            ((case_count as f64) / (MAX_GUILD_MODERATION_CASES_PER_MESSAGE as f64)).ceil() as i64
        ),
        color: 0xffffff,
    )]).build()
}
