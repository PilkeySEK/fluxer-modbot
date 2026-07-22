use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub enum CommandError {
    Parse(CommandParseError),
    NotExecutable,
    /// A function in `.requires()` returned `false`.
    RequirementNotSatisfied,
    Other(&'static str),
    OtherString(String),
}

#[derive(Debug)]
pub enum CommandParseError {
    /// The pattern expected by the parser did not match.
    /// For example, the literal value expected by a literal was not
    /// present or the number expected by a number parser contained
    /// non-numeric characters.
    NoMatch,
    Other(String),
}

impl Error for CommandError {}

impl Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CommandError::NotExecutable => f.write_str("Not executable at this node"),
            CommandError::Parse(e) => {
                f.write_str("Parse error: ")?;
                e.fmt(f)
            }
            CommandError::RequirementNotSatisfied => f.write_str("Requirement not satisfied"),
            CommandError::Other(s) => s.fmt(f),
            CommandError::OtherString(s) => s.fmt(f),
        }
    }
}

impl Error for CommandParseError {}

impl Display for CommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoMatch => f.write_str("No arguments matched"),
            Self::Other(s) => s.fmt(f),
        }
    }
}
