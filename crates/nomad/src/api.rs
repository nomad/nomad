use serde::de::Deserialize;

use crate::action::Action;
use crate::action_name::ActionName;
use crate::nvim::{self, Object};
use crate::prelude::SetCtx;

/// TODO: docs
#[derive(Default)]
pub struct Api {
    #[allow(clippy::type_complexity)]
    functions: Vec<(ActionName, Box<dyn Fn(Object, &mut SetCtx)>)>,
}

impl Api {
    /// TODO: docs
    #[inline]
    pub(crate) fn into_iter(
        self,
    ) -> impl Iterator<Item = (ActionName, Box<dyn Fn(Object, &mut SetCtx)>)>
    {
        self.functions.into_iter()
    }

    /// TODO: docs
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// TODO: docs
    #[inline]
    pub fn with_function<A: Action>(mut self, action: A) -> Self {
        let function = move |args: Object, ctx: &mut SetCtx| {
            let deserializer = nvim::serde::Deserializer::new(args);
            let args = A::Args::deserialize(deserializer).unwrap();
            action.execute(args, ctx);
        };

        self.functions.push((A::NAME, Box::new(function)));

        self
    }
}
