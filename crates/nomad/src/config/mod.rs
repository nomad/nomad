use nvim::Object;

use super::EnableConfig;
use crate::ctx::{Ctx, Set};
use crate::prelude::{nvim, Module};

/// TODO: docs
pub(crate) fn config() -> nvim::Function<Object, ()> {
    todo!();
}

/// TODO: docs
#[inline]
pub(crate) fn with_module<M>(set_config: Set<EnableConfig<M>>, ctx: &Ctx)
where
    M: Module,
{
    todo!();
}
