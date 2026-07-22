use std::any::Any;

use crate::{CommandExecuteFn, CommandParseError};

mod literal;
pub use literal::*;
mod string;
pub use string::*;

pub struct CommandArgument<C, R> {
    pub children: Vec<CommandArgument<C, R>>,
    pub kind: Box<dyn CommandArgumentKind>,
    pub name: Option<&'static str>,
    pub executes: Option<Box<dyn CommandExecuteFn<C, R>>>,
    #[expect(clippy::type_complexity)]
    pub requires: Option<Box<dyn Fn(&C) -> bool + Send + Sync>>,
}

pub trait CommandArgumentKind: Send + Sync {
    /// Parse this argument. On success, return `(rest, argument)`.
    ///
    /// # Errors
    /// Returns an error if parsing failed.
    fn parse<'a>(
        &self,
        command: &'a str,
    ) -> Result<(&'a str, Box<dyn Any + Send + Sync>), CommandParseError>;
}

impl<C, R> CommandArgument<C, R> {
    #[must_use]
    pub fn executes(mut self, f: impl CommandExecuteFn<C, R> + 'static) -> Self {
        self.executes = Some(Box::new(f));
        self
    }

    #[must_use]
    pub fn then(mut self, then: CommandArgument<C, R>) -> Self {
        self.children.push(then);
        self
    }

    #[must_use]
    pub fn requires(mut self, requires: impl Fn(&C) -> bool + Send + Sync + 'static) -> Self {
        self.requires = Some(Box::new(requires));
        self
    }
}
