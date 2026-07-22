mod dispatcher;
use std::{
    any::Any,
    collections::HashMap,
    ops::{Deref, DerefMut},
    pin::Pin,
};

pub use dispatcher::*;
mod node;
pub use node::*;
pub mod arg;
mod error;
pub use error::*;

pub trait CommandExecuteFn<C, R>: Send + Sync {
    fn call(&self, context: CommandContext<C>) -> Pin<Box<dyn Future<Output = R> + Send>>;
}

pub struct CommandContext<C> {
    pub context: C,
    pub args: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
}

impl<F, Fut, C, R> CommandExecuteFn<C, R> for F
where
    F: Fn(CommandContext<C>) -> Fut + Send + Sync,
    Fut: Future<Output = R> + Send + 'static,
{
    fn call(&self, context: CommandContext<C>) -> Pin<Box<dyn Future<Output = R> + Send>> {
        Box::pin(self(context))
    }
}

impl<C> Deref for CommandContext<C> {
    type Target = C;
    fn deref(&self) -> &Self::Target {
        &self.context
    }
}

impl<C> DerefMut for CommandContext<C> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.context
    }
}

impl<C> CommandContext<C> {
    /// Gets the argument. This can only be called once per argument name.
    ///
    /// # Panics
    /// Panics if an argument with that name does not exist or it is not of type `T`.
    #[expect(clippy::unwrap_used)]
    pub fn take_argument<T: 'static>(&mut self, name: &'static str) -> T {
        let arg = self.args.remove(&name).unwrap();
        let arg = arg.downcast().unwrap();
        *arg
    }

    /// Gets the argument. This can only be called once per argument name.
    /// Returns `None` if there is no argument with that name. If it returns `Some(...)`, you need
    /// to downcast the argument to a concrete type yourself.
    pub fn try_take_argument(&mut self, name: &'static str) -> Option<Box<dyn Any + Send + Sync>> {
        self.args.remove(&name)
    }

    /// Gets a reference to the argument. This can be called multiple times for the same argument name.
    ///
    /// # Panics
    /// Panics if an argument with that name does not exist or it is not of type `T`.
    #[expect(clippy::unwrap_used)]
    pub fn get_argument<T: 'static>(&mut self, name: &'static str) -> &T {
        self.args.get(&name).unwrap().downcast_ref().unwrap()
    }

    /// Gets a reference to the argument. This can be called multiple times for the same argument name.
    /// Returns `None` if there is no argument with that name. If it returns `Some(...)`, you need
    /// to downcast the argument to a concrete type yourself.
    pub fn try_get_argument(&mut self, name: &'static str) -> Option<&Box<dyn Any + Send + Sync>> {
        self.args.get(&name)
    }
}
