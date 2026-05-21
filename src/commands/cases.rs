use fluxer_neptunium::{
    exts::MessageExt,
    http::endpoints::channel::EditMessageBody,
    model::time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
};

use crate::{
    commands::{CommandContext, CommandError},
    db::schema::GuildModerationCase,
    macros::embed_default_footer,
    util::{Expiry, try_db, user_arg::parse_user_arg},
};

const MAX_GUILD_MODERATION_CASES_PER_MESSAGE: i64 = 10;

pub async fn list_cases(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (involving_user_str, _rest) = args.split_once(' ').unwrap_or((args, ""));
    let involving_user_str = involving_user_str.trim();
    let involving_user = if involving_user_str.is_empty() {
        None
    } else {
        Some(match parse_user_arg(&ctx, involving_user_str).await? {
            Expiry::Expired => return Ok(()),
            Expiry::NotExpired(Some(id)) => id,
            Expiry::NotExpired(None) => {
                ctx.message
                    .reply(
                        ctx.ctx,
                        embed_default_footer!(
                            ctx,
                            {
                                description: "Could not find a user matching your query.",
                                color: 0xff0000,
                            }
                        ),
                    )
                    .await?;
                return Ok(());
            }
        })
    };

    let (cases, case_count) = tokio::join!(
        ctx.db.list_guild_moderation_cases(
            ctx.guild_id,
            MAX_GUILD_MODERATION_CASES_PER_MESSAGE,
            None,
            involving_user
        ),
        ctx.db
            .count_guild_moderation_cases(ctx.guild_id, involving_user)
    );
    let cases = try_db(&ctx, cases).await?;
    let case_count = try_db(&ctx, case_count).await?;

    ctx.message
        .reply(ctx.ctx, format_case_list(&ctx, cases, case_count))
        .await?;

    Ok(())
}

fn format_case_list(
    ctx: &CommandContext<'_>,
    cases: Vec<GuildModerationCase>,
    case_count: i64,
) -> EditMessageBody {
    fn format_case_oneline(case: GuildModerationCase) -> String {
        format!(
            "[{}] **{}** of <@{}> - `{}` ({}){}: {}",
            Timestamp::<UnixMillis>::from(case.created_at).time_string(TimestampDisplayType::Date),
            case.moderation_kind,
            case.target_id,
            case.case_id,
            if let Some(expires_at) = case.expires_at {
                format!(
                    "expires {}",
                    Timestamp::<UnixMillis>::from(expires_at)
                        .time_string(TimestampDisplayType::Relative)
                )
            } else {
                "permanent".to_owned()
            },
            if case.closed { " (**closed**)" } else { "" },
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
                "Displaying `{}` out of `{}` cases matching the current filters:\n\n{}\n-# Page {}/{}",
                cases_formatted.len(),
                case_count,
                cases_string,
                1,
                ((case_count as f64) / (MAX_GUILD_MODERATION_CASES_PER_MESSAGE as f64)).ceil() as i64
            ),
            color: 0xffffff,
        }
    )]).build()
}

