//! Utility functions for interacting with Neovim's Lua environment.

use std::panic;

use crate::oxi::{self, lua, mlua};

/// Returns whether the given module is available, i.e. whether in can be
/// `require(..)`d.
#[inline]
pub fn is_module_available(module_name: &str) -> bool {
    #[inline]
    fn inner(module_name: &str) -> mlua::Result<bool> {
        let lua = mlua::lua();
        let pcall = lua.globals().get::<mlua::Function>("pcall")?;
        let require = lua.globals().get::<mlua::Function>("require")?;
        let (is_available, _) =
            pcall.call::<(bool, mlua::Value)>((require, module_name))?;
        Ok(is_available)
    }
    inner(module_name).unwrap_or_default()
}

pub(crate) trait CallbackExt<T, U>:
    FnMut(T) -> U + Sized + 'static
{
    /// TODO: docs.
    fn catch_unwind(mut self) -> impl CallbackExt<T, Option<U>> {
        move |arg: T| match panic::catch_unwind(panic::AssertUnwindSafe(
            || self(arg),
        )) {
            Ok(result) => Some(result),

            Err(err) => {
                let payload = err
                    .downcast_ref::<String>()
                    .map(|s| &**s)
                    .or_else(|| err.downcast_ref::<&str>().copied());

                tracing::error!(
                    payload = ?payload,
                    "callback panicked",
                );

                None
            },
        }
    }

    /// TODO: docs.
    fn map<U2>(
        mut self,
        mut f: impl FnMut(U) -> U2 + 'static,
    ) -> impl CallbackExt<T, U2> {
        move |arg: T| f(self(arg))
    }

    /// TODO: docs.
    fn into_function(self) -> oxi::Function<T, U>
    where
        T: lua::Poppable,
        U: lua::Pushable,
    {
        oxi::Function::from_fn_mut(self)
    }
}

impl<F, T, U> CallbackExt<T, U> for F where F: FnMut(T) -> U + 'static {}
