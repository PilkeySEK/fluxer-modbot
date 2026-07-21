use crate::arg::{CommandArgument, CommandArgumentKind};

pub enum StringArgumentKind {
    Word,
    Greedy,
}

impl CommandArgumentKind for StringArgumentKind {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Box<dyn std::any::Any + Send + Sync>), crate::CommandParseError> {
        Ok(match self {
            Self::Word => {
                let (word, rest) = command.split_once(' ').unwrap_or((command, ""));
                (rest, Box::new(String::from(word)))
            }
            Self::Greedy => ("", Box::new(String::from(command))),
        })
    }
}

#[must_use]
pub fn word<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(StringArgumentKind::Word),
        name: Some(name),
        executes: None,
        requires: None,
    }
}

#[must_use]
pub fn greedy_string<C, R>(name: &'static str) -> CommandArgument<C, R> {
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(StringArgumentKind::Greedy),
        name: Some(name),
        executes: None,
        requires: None,
    }
}
