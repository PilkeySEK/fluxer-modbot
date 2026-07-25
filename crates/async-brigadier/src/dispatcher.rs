use std::collections::HashMap;

use crate::{CommandContext, CommandError, CommandNode, CommandParseError, arg::CommandArgument};

pub struct CommandDispatcher<C, R> {
    root_children: Vec<CommandArgument<C, R>>,
}

impl<C, R> CommandDispatcher<C, R> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, arg: CommandArgument<C, R>) {
        self.root_children.push(arg);
    }

    /// Execute the given command.
    ///
    /// # Errors
    /// Returns an error if the command has invalid syntax.
    pub async fn execute(&self, command: &str, context: C) -> Result<R, CommandError> {
        let command_trimmed = command.trim_start();
        let matched_node = parse_args_recursively(command_trimmed, &self.root_children, &context)?;
        let Some(executes) = matched_node.executes else {
            return Err(CommandError::NotExecutable);
        };
        Ok(executes
            .call(CommandContext {
                context,
                args: matched_node.args,
            })
            .await)
    }
}

/// Same as `parse_args_recursively` except accepts a single `CommandArgument` initially.
#[expect(clippy::missing_errors_doc)]
pub fn parse_arg_recursively<'a, C, R>(
    rest: &str,
    arg: &'a CommandArgument<C, R>,
    ctx: &C,
) -> Result<CommandNode<'a, C, R>, CommandError> {
    if !arg.pass_empty && rest.is_empty() {
        return Err(CommandError::Parse(CommandParseError::NoMatch));
    }
    if let Ok((rest, value)) = arg.kind.parse(rest) {
        if let Some(requires) = &arg.requires
            && !requires(ctx)
        {
            return Err(CommandError::RequirementNotSatisfied);
        }
        let rest = rest.trim_start();
        if rest.is_empty() && arg.children.iter().find(|arg| arg.pass_empty).is_none() {
            let mut hashmap = HashMap::new();
            hashmap.insert(arg.name, value);
            return Ok(CommandNode {
                args: hashmap
                    .into_iter()
                    .filter_map(|(k, v)| match (k, v) {
                        (Some(k), Some(v)) => Some((k, v)),
                        _ => None,
                    })
                    .collect(),
                executes: arg.executes.as_deref(),
            });
        }
        let mut node = parse_args_recursively(rest, &arg.children, ctx)?;
        if let Some(name) = arg.name
            && let Some(value) = value
        {
            node.args.insert(name, value);
        }
        return Ok(node);
    }

    Err(CommandError::Parse(CommandParseError::NoMatch))
}

/// Parse the passed command recursively using the provided available arguments,
/// until the input is fully parsed. Returns the last node in the chain which holds
/// all previous arguments parsed from the command.
///
/// # Errors
/// Returns an error if parsing fails or a `.requires()` call returns `false`.
pub fn parse_args_recursively<'a, C, R>(
    rest: &str,
    args: &'a Vec<CommandArgument<C, R>>,
    ctx: &C,
) -> Result<CommandNode<'a, C, R>, CommandError> {
    for arg in args {
        if !arg.pass_empty && rest.is_empty() {
            continue;
        }
        if let Ok((rest, value)) = arg.kind.parse(rest) {
            if let Some(requires) = &arg.requires
                && !requires(ctx)
            {
                return Err(CommandError::RequirementNotSatisfied);
            }
            let rest = rest.trim_start();
            if rest.is_empty() && arg.children.iter().find(|arg| arg.pass_empty).is_none() {
                let mut hashmap = HashMap::new();
                hashmap.insert(arg.name, value);
                return Ok(CommandNode {
                    args: hashmap
                        .into_iter()
                        .filter_map(|(k, v)| match (k, v) {
                            (Some(k), Some(v)) => Some((k, v)),
                            _ => None,
                        })
                        .collect(),
                    executes: arg.executes.as_deref(),
                });
            }
            let mut node = parse_args_recursively(rest, &arg.children, ctx)?;
            if let Some(name) = arg.name
                && let Some(value) = value
            {
                node.args.insert(name, value);
            }
            return Ok(node);
        }
    }
    Err(CommandError::Parse(CommandParseError::NoMatch))
}

impl<C, R> Default for CommandDispatcher<C, R> {
    fn default() -> Self {
        Self {
            root_children: Vec::new(),
        }
    }
}
