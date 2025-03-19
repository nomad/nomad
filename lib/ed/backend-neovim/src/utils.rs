//! Utility functions for interacting with Neovim's Lua environment.

use crate::oxi::mlua;

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
