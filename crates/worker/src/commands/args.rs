use std::{any::Any, str::FromStr};

use async_brigadier::{
    CommandParseError,
    arg::{CommandArgument, CommandArgumentKind},
};
use chrono::Utc;
use fluxer_neptunium::model::id::{Id, marker::UserMarker};
use rust_shared::db::CaseId;

use crate::util::{
    expiry_from_duration, parse_channel_mention_or_id_or_link, parse_duration, parse_webhook_url,
};

pub struct UserIdOrMentionArgumentKind;

impl CommandArgumentKind for UserIdOrMentionArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<
        (&'a str, Option<Box<dyn std::any::Any + Send + Sync>>),
        async_brigadier::CommandParseError,
    > {
        let (id_or_mention, rest) = command.split_once(' ').unwrap_or((command, ""));
        Id::<UserMarker>::try_from(
            id_or_mention
                .trim_start()
                .strip_prefix("<@")
                .and_then(|input| input.strip_suffix('>'))
                .unwrap_or(id_or_mention),
        )
        .map_err(|_| CommandParseError::Other("Invalid user ID or mention"))
        .map(|value| (rest, Some(Box::new(value) as Box<dyn Any + Send + Sync>)))
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
        pass_empty: false,
    }
}

/*
pub struct DurationArgumentKind;

impl CommandArgumentKind for DurationArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<
        (&'a str, Option<Box<dyn std::any::Any + Send + Sync>>),
        async_brigadier::CommandParseError,
    > {
        let (duration, rest) = command.split_once(' ').unwrap_or((command, ""));
        let duration = parse_duration(duration);
        if let Some(duration) = duration {
            Ok((rest, Some(Box::new(duration))))
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
        pass_empty: false,
    }
}
*/

pub struct ExpiryArgumentKind;

impl CommandArgumentKind for ExpiryArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<
        (&'a str, Option<Box<dyn std::any::Any + Send + Sync>>),
        async_brigadier::CommandParseError,
    > {
        let (duration, rest) = command.split_once(' ').unwrap_or((command, ""));
        let duration = parse_duration(duration);
        if let Some(duration) = duration {
            Ok((
                rest,
                Some(Box::new(expiry_from_duration(Utc::now(), duration)?)),
            ))
        } else {
            Err(async_brigadier::CommandParseError::NoMatch)
        }
    }
}

#[must_use]
pub fn expiry<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(ExpiryArgumentKind),
        name: Some(name),
        executes: None,
        requires: None,
        pass_empty: false,
    }
}

pub struct WebhookUrlArgumentKind;

impl CommandArgumentKind for WebhookUrlArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<
        (&'a str, Option<Box<dyn std::any::Any + Send + Sync>>),
        async_brigadier::CommandParseError,
    > {
        let (url_str, rest) = command.split_once(' ').unwrap_or((command, ""));
        if let Some((webhook_id, webhook_token)) = parse_webhook_url(url_str) {
            Ok((rest, Some(Box::new((webhook_id, webhook_token.to_owned())))))
        } else {
            Err(CommandParseError::Other("Invalid webhook URL"))
        }
    }
}

#[must_use]
pub fn webhook_url<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(WebhookUrlArgumentKind),
        name: Some(name),
        executes: None,
        requires: None,
        pass_empty: false,
    }
}

pub struct ChannelIdOrMentionOrLinkArgumentKind;

impl CommandArgumentKind for ChannelIdOrMentionOrLinkArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<
        (&'a str, Option<Box<dyn std::any::Any + Send + Sync>>),
        async_brigadier::CommandParseError,
    > {
        let (id_or_mention, rest) = command.split_once(' ').unwrap_or((command, ""));
        if let Some((_, channel_id)) = parse_channel_mention_or_id_or_link(id_or_mention) {
            Ok((rest, Some(Box::new(channel_id))))
        } else {
            Err(CommandParseError::Other(
                "Invalid channel ID, channel mention or channel link",
            ))
        }
    }
}

#[must_use]
pub fn channel_id_or_mention_or_link<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(ChannelIdOrMentionOrLinkArgumentKind),
        name: Some(name),
        executes: None,
        requires: None,
        pass_empty: false,
    }
}

pub struct CaseIdArgumentKind;

impl CommandArgumentKind for CaseIdArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Option<Box<dyn Any + Send + Sync>>), CommandParseError> {
        let (word, rest) = command.split_once(' ').unwrap_or((command, ""));
        let Ok(case_id) = CaseId::from_str(word) else {
            return Err(CommandParseError::Other("Invalid case ID"));
        };
        Ok((rest, Some(Box::new(case_id))))
    }
}

#[must_use]
pub fn case_id<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(CaseIdArgumentKind),
        name: Some(name),
        executes: None,
        requires: None,
        pass_empty: false,
    }
}

pub struct EmptyArgumentKind;

impl CommandArgumentKind for EmptyArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Option<Box<dyn Any + Send + Sync>>), CommandParseError> {
        Ok((command, None))
    }
}

#[must_use]
pub fn empty<C, R>() -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(EmptyArgumentKind),
        name: None,
        executes: None,
        requires: None,
        pass_empty: true,
    }
}
