use fluxer_neptunium::{
    events::EventError,
    exts::MessageExt,
    model::time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
};
use pretty_duration::pretty_duration;
use time::OffsetDateTime;

use crate::{
    commands::CommandContext,
    db::schema::{CaseId, GuildModerationCase},
    macros::{embed_default_footer, try_db},
    util::parse_mention_or_id,
};

fn format_case(case: GuildModerationCase) -> String {
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

pub async fn list_cases(ctx: CommandContext<'_>, args: &str) -> Result<(), EventError> {
    let (involving_user_str, _rest) = args.split_once(' ').unwrap_or((args, ""));
    let involving_user = parse_mention_or_id(involving_user_str);

    let (cases, case_count) = tokio::join!(
        ctx.db
            .list_guild_moderation_cases(ctx.guild_id, 10, None, involving_user),
        ctx.db
            .count_guild_moderation_cases(ctx.guild_id, involving_user)
    );
    let cases = try_db!(ctx, cases);
    let case_count = try_db!(ctx, case_count);

    let cases_formatted = cases.into_iter().map(format_case).collect::<Vec<String>>();
    let cases_string = if cases_formatted.is_empty() {
        "*There are no cases.*".to_owned()
    } else {
        cases_formatted.join("\n")
    };

    ctx.message
        .reply(
            ctx.ctx,
            embed_default_footer!(
                ctx,
                {
                    description: format!(
                        "Displaying `{}` out of `{}` cases matching the current filters:\n\n{}",
                        cases_formatted.len(),
                        case_count,
                        cases_string
                    ),
                    color: 0xffffff,
                }
            ),
        )
        .await?;

    Ok(())
}

fn case_details(case: GuildModerationCase) -> String {
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

pub async fn case_info(ctx: CommandContext<'_>, args: &str) -> Result<(), EventError> {
    let (case_id_str, _rest) = args.split_once(' ').unwrap_or((args, ""));

    let Some(case_id) = CaseId::from_str(case_id_str) else {
        ctx.message
            .reply(
                ctx.ctx,
                embed_default_footer!(
                    ctx,
                    {
                        description: "Provide a valid case ID.",
                        color: 0xff0000,
                    }
                ),
            )
            .await?;
        return Ok(());
    };

    let case = try_db!(
        ctx,
        ctx.db
            .get_guild_moderation_case(ctx.guild_id, case_id)
            .await
    );

    match case {
        Some(case) => {
            ctx.message
                .reply(
                    ctx.ctx,
                    embed_default_footer!(
                        ctx,
                        {
                            title: format!("Case `{}`", case.case_id),
                            description: case_details(case),
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
