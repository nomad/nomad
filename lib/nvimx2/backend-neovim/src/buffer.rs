use std::borrow::Cow;
use std::path::PathBuf;

use nvimx_core::ByteOffset;
use nvimx_core::backend::Buffer;

use crate::oxi;

/// TODO: docs.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct NeovimBuffer(oxi::BufHandle);

#[derive(Debug, Copy, Clone, PartialEq)]
struct Point {
    /// The index of the line in the buffer.
    line_idx: usize,

    /// The byte offset in the line.
    byte_offset: ByteOffset,
}

impl NeovimBuffer {
    /// Returns this buffer's handle.
    #[inline]
    pub fn handle(self) -> oxi::BufHandle {
        self.0
    }

    #[inline]
    pub(crate) fn current() -> Self {
        Self::new(oxi::api::Buffer::current())
    }

    #[inline]
    pub(crate) fn exists(self) -> bool {
        self.inner().is_valid()
    }

    #[inline]
    pub(crate) fn get_name(self) -> PathBuf {
        debug_assert!(self.exists());
        self.inner().get_name().expect("buffer exists")
    }

    #[inline]
    pub(crate) fn new(inner: oxi::api::Buffer) -> Self {
        Self(inner.handle())
    }

    /// Converts the given [`Point`] to the corresponding [`ByteOffset`] in the
    /// buffer.
    #[track_caller]
    fn byte_offset_of_point(self, point: Point) -> ByteOffset {
        let line_offset: ByteOffset = self
            .inner()
            .get_offset(point.line_idx)
            .expect("couldn't get line offset")
            .into();
        line_offset + point.byte_offset
    }

    /// Returns the [`Point`] at the end of the buffer.
    fn point_of_eof(self) -> Point {
        fn point_of_eof(
            buffer: NeovimBuffer,
        ) -> Result<Point, oxi::api::Error> {
            let nvim_buf = buffer.inner();

            let num_lines = nvim_buf.line_count()?;

            if num_lines == 0 {
                return Ok(Point::zero());
            }

            let last_line_len = nvim_buf.get_offset(num_lines)?
            // `get_offset(line_count)` seems to always include the trailing
            // newline, even when `eol` is turned off.
            //
            // TODO: shouldn't we only correct this if `eol` is turned off?
            - 1
            - nvim_buf.get_offset(num_lines - 1)?;

            Ok(Point {
                line_idx: num_lines - 1,
                byte_offset: ByteOffset::new(last_line_len),
            })
        }

        point_of_eof(self).expect("not deleted")
    }

    #[inline]
    fn inner(&self) -> oxi::api::Buffer {
        self.handle().into()
    }
}

impl Point {
    /// TODO: docs.
    fn zero() -> Self {
        Self { line_idx: 0, byte_offset: ByteOffset::new(0) }
    }
}

impl Buffer for NeovimBuffer {
    type Id = Self;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        self.byte_offset_of_point(self.point_of_eof())
    }

    #[inline]
    fn id(&self) -> Self::Id {
        *self
    }

    #[inline]
    fn name(&self) -> Cow<'_, str> {
        self.get_name().to_string_lossy().into_owned().into()
    }
}

#[cfg(feature = "mlua")]
impl oxi::mlua::IntoLua for NeovimBuffer {
    #[inline]
    fn into_lua(
        self,
        lua: &oxi::mlua::Lua,
    ) -> oxi::mlua::Result<oxi::mlua::Value> {
        self.handle().into_lua(lua)
    }
}
