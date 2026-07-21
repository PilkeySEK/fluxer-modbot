#[derive(Debug)]
pub struct CommandError {
    pub kind: CommandErrorKind,
}

#[derive(Debug)]
pub enum CommandErrorKind {
    Parse(CommandParseError),
    NotExecutable,
    /// A function in `.requires()` returned `false`.
    RequirementNotSatisfied,
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
