use std::borrow::Cow;
use std::path::PathBuf;

use nvimx_core::backend::Buffer;

use crate::Neovim;

/// TODO: docs.
#[derive(Clone, PartialEq, Eq)]
pub struct NeovimBuffer(crate::oxi::api::Buffer);

impl NeovimBuffer {
    #[inline]
    pub(crate) fn current() -> Self {
        Self(crate::oxi::api::Buffer::current())
    }

    #[inline]
    pub(crate) fn exists(&self) -> bool {
        self.0.is_valid()
    }

    #[inline]
    pub(crate) fn get_name(&self) -> PathBuf {
        debug_assert!(self.exists());
        self.0.get_name().expect("buffer exists")
    }
}

impl Buffer<Neovim> for NeovimBuffer {
    type Id = Self;

    #[inline]
    fn id(&self) -> Self::Id {
        self.clone()
    }

    #[inline]
    fn name(&self) -> Cow<'_, str> {
        self.get_name().to_string_lossy().into_owned().into()
    }
}

#[cfg(feature = "mlua")]
impl crate::oxi::mlua::IntoLua for NeovimBuffer {
    #[inline]
    fn into_lua(
        self,
        lua: &crate::oxi::mlua::Lua,
    ) -> crate::oxi::mlua::Result<crate::oxi::mlua::Value> {
        self.0.handle().into_lua(lua)
    }
}
