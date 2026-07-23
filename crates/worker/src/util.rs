use std::time::Duration;

use async_brigadier::CommandParseError;
use chrono::{TimeDelta, Utc};
use fluxer_neptunium::model::{
    guild::permissions::Permissions,
    id::{
        Id,
        marker::{ChannelMarker, GuildMarker, WebhookMarker},
    },
};
use nom::{Parser, error::ErrorKind};

pub mod confirmation;
// pub mod pages;
// pub mod user_arg;
pub mod user_fetcher;

pub type Expiry = (chrono::DateTime<Utc>, Duration);

pub enum MaybeExpired<T> {
    NotExpired(T),
    Expired,
}

pub type MaybeExpiringResult<T, E> = Result<MaybeExpired<T>, E>;

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

pub fn parse_webhook_url(url: &str) -> Option<(Id<WebhookMarker>, &str)> {
    let mut parts = url.split('/').filter(|s| !s.is_empty());
    let token = parts.next_back()?;
    let id_str = parts.next_back()?;
    let id = Id::try_from(id_str).ok()?;
    Some((id, token))
}

pub fn parse_channel_mention_or_id_or_link(
    input: &str,
) -> Option<(Option<Id<GuildMarker>>, Id<ChannelMarker>)> {
    if let Some(input) = input.strip_prefix("<#") {
        if let Some(input) = input.strip_suffix(">")
            && let Ok(id) = input.try_into()
        {
            Some((None, id))
        } else {
            None
        }
    } else if let Ok(id) = Id::try_from(input) {
        Some((None, id))
    } else {
        let mut parts = input.split('/').filter(|part| !part.is_empty());
        let channel_id_str = parts.next_back()?;
        let guild_id_str = parts.next_back()?;
        Some((
            Some(guild_id_str.try_into().ok()?),
            channel_id_str.try_into().ok()?,
        ))
    }
}

pub fn expiry_from_duration(
    now: chrono::DateTime<Utc>,
    std_duration: Duration,
) -> Result<Expiry, CommandParseError> {
    if std_duration > Duration::from_hours(24 * 356) {
        return Err(CommandParseError::Other(
            "The duration can not be more than 1 year.",
        ));
    }
    Ok((
        if let Some(time) = now.checked_add_signed(match TimeDelta::from_std(std_duration) {
            Ok(delta) => delta,
            Err(e) => {
                tracing::error!("Failed to convert Duration to TimeDelta: {e}");
                return Err(CommandParseError::Other("Internal error"));
            }
        }) {
            time
        } else {
            tracing::error!(?std_duration, %now, "Duration add overflow!");
            return Err(CommandParseError::Other("Internal error"));
        },
        std_duration,
    ))
}

pub fn has_permission(permissions: Permissions, require: Permissions) -> bool {
    if permissions.contains(Permissions::ADMINISTRATOR) {
        true
    } else {
        permissions.contains(require)
    }
}