/*

use std::time::Duration;

use fluxer_neptunium::{
    exts::MessageExt,
    http::endpoints::channel::EditMessageBody,
    model::{
        guild::Emoji,
        time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
    },
};
use pretty_duration::pretty_duration;
use time::OffsetDateTime;

use crate::{
    commands::{CommandContext, CommandError},
    db::schema::{CaseId, GuildModerationCase},
    macros::embed_default_footer,
    util::{parse_mention_or_id, try_db},
};

const MAX_GUILD_MODERATION_CASES_PER_MESSAGE: i64 = 10;
const REACTION_EXPIRY_TIME: Option<Duration> = Some(Duration::from_hours(1));

fn format_case_oneline(case: GuildModerationCase) -> String {
    format!(
        "[{}] **{}** of <@{}> - `{}` ({}){}: {}",
        Timestamp::<UnixMillis>::from(case.created_at).time_string(TimestampDisplayType::Date),
        case.moderation_kind,
        case.target_id,
        case.case_id,
        if let Some(expires_at) = case.expires_at {
            format!(
                "expires {}",
                Timestamp::<UnixMillis>::from(expires_at)
                    .time_string(TimestampDisplayType::Relative)
            )
        } else {
            "permanent".to_owned()
        },
        if case.closed { " (**closed**)" } else { "" },
        case.reason.unwrap_or_else(|| "*No reason.*".to_owned()),
    )
}

fn format_case_list(
    ctx: &CommandContext<'_>,
    cases: Vec<GuildModerationCase>,
    case_count: i64,
) -> EditMessageBody {
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
                "Displaying `{}` out of `{}` cases matching the current filters:\n\n{}\n-# Page {}/{}",
                cases_formatted.len(),
                case_count,
                cases_string,
                1,
                ((case_count as f64) / (MAX_GUILD_MODERATION_CASES_PER_MESSAGE as f64)).ceil() as i64
            ),
            color: 0xffffff,
        }
    )]).build()
}

pub async fn list_cases(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (involving_user_str, _rest) = args.split_once(' ').unwrap_or((args, ""));
    let involving_user = parse_mention_or_id(involving_user_str);

    let (cases, case_count) = tokio::join!(
        ctx.db.list_guild_moderation_cases(
            ctx.guild_id,
            MAX_GUILD_MODERATION_CASES_PER_MESSAGE,
            None,
            involving_user
        ),
        ctx.db
            .count_guild_moderation_cases(ctx.guild_id, involving_user)
    );
    let cases = try_db(&ctx, cases).await?;
    let cases_len = cases.len();
    let case_count = try_db(&ctx, case_count).await?;

    let message = ctx
        .message
        .reply(ctx.ctx, format_case_list(&ctx, cases, case_count))
        .await?;

    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    if cases_len < case_count as usize {
        message.add_reaction(ctx.ctx, "⬅️").await?;
        message.add_reaction(ctx.ctx, "➡️").await?;
        let response_owner_id = ctx.message.author.id;
        ctx.register_reaction_handler(
            message.id,
            Box::new(move |_, event| {
                Box::pin(async move {
                    if event.user_id != response_owner_id {
                        return (false, Ok(()));
                    }
                    let Emoji::Default(emoji) = &event.emoji else {
                        return (false, Ok(()));
                    };
                    if emoji == "⬅️" {
                        todo!()
                    } else if emoji == "➡️" {
                        todo!()
                    } else {
                        (false, Ok(()))
                    }
                })
            }),
            REACTION_EXPIRY_TIME,
        );
    }

    Ok(())
}

fn format_case_details(case: GuildModerationCase) -> String {
    format!(
        "> **Type:** `{}`\n> **User:** <@{}> ({})\n> **Reason:** {}\n> **Duration:** {}{}\n> **Moderator:** {}",
        case.moderation_kind,
        case.target_id,
        case.target_id,
        case.reason
            .unwrap_or_else(|| "*No reason provided.*".to_owned()),
        case.duration.map_or_else(
            || "Permanent".to_string(),
            |duration| pretty_duration(&duration, crate::PRETTY_DURATION_OPTIONS)
        ),
        case.expires_at.map_or_else(String::new, |expires_at| {
            let now = OffsetDateTime::now_utc();
            let word = if expires_at > now {
                "expires"
            } else {
                "expired"
            };
            format!(
                " ({} {})",
                word,
                Timestamp::<UnixMillis>::from(expires_at)
                    .time_string(TimestampDisplayType::Relative)
            )
        }),
        case.moderator_id.map_or_else(
            || "*Automated action.*".to_string(),
            |moderator_id| format!("<@{moderator_id}>")
        ),
    )
}

pub async fn case_info(ctx: CommandContext<'_>, args: &str) -> Result<(), CommandError> {
    let (case_id_str, _rest) = args.split_once(' ').unwrap_or((args, ""));

    let case_id = CaseId::from_str(case_id_str);

    let case = if let Some(case_id) = case_id {
        try_db(
            &ctx,
            ctx.db
                .get_guild_moderation_case(ctx.guild_id, case_id)
                .await,
        )
        .await?
    } else {
        let Some(case) = try_db(
            &ctx,
            ctx.db
                .get_last_guild_moderation_case_made_by_user(ctx.guild_id, ctx.message.author.id)
                .await,
        )
        .await?
        else {
            ctx.message
                    .reply(
                        ctx.ctx,
                        embed_default_footer!(
                            ctx,
                            {
                                description: "You did not create any cases before, so no case can be displayed. Please provide a case ID.",
                                color: 0xff0000,
                            }
                        ),
                    )
                    .await?;
            return Ok(());
        };
        Some(case)
    };

    match case {
        Some(case) => {
            ctx.message
                .reply(
                    ctx.ctx,
                    embed_default_footer!(
                        ctx,
                        {
                            title: format!("Case `{}`", case.case_id),
                            description: format_case_details(case),
                            color: 0xffffff,
                        }
                    ),
                )
                .await?;
        }
        None => {
            ctx.message
                .reply(
                    ctx.ctx,
                    embed_default_footer!(
                        ctx,
                        {
                            description: format!("The case `{case_id_str}` doesn't exist."),
                            color: 0xff0000,
                        }
                    ),
                )
                .await?;
        }
    }

    Ok(())
}

*/
