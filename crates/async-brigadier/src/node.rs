use std::{any::Any, collections::HashMap};

use crate::CommandExecuteFn;

pub struct CommandNode<'a, C, R> {
    pub args: HashMap<&'static str, Box<dyn Any + Send + Sync>>,
    pub executes: Option<&'a dyn CommandExecuteFn<C, R>>,
}
