use std::{sync::Arc, time::Duration};

use api_types::db::ModerationKind;
use async_brigadier::arg::{greedy_string, literal, optional};
use chrono::Utc;
use fluxer_neptunium::{
    create_embed,
    model::{
        id::{Id, marker::UserMarker},
        time::timestamp::{Timestamp, TimestampDisplayType, representations::UnixMillis},
    },
};

use crate::{
    commands::{
        Ctx, Dispatcher,
        args::{duration, user_id_or_mention},
    },
    db::schema::CreateGuildModerationCaseData,
    util::{expiry_from_duration, user_fetcher::fetch_and_add_users_to_db},
};

pub fn register(dispatcher: &mut Dispatcher) {
    dispatcher.register(literal("warn").then(user_id_or_mention("target_id").then(
        optional(duration("duration")).then(optional(greedy_string("reason")).executes(
            async |mut ctx: Ctx| {
                let duration: Option<Duration> = ctx
                    .try_take_argument("duration")
                    .and_then(|value| value.downcast().ok())
                    .map(|value| *value);
                let target_id: Id<UserMarker> = ctx.take_argument("target_id");
                let reason: Option<String> = ctx
                    .try_take_argument("reason")
                    .and_then(|value| value.downcast().ok())
                    .map(|value| *value);

                let now = Utc::now();
                let expiry = if let Some(duration) = duration {
                    Some(expiry_from_duration(now, duration)?)
                } else {
                    None
                };

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
                                        tracing::error!(
                                            ?duration,
                                            "Failed to convert seconds to i64."
                                        );
                                        return Ok(());
                                    }
                                }
                                None => None,
                            },
                            reason: reason.as_deref(),
                            created_at: now,
                        },
                        fetch_and_add_users_to_db(
                            Arc::clone(&ctx.db),
                            ctx.ctx.clone(),
                            [target_id, ctx.message.author.id],
                        ),
                    )
                    .await?;

                ctx.reply(create_embed!(
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
                ))
                .await?;

                Ok(())
            },
        )),
    )));
}
