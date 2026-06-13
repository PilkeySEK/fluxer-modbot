use std::str::FromStr;

use api_types::db::{CaseId, GuildModerationCase, GuildModerationCaseExpiry};
use chrono::Utc;
use fluxer_neptunium::{
    cache::Cached,
    exts::MessageExt,
    http::endpoints::channel::{CreateMessageBody, EditMessageBody},
    model::{
        id::{Id, marker::UserMarker},
        time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
    },
};
use pretty_duration::pretty_duration;

use crate::{
    commands::{CommandContext, CommandError},
    macros::{debug_panic, embed_default_footer, get_user_arg},
    util::{
        MaybeExpired, MaybeExpiringResult,
        pages::{PageAction, pages},
        user_arg::parse_user_arg,
    },
};

const MAX_GUILD_MODERATION_CASES_PER_MESSAGE: i64 = 10;

pub async fn list_cases(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (involving_user, _rest) = get_user_arg!(ctx, args, not required);

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
            ctx.ctx,
            format_case_list(&ctx, cases, case_count, involving_user, 0),
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
                        debug_panic!("Offset of {current_offset} > case count {case_count}");
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
                    ctx.ctx,
                    format_case_list(&ctx, cases, case_count, involving_user, current_offset),
                )
                .await?;
            tracing::info!(?result);
        }
    } else {
        Ok(())
    }
}

pub async fn case_info(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (maybe_case_id_or_user_str, _rest) = args.split_once(' ').unwrap_or((args, ""));

    let case = if maybe_case_id_or_user_str.is_empty() {
        if let Some(case) = ctx
            .db
            .get_last_guild_moderation_case_involving_user(ctx.guild_id, ctx.message.author.id)
            .await?
        {
            case
        } else {
            ctx.message
                .reply(
                    ctx.ctx,
                    embed_default_footer!(
                        ctx,
                        {
                            description: "You do not have any previous moderation cases.",
                            color: 0xffffff,
                        }
                    ),
                )
                .await?;
            return Ok(());
        }
    } else if let Ok(case_id) = CaseId::from_str(maybe_case_id_or_user_str)
        && let Some(case) = ctx
            .db
            .get_guild_moderation_case(ctx.guild_id, case_id)
            .await?
    {
        case
    } else {
        match parse_user_arg(&ctx, maybe_case_id_or_user_str).await? {
            MaybeExpired::NotExpired(Some(user_id)) => {
                if let Some(case) = ctx
                    .db
                    .get_last_guild_moderation_case_involving_user(ctx.guild_id, user_id)
                    .await?
                {
                    case
                } else {
                    ctx.message.reply(ctx.ctx, embed_default_footer!(
                        ctx,
                        {
                            description: "That user does not have any previous moderation cases.",
                            color: 0xffffff,
                        }
                    )).await?;
                    return Ok(());
                }
            }
            MaybeExpired::NotExpired(None) | MaybeExpired::Expired => return Ok(()),
        }
    };

    ctx.message
        .reply(ctx.ctx, format_case_info(&ctx, case))
        .await?;

    Ok(())
}

fn format_case_info(ctx: &CommandContext<'_>, case: GuildModerationCase) -> CreateMessageBody {
    let case_string = format!(
        "> **Type:** `{}`\n> **User:** <@{}> ({})\n> **Reason:** {}\n> **Duration:** {}\n> **Moderator:** {}\n> **Closed:** {}",
        case.moderation_kind,
        case.target_id,
        case.target_id,
        case.reason
            .unwrap_or_else(|| "*No reason provided.*".to_owned()),
        case.expiry.map_or_else(
            || "Permanent".to_string(),
            |GuildModerationCaseExpiry {
                 expires_at,
                 duration,
             }| {
                let now = Utc::now();
                format!(
                    "{} ({} {})",
                    pretty_duration(&duration, crate::PRETTY_DURATION_OPTIONS),
                    if expires_at > now {
                        "expires"
                    } else {
                        "expired"
                    },
                    Timestamp::<UnixMillis>::from(expires_at)
                        .time_string(TimestampDisplayType::Relative)
                )
            }
        ),
        case.moderator_id.map_or_else(
            || "*Automated action.*".to_string(),
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

    embed_default_footer!(
        ctx,
        {
            title: format!("Case `{}`", case.case_id),
            description: case_string,
            color: 0xffffff,
        }
    )
    .into()
}

fn format_case_list(
    ctx: &CommandContext<'_>,
    cases: Vec<GuildModerationCase>,
    case_count: i64,
    involving_user: Option<Id<UserMarker>>,
    offset: i64,
) -> EditMessageBody {
    fn format_case_oneline(case: GuildModerationCase) -> String {
        format!(
            "[{}] **{}** of <@{}> - `{}` ({}){}: {}",
            Timestamp::<UnixMillis>::from(case.created_at).time_string(TimestampDisplayType::ShortDate),
            case.moderation_kind,
            case.target_id,
            case.case_id,
            if let Some(GuildModerationCaseExpiry {
                expires_at,
                duration: _,
            }) = case.expiry
            {
                format!(
                    "expires {}",
                    Timestamp::<UnixMillis>::from(expires_at)
                        .time_string(TimestampDisplayType::Relative)
                )
            } else {
                "permanent".to_owned()
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

    #[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    EditMessageBody::builder().embeds(vec![embed_default_footer!(
        ctx,
        {
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
        }
    )]).build()
}
