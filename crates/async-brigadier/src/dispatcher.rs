use std::collections::HashMap;

use crate::{
    CommandContext, CommandError, CommandErrorKind, CommandNode, CommandParseError,
    arg::CommandArgument,
};

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
        let matched_node = parse_args_recursively(command_trimmed, &self.root_children, &context)
            .map_err(|e| CommandError { kind: e })?;
        let Some(executes) = matched_node.executes else {
            return Err(CommandError {
                kind: CommandErrorKind::NotExecutable,
            });
        };
        Ok(executes
            .call(CommandContext {
                context,
                args: matched_node.args,
            })
            .await)
    }
}

fn parse_args_recursively<'a, C, R>(
    rest: &str,
    args: &'a Vec<CommandArgument<C, R>>,
    ctx: &C,
) -> Result<CommandNode<'a, C, R>, CommandErrorKind> {
    for arg in args {
        if !arg.pass_empty && rest.is_empty() {
            continue;
        }
        if let Ok((rest, value)) = arg.kind.parse(rest) {
            if let Some(requires) = &arg.requires
                && !requires(ctx)
            {
                return Err(CommandErrorKind::RequirementNotSatisfied);
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
    Err(CommandErrorKind::Parse(CommandParseError::NoMatch))
}

impl<C, R> Default for CommandDispatcher<C, R> {
    fn default() -> Self {
        Self {
            root_children: Vec::new(),
        }
    }
}
