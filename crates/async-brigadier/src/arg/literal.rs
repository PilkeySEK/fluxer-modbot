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
    ) -> Result<(&'a str, Option<Box<dyn std::any::Any + Send + Sync>>), crate::CommandParseError>
    {
        let Some(rest) = command.strip_prefix(self.literal) else {
            return Err(CommandParseError::NoMatch);
        };
        Ok((rest, None))
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
        pass_empty: false,
    }
}

pub struct MultiLiteralArgumentKind {
    literals: Vec<&'static str>,
}

impl CommandArgumentKind for MultiLiteralArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Option<Box<dyn std::any::Any + Send + Sync>>), CommandParseError> {
        for literal in &self.literals {
            if let Some(rest) = command.strip_prefix(literal) {
                return Ok((rest, None));
            }
        }
        Err(CommandParseError::NoMatch)
    }
}

#[must_use]
pub fn multi_literal<C, R>(literals: Vec<&'static str>) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(MultiLiteralArgumentKind { literals }),
        name: None,
        executes: None,
        requires: None,
        pass_empty: false,
    }
}
