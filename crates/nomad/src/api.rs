//! TODO: docs

use nvim::{self, Object};
use serde::de::Deserialize;

use crate::prelude::*;

/// TODO: docs
pub struct Api<M: Module> {
    pub(crate) functions: Functions,
    pub(crate) module: M,
}

impl<M: Module> Api<M> {
    /// TODO: docs
    #[inline]
    pub fn new(module: M) -> Self {
        Self { functions: Functions::default(), module }
    }

    /// TODO: docs
    #[inline]
    pub fn with_command<A>(self, _action: A) -> Self
    where
        A: Action<M>,
        A::Args: TryFrom<CommandArgs>,
        <A::Args as TryFrom<CommandArgs>>::Error: Into<WarningMsg>,
    {
        self
    }

    /// TODO: docs
    #[inline]
    pub fn with_function<A>(mut self, action: A) -> Self
    where
        A: Action<M>,
    {
        self.functions.push(action);
        self
    }
}

type Function = Box<dyn Fn(Object, &mut SetCtx)>;

/// TODO: docs
#[derive(Default)]
pub(crate) struct Functions {
    functions: Vec<(ActionName, Function)>,
}

impl Functions {
    /// TODO: docs
    #[inline]
    pub(crate) fn into_iter(
        self,
        ctx: Ctx,
    ) -> impl Iterator<Item = (&'static str, nvim::Function<Object, ()>)> {
        self.functions.into_iter().map(move |(name, function)| {
            let ctx = ctx.clone();

            let function = nvim::Function::from_fn(move |object: Object| {
                ctx.with_set(|set_ctx| function(object, set_ctx));
                Ok::<_, core::convert::Infallible>(())
            });

            (name.as_str(), function)
        })
    }

    /// TODO: docs
    #[inline]
    fn push<M: Module, A: Action<M>>(&mut self, action: A) {
        let function = move |args: Object, ctx: &mut SetCtx| {
            let deserializer = nvim::serde::Deserializer::new(args);
            let args = A::Args::deserialize(deserializer).unwrap();
            action.execute(args, ctx);
        };

        self.functions.push((A::NAME, Box::new(function)));
    }
}

/// TODO: docs
pub struct CommandArgs {}
