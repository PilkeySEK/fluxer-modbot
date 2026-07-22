use crate::arg::{CommandArgument, CommandArgumentKind};

pub struct OptionalArgumentKind<C, R>(CommandArgument<C, R>);

impl<C, R> CommandArgumentKind for OptionalArgumentKind<C, R> {
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Option<Box<dyn std::any::Any + Send + Sync>>), crate::CommandParseError>
    {
        if !command.is_empty()
            && let Ok(parsed) = self.0.kind.parse(command)
        {
            Ok(parsed)
        } else {
            Ok((command, None))
        }
    }
}

#[must_use]
pub fn optional<C: 'static, R: 'static>(argument: CommandArgument<C, R>) -> CommandArgument<C, R> {
    let name = argument.name;
    CommandArgument {
        children: Vec::new(),
        kind: Box::new(OptionalArgumentKind(argument)),
        name,
        executes: None,
        requires: None,
        pass_empty: true,
    }
}
