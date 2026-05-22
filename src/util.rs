use std::time::Duration;

use fluxer_neptunium::{
    exts::MessageExt,
    model::id::{Id, marker::UserMarker},
};
use nom::{Parser, error::ErrorKind};

use crate::{commands::CommandContext, db::DatabaseError};

pub mod confirmation;
pub mod pages;
pub mod user_arg;

pub enum Expiry<T> {
    NotExpired(T),
    Expired,
}

pub type MaybeExpiringResult<T, E> = Result<Expiry<T>, E>;

pub fn parse_mention_or_id(input: &str) -> Option<Id<UserMarker>> {
    Id::try_from(
        input
            .trim_start()
            .strip_prefix("<@")
            .and_then(|input| input.strip_suffix('>'))
            .unwrap_or(input),
    )
    .ok()
}

pub fn parse_duration(input: &str) -> Option<Duration> {
    let Ok((_, duration_elems)) = nom::multi::many1(nom::sequence::pair(
        nom::character::complete::u64::<_, (_, ErrorKind)>,
        nom::character::complete::alpha1,
    ))
    .parse(input) else {
        return None;
    };

    duration_elems
        .into_iter()
        .map(|(number, unit)| {
            Some(match unit {
                "s" | "sec" | "seconds" | "secs" => Duration::from_secs(number),
                "m" | "min" | "minutes" | "mins" => Duration::from_mins(number),
                "h" | "hour" => Duration::from_hours(number),
                "d" | "day" | "days" => Duration::from_hours(number.checked_mul(24)?),
                _ => return None,
            })
        })
        .collect::<Option<Vec<Duration>>>()?
        .into_iter()
        .try_fold(Duration::ZERO, std::time::Duration::checked_add)
}

pub async fn try_db<T>(
    ctx: &CommandContext<'_>,
    result: Result<T, DatabaseError>,
) -> Result<T, DatabaseError> {
    match result {
        Ok(value) => Ok(value),
        Err(e) => {
            let _ = ctx
                .message
                .reply(
                    ctx.ctx,
                    crate::macros::embed_default_footer!(
                        ctx,
                        {
                            description: "Database error.",
                            color: 0xff0000,
                        }
                    ),
                )
                .await;
            Err(e)
        }
    }
}
