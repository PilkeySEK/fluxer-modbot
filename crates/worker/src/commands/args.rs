use async_brigadier::arg::{CommandArgument, CommandArgumentKind};
use fluxer_neptunium::model::id::{Id, marker::UserMarker};

use crate::util::parse_duration;

pub struct UserIdOrMentionArgumentKind;

impl CommandArgumentKind for UserIdOrMentionArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Box<dyn std::any::Any + Send + Sync>), async_brigadier::CommandParseError>
    {
        let (id_or_mention, rest) = command.split_once(' ').unwrap_or((command, ""));
        let id = if let Some(prefix_stripped) = id_or_mention.strip_prefix("<@")
            && let Some(suffix_stripped) = prefix_stripped.strip_suffix(">")
        {
            let Ok(id) = Id::<UserMarker>::try_from(suffix_stripped) else {
                return Err(async_brigadier::CommandParseError::NoMatch);
            };
            id
        } else {
            let Ok(id) = Id::<UserMarker>::try_from(id_or_mention) else {
                return Err(async_brigadier::CommandParseError::NoMatch);
            };
            id
        };
        Ok((rest, Box::new(id)))
    }
}

#[must_use]
pub fn user_id_or_mention<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(UserIdOrMentionArgumentKind),
        name: Some(name),
        executes: None,
        requires: None,
    }
}

pub struct DurationArgumentKind;

impl CommandArgumentKind for DurationArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Box<dyn std::any::Any + Send + Sync>), async_brigadier::CommandParseError>
    {
        let (duration, rest) = command.split_once(' ').unwrap_or((command, ""));
        let duration = parse_duration(duration);
        if let Some(duration) = duration {
            Ok((rest, Box::new(duration)))
        } else {
            Err(async_brigadier::CommandParseError::NoMatch)
        }
    }
}

#[must_use]
pub fn duration<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(DurationArgumentKind),
        name: Some(name),
        executes: None,
        requires: None,
    }
}
