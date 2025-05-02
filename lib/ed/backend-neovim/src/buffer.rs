use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::Range;
use std::borrow::Cow;
use std::path::PathBuf;

use ed_core::ByteOffset;
use ed_core::backend::{AgentId, Buffer, Edit};

use crate::autocmd::EventHandle;
use crate::{Neovim, oxi};

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
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
    pub(crate) fn is_focused(self) -> bool {
        oxi::api::Window::current().get_buf().expect("window is valid")
            == self.inner()
    }

    #[inline]
    pub(crate) fn new(inner: oxi::api::Buffer) -> Self {
        Self(inner.handle())
    }

    #[inline]
    pub(crate) fn selection(&self) -> Option<Range<ByteOffset>> {
        let mode = oxi::api::get_mode().expect("couldn't get mode").mode;

        if !(mode.is_visual() || mode.is_visual_select()) {
            return None;
        }

        let (anchor_row, anchor_col) = self.inner().get_mark('<').ok()?;

        let (head_row, head_col) = self.inner().get_mark('>').ok()?;

        let anchor = self.byte_offset_of_point(Point {
            line_idx: anchor_row - 1,
            byte_offset: ByteOffset::new(anchor_col),
        });

        let head = self.byte_offset_of_point(Point {
            line_idx: head_row - 1,
            byte_offset: ByteOffset::new(head_col),
        });

        match anchor.cmp(&head) {
            Ordering::Less => Some(anchor..head),
            Ordering::Equal => None,
            Ordering::Greater => Some(head..anchor),
        }
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
    type Backend = Neovim;
    type EventHandle = EventHandle;
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

    #[inline]
    fn on_edited<Fun>(&mut self, _fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self, &Edit) + 'static,
    {
        todo!();
    }

    #[inline]
    fn on_removed<Fun>(&mut self, _fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self, AgentId) + 'static,
    {
        todo!();
    }

    #[inline]
    fn on_saved<Fun>(&mut self, _fun: Fun) -> Self::EventHandle
    where
        Fun: FnMut(&Self, AgentId) + 'static,
    {
        todo!();
    }
}

impl From<NeovimBuffer> for oxi::api::Buffer {
    #[inline]
    fn from(buf: NeovimBuffer) -> Self {
        buf.inner()
    }
}

impl oxi::mlua::IntoLua for NeovimBuffer {
    #[inline]
    fn into_lua(
        self,
        lua: &oxi::mlua::Lua,
    ) -> oxi::mlua::Result<oxi::mlua::Value> {
        self.handle().into_lua(lua)
    }
}

impl Hash for NeovimBuffer {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.handle());
    }
}

impl nohash::IsEnabled for NeovimBuffer {}
