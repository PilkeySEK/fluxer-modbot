use crate::{
    CommandParseError,
    arg::{CommandArgument, CommandArgumentKind},
};

pub struct LiteralArgumentKind {
    literal: &'static str,
}

impl CommandArgumentKind for LiteralArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Box<dyn std::any::Any + Send + Sync>), crate::CommandParseError> {
        let Some(rest) = command.strip_prefix(self.literal) else {
            return Err(CommandParseError::NoMatch);
        };
        Ok((rest, Box::new(())))
    }
}

#[must_use]
pub fn literal<C, R>(literal: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(LiteralArgumentKind { literal }),
        name: None,
        executes: None,
        requires: None,
    }
}
