use std::{any::Any, marker::PhantomData, str::FromStr};

use crate::arg::{CommandArgument, CommandArgumentKind};

pub struct FromStrArgumentKind<T: FromStr + Any + Send + Sync> {
    _marker: PhantomData<T>,
}

impl<T: FromStr + Any + Send + Sync> CommandArgumentKind for FromStrArgumentKind<T> {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Option<Box<dyn std::any::Any + Send + Sync>>), crate::CommandParseError>
    {
        let (word, rest) = command.split_once(' ').unwrap_or((command, ""));
        if let Ok(t) = T::from_str(word) {
            Ok((rest, Some(Box::new(t))))
        } else {
            Err(crate::CommandParseError::NoMatch)
        }
    }
}

#[must_use]
pub fn from_str<T: FromStr + Any + Send + Sync, C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(FromStrArgumentKind {
            _marker: PhantomData::<T>,
        }),
        name: Some(name),
        executes: None,
        requires: None,
        pass_empty: false,
    }
}
