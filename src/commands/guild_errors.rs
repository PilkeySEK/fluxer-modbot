use fluxer_neptunium::{
    cache::Cached,
    exts::MessageExt,
    http::endpoints::channel::EditMessageBody,
    model::time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
};

use crate::{
    commands::{CommandContext, CommandError},
    db::schema::GuildError,
    macros::{debug_panic, embed_default_footer},
    util::{
        MaybeExpired,
        pages::{PageAction, pages},
    },
};

const MAX_GUILD_ERRORS_PER_MESSAGE: i64 = 10;

pub async fn list_guild_errors(ctx: CommandContext<'_>, _args: &str) -> Result<(), CommandError> {
    let (errors, error_count) = tokio::join!(
        ctx.db
            .list_guild_errors(ctx.guild_id, MAX_GUILD_ERRORS_PER_MESSAGE, 0),
        ctx.db.count_guild_errors(ctx.guild_id)
    );
    let errors = errors?;
    let error_count = error_count?;

    let message = ctx
        .message
        .reply(
            ctx.ctx,
            format_guild_error_list(&ctx, errors, 0, error_count),
        )
        .await?;

    if error_count > MAX_GUILD_ERRORS_PER_MESSAGE {
        let mut current_offset = 0;
        let mut is_first_loop_iteration = true;
        loop {
            let page_action = match pages(
                &ctx,
                Cached::clone(&message),
                ctx.message.author.id,
                if current_offset + MAX_GUILD_ERRORS_PER_MESSAGE >= error_count {
                    [PageAction::Back].into()
                } else if current_offset <= 0 {
                    if current_offset < 0 {
                        debug_panic!("current_offset = {current_offset} < 0");
                    }
                    [PageAction::Continue].into()
                } else {
                    [PageAction::Back, PageAction::Continue].into()
                },
                is_first_loop_iteration,
            )
            .await?
            {
                MaybeExpired::Expired => break,
                MaybeExpired::NotExpired(action) => action,
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
                    if current_offset >= error_count {
                        debug_panic!("Offset of {current_offset} > case count {error_count}");
                    }
                }
            }

            let errors = ctx
                .db
                .list_guild_errors(ctx.guild_id, MAX_GUILD_ERRORS_PER_MESSAGE, current_offset)
                .await?;

            message
                .edit(
                    ctx.ctx,
                    format_guild_error_list(&ctx, errors, current_offset, error_count),
                )
                .await?;

            is_first_loop_iteration = false;
        }
    }
    Ok(())
}

fn format_guild_error_list(
    ctx: &CommandContext<'_>,
    errors: Vec<GuildError>,
    offset: i64,
    error_count: i64,
) -> EditMessageBody {
    let errors_len = errors.len();
    let errors_str = errors
        .into_iter()
        .map(|error| {
            format!(
                "[{}] ({}) {}",
                Timestamp::<UnixMillis>::from(error.created_at)
                    .time_string(TimestampDisplayType::Date),
                error.log_entry_id,
                error.message
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    #[expect(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
    EditMessageBody::builder()
        .embeds(vec![embed_default_footer!(
            ctx,
            {
                description: format!(
                    "Displaying `{}` out of `{}` errors:\n\n{}\n-# Page {}/{}",
                    errors_len,
                    error_count,
                    if errors_str.is_empty() {
                        "*There are no errors.*".to_owned()
                    } else {
                        errors_str
                    },
                    ((offset as f64) / MAX_GUILD_ERRORS_PER_MESSAGE as f64).ceil() as i64 + 1,
                    ((error_count as f64) / (MAX_GUILD_ERRORS_PER_MESSAGE as f64)).ceil() as i64
                ),
                color: 0xffffff,
            }
        )])
        .build()
}
