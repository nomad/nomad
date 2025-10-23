//! TODO: docs.

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};
use core::ops::{self, Range};
use core::{fmt, iter};
use std::borrow::Cow;

use abs_path::AbsPath;
use editor::{
    AccessMut,
    AgentId,
    Buffer as _,
    ByteOffset,
    Chunks,
    Edit,
    Replacement,
};
use futures_util::FutureExt;
use smallvec::{SmallVec, smallvec_inline};

pub use crate::buffer_ext::{BufferExt, GraphemeOffsets};
use crate::convert::Convert;
use crate::cursor::NeovimCursor;
use crate::oxi::{self, BufHandle, api};
use crate::{Neovim, decoration_provider, events, utils};

/// TODO: docs.
pub struct NeovimBuffer<'a> {
    /// The inner buffer.
    inner: api::Buffer,

    /// The buffer's path.
    path: Cow<'a, AbsPath>,

    /// An exclusive reference to the Neovim instance.
    pub(crate) nvim: &'a mut Neovim,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BufferId(BufHandle);

/// TODO: docs.
pub struct HighlightRange<'a> {
    buffer: api::Buffer,
    handle: &'a HighlightRangeHandle,
}

/// TODO: docs.
pub struct HighlightRangeHandle {
    inner: decoration_provider::HighlightRange,
}

/// The 2D equivalent of a [`ByteOffset`] in a buffer.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Point {
    /// The number of `\n` characters before this point.
    pub newline_offset: usize,

    /// The byte offset in the line.
    pub byte_offset: ByteOffset,
}

impl<'a> NeovimBuffer<'a> {
    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn highlight_range(
        &self,
        byte_range: Range<ByteOffset>,
        highlight_group_name: &str,
    ) -> HighlightRangeHandle {
        debug_assert!(byte_range.start <= byte_range.end);
        debug_assert!(byte_range.end <= self.byte_len());
        let start = self.point_of_byte(byte_range.start);
        let end = self.point_of_byte(byte_range.end);
        HighlightRangeHandle {
            inner: self.nvim.decoration_provider.highlight_range(
                self.id(),
                start..end,
                highlight_group_name,
            ),
        }
    }

    /// Returns an iterator over the `(byte_range, hl_groups)` tuples of all
    /// highlight ranges set on this buffer.
    #[inline]
    pub fn highlight_ranges(
        &self,
    ) -> impl Iterator<Item = (Range<ByteOffset>, SmallVec<[String; 1]>)> {
        let opts = api::opts::GetExtmarksOpts::builder()
            .details(true)
            .ty("highlight")
            .build();

        self.get_extmarks(
            api::types::GetExtmarksNamespaceId::All,
            Point::zero().into(),
            self.point_of_eof().into(),
            &opts,
        )
        .expect("couldn't get extmarks")
        .map(|(_ns_id, start_row, start_col, maybe_infos)| {
            let infos = maybe_infos.expect("requested details");
            let end_row = infos.end_row.expect("set for hl marks");
            let end_col = infos.end_col.expect("set for hl marks");
            let hl_group = infos.hl_group.expect("set for hl marks");

            let start_point = Point::new(start_row, start_col);
            let end_point = Point::new(end_row, end_col);
            let start = self.byte_of_point(start_point);
            let end = self.byte_of_point(end_point);
            (start..end, hl_group.convert())
        })
    }

    #[inline]
    pub(crate) fn new(id: BufferId, nvim: &'a mut Neovim) -> Option<Self> {
        let inner = api::Buffer::from(id);

        if !inner.is_loaded() {
            return None;
        }

        let buftype = api::get_option_value::<oxi::String>(
            "buftype",
            &api::opts::OptionOpts::builder().buf(inner.clone()).build(),
        )
        .ok()?;

        if !buftype.is_empty() {
            return None;
        }

        let path = inner.name().to_str().ok()?.parse().ok()?;

        Some(Self { inner, path: Cow::Owned(path), nvim })
    }

    #[inline]
    pub(crate) fn reborrow(&mut self) -> NeovimBuffer<'_> {
        NeovimBuffer {
            inner: self.inner.clone(),
            path: Cow::Borrowed(&*self.path),
            nvim: self.nvim,
        }
    }
}

impl<'a> HighlightRange<'a> {
    /// Returns the buffer this highlight range is on.
    #[inline]
    pub fn buffer(&self) -> api::Buffer {
        self.buffer.clone()
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn r#move(&self, byte_range: Range<ByteOffset>) {
        debug_assert!(byte_range.start <= byte_range.end);
        debug_assert!(byte_range.end <= self.buffer.num_bytes());
        let start = self.buffer.point_of_byte(byte_range.start);
        let end = self.buffer.point_of_byte(byte_range.end);
        self.handle.inner.r#move(start..end);
    }

    /// TODO: docs.
    #[inline]
    pub fn set_highlight_group(&self, highlight_group_name: &str) {
        self.handle.inner.set_hl_group(highlight_group_name);
    }

    #[inline]
    pub(crate) fn new(
        buffer: api::Buffer,
        handle: &'a HighlightRangeHandle,
    ) -> Self {
        debug_assert_eq!(BufferId(buffer.handle()), handle.buffer_id());
        Self { buffer, handle }
    }
}

impl HighlightRangeHandle {
    #[inline]
    pub(crate) fn buffer_id(&self) -> BufferId {
        self.inner.buffer_id()
    }
}

impl Point {
    /// Creates a new `Point`.
    #[inline]
    pub fn new(newline_offset: usize, byte_offset: usize) -> Self {
        Self { newline_offset, byte_offset }
    }

    #[inline]
    pub(crate) fn zero() -> Self {
        Self::new(0, 0)
    }
}

impl<'a> editor::Buffer for NeovimBuffer<'a> {
    type Editor = Neovim;

    #[inline]
    fn byte_len(&self) -> ByteOffset {
        self.inner.num_bytes()
    }

    #[inline]
    fn get_text_range(&self, byte_range: Range<ByteOffset>) -> impl Chunks {
        let start = self.point_of_byte(byte_range.start);
        let end = self.point_of_byte(byte_range.end);
        self.get_text_in_point_range(start..end)
    }

    #[inline]
    fn id(&self) -> BufferId {
        self.inner.clone().into()
    }

    #[inline]
    fn for_each_cursor<Fun>(&mut self, mut fun: Fun)
    where
        Fun: FnMut(NeovimCursor),
    {
        if self.is_focused() {
            fun(NeovimCursor::from(self.reborrow()));
        }
    }

    #[inline]
    fn on_edited<Fun>(
        &mut self,
        mut fun: Fun,
        nvim: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> events::EventHandle
    where
        Fun: FnMut(&NeovimBuffer, &Edit) + 'static,
    {
        self.nvim.events.insert(
            events::BufferEdited(self.id()),
            move |(this, edit)| fun(&this, edit),
            nvim,
        )
    }

    #[inline]
    fn on_removed<Fun>(
        &mut self,
        mut fun: Fun,
        nvim: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> events::EventHandle
    where
        Fun: FnMut(BufferId, AgentId) + 'static,
    {
        self.nvim.events.insert(
            events::BufferRemoved(self.id()),
            move |(buffer_id, removed_by)| fun(buffer_id, removed_by),
            nvim,
        )
    }

    #[inline]
    fn on_saved<Fun>(
        &mut self,
        mut fun: Fun,
        nvim: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> events::EventHandle
    where
        Fun: FnMut(&NeovimBuffer, AgentId) + 'static,
    {
        let buffer_id = self.id();
        self.nvim.events.insert(
            events::BufWritePost(buffer_id),
            move |(this, saved_by)| fun(&this, saved_by),
            nvim,
        )
    }

    #[inline]
    fn path(&self) -> Cow<'_, AbsPath> {
        Cow::Borrowed(&self.path)
    }

    #[inline]
    fn schedule_edit<R>(
        &mut self,
        replacements: R,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static
    where
        R: IntoIterator<Item = Replacement>,
    {
        let buffer_id = self.id();

        let replacements = replacements
            .into_iter()
            .filter(|repl| !repl.is_no_op())
            .collect::<SmallVec<[_; 1]>>();

        let buffer_edited = self
            .nvim
            .events
            .on_buffer_edited
            .get(&buffer_id)
            .map(|callbacks| callbacks.register_output().clone());

        // We schedule this because editing text in the buffer will immediately
        // trigger an OnBytes event, which would panic due to a double mutable
        // borrow of Neovim.
        utils::schedule(move || {
            let buffer_edited = buffer_edited.as_ref();

            let mut buffer = api::Buffer::from(buffer_id);

            for replacement in replacements {
                if let Some(buffer_edited) = buffer_edited {
                    buffer_edited.enqueue(Edit {
                        made_by: agent_id,
                        replacements: smallvec_inline![replacement.clone()],
                    });
                }
                apply_replacement(&mut buffer, replacement, buffer_edited);
            }
        })
    }

    #[inline]
    fn schedule_focus(
        &mut self,
        agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        let buffer = self.inner.clone();

        if let Some(callbacks) = &mut self.nvim.events.on_cursor_created {
            callbacks.register_output_mut().set_created_by(agent_id);
        }

        // We schedule this because setting the current window's buffer will
        // immediately trigger a BufEnter event, which would panic due to a
        // double mutable borrow of Neovim.
        utils::schedule(move || buffer.focus())
    }

    #[inline]
    fn schedule_save(
        &mut self,
        _agent_id: AgentId,
    ) -> impl Future<
        Output = Result<(), <Self::Editor as editor::Editor>::BufferSaveError>,
    > + 'static {
        // We schedule this because writing the buffer will immediately trigger
        // a BufWritePost event, which would panic due to a double mutable
        // borrow of Neovim.
        //
        // TODO: save agent ID.
        let buffer = self.buffer();
        utils::schedule(move || {
            buffer
                .call(|()| {
                    api::command("silent keepjumps keepalt write")
                        .expect("couldn't save buffer")
                })
                .expect("couldn't run command in buffer");
        })
        .map(|()| Ok(()))
    }
}

impl ops::Deref for NeovimBuffer<'_> {
    type Target = api::Buffer;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ops::DerefMut for NeovimBuffer<'_> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl BufferExt for NeovimBuffer<'_> {
    #[inline]
    fn buffer(&self) -> api::Buffer {
        self.inner.clone()
    }
}

#[allow(clippy::too_many_lines)]
fn apply_replacement(
    buffer: &mut api::Buffer,
    replacement: Replacement,
    buffer_edited: Option<&events::BufferEditedRegisterOutput>,
) {
    debug_assert!(!replacement.is_no_op());

    let deletion_range = replacement.removed_range();
    let insert_text = replacement.inserted_text();

    debug_assert!(deletion_range.start <= deletion_range.end);
    debug_assert!(deletion_range.end <= buffer.num_bytes());

    let deletion_start = buffer.point_of_byte(deletion_range.start);
    let deletion_end = buffer.point_of_byte(deletion_range.end);

    if !buffer.is_point_after_uneditable_eol(deletion_end) {
        apply_replacement_whose_deletion_ends_before_fixeol(
            buffer,
            deletion_start..deletion_end,
            deletion_range.len(),
            insert_text,
            buffer_edited,
        );
        return;
    }

    // The replacement is a pure insertion past the fixeol.
    if deletion_start == deletion_end {
        apply_insertion_after_fixeol(
            buffer,
            deletion_start,
            insert_text,
            buffer_edited,
        );
        return;
    }

    // The replacement is a pure deletion.
    if insert_text.is_empty() {
        apply_deletion_ending_after_fixeol(
            buffer,
            deletion_start..deletion_end,
            deletion_range.len(),
            buffer_edited,
        );
        return;
    }

    // Clamp the end of the deleted range to the end of the previous line.
    let clamped_end = Point {
        newline_offset: deletion_end.newline_offset - 1,
        byte_offset: buffer
            .num_bytes_in_line_after(deletion_end.newline_offset - 1),
    };

    // We've clamped the end of the deletion range, so it's now 1 byte shorter.
    let deletion_len = deletion_range.len() - 1;

    // If the text ends with a newline, we can remove the newline and clamp the
    // end of the deleted range to the previous point.
    //
    // For example, if the buffer is "Hello\n", the replacement is delete 4..6
    // and insert "!\n", then we can delete 4..5 and insert "!" instead.
    if let Some(stripped) = insert_text.strip_suffix('\n') {
        apply_replacement_whose_deletion_ends_before_fixeol(
            buffer,
            deletion_start..clamped_end,
            deletion_len,
            stripped,
            buffer_edited,
        );
        return;
    }

    // Enqueue the re-insertion of the newline that this replacement deletes.
    if let Some(buffer_edited) = buffer_edited {
        let len_after_edit =
            buffer.num_bytes() - deletion_range.len() + insert_text.len();
        let re_insert_newline = Replacement::insertion(len_after_edit, "\n");
        buffer_edited.enqueue(Edit {
            made_by: AgentId::UNKNOWN,
            replacements: smallvec_inline![re_insert_newline],
        });
    }

    apply_replacement_whose_deletion_ends_before_fixeol(
        buffer,
        deletion_start..clamped_end,
        deletion_len,
        insert_text,
        buffer_edited,
    );
}

#[allow(clippy::too_many_arguments)]
fn apply_replacement_whose_deletion_ends_before_fixeol(
    buffer: &mut api::Buffer,
    delete_range: Range<Point>,
    deletion_len: ByteOffset,
    insert_text: &str,
    buffer_edited: Option<&events::BufferEditedRegisterOutput>,
) {
    debug_assert!(!buffer.is_point_after_uneditable_eol(delete_range.end));
    debug_assert!(!(delete_range.is_empty() && insert_text.is_empty()));
    debug_assert_eq!(
        deletion_len,
        buffer.byte_of_point(delete_range.end)
            - buffer.byte_of_point(delete_range.start)
    );

    let lines = insert_text
        .lines()
        // If the text has a trailing newline, Neovim expects an additional
        // empty line to be included.
        .chain(insert_text.ends_with('\n').then_some(""));

    if let Some(buffer_edited) = buffer_edited
        && buffer.has_uneditable_eol()
    {
        // If the buffer goes from empty to not empty, the trailing EOL
        // "activates".
        if buffer.is_empty() {
            let insert_newline =
                Replacement::insertion(insert_text.len(), "\n");
            buffer_edited.enqueue(Edit {
                made_by: AgentId::UNKNOWN,
                replacements: smallvec_inline![insert_newline],
            });
        }

        // Viceversa, if the buffer goes from not empty to empty, the trailing
        // EOL "deactivates".
        if deletion_len + 1 == buffer.num_bytes() && insert_text.is_empty() {
            let delete_newline = Replacement::deletion(0..1);
            buffer_edited.enqueue(Edit {
                made_by: AgentId::UNKNOWN,
                replacements: smallvec_inline![delete_newline],
            });
        }
    }

    buffer
        .set_text(
            delete_range.start.newline_offset..delete_range.end.newline_offset,
            delete_range.start.byte_offset,
            delete_range.end.byte_offset,
            lines,
        )
        .expect("replacing text failed");
}

fn apply_insertion_after_fixeol(
    buffer: &mut api::Buffer,
    insert_point: Point,
    insert_text: &str,
    buffer_edited: Option<&events::BufferEditedRegisterOutput>,
) {
    debug_assert!(buffer.is_point_after_uneditable_eol(insert_point));
    debug_assert!(!insert_text.is_empty());

    if !insert_text.ends_with('\n') {
        // Enqueue the insertion of a newline after the inserted text.
        if let Some(buffer_edited) = buffer_edited {
            let len_after_edit = buffer.num_bytes() + insert_text.len();
            let insert_newline = Replacement::insertion(len_after_edit, "\n");
            buffer_edited.enqueue(Edit {
                made_by: AgentId::UNKNOWN,
                replacements: smallvec_inline![insert_newline],
            });
        }
    }

    let num_newlines = insert_point.newline_offset;

    buffer
        .set_lines(num_newlines..num_newlines, true, insert_text.lines())
        .expect("couldn't insert lines");
}

fn apply_deletion_ending_after_fixeol(
    buffer: &mut api::Buffer,
    Range { start, end }: Range<Point>,
    deletion_len: ByteOffset,
    buffer_edited: Option<&events::BufferEditedRegisterOutput>,
) {
    debug_assert!(start < end);
    debug_assert!(buffer.is_point_after_uneditable_eol(end));
    debug_assert_eq!(
        deletion_len,
        buffer.byte_of_point(end) - buffer.byte_of_point(start)
    );

    // If the start of the deletion range is after a newline (or at the
    // start of the buffer), we can just delete the last n lines.
    if start.byte_offset == 0 {
        let line_range = start.newline_offset..end.newline_offset;
        buffer
            .set_lines(line_range, true, iter::empty::<&str>())
            .expect("couldn't set lines");
        return;
    }

    let clamped_end = Point {
        newline_offset: end.newline_offset - 1,
        byte_offset: buffer.num_bytes_in_line_after(end.newline_offset - 1),
    };

    // We've clamped the end of the deletion range, so it's 1 byte shorter.
    let deletion_len = deletion_len - 1;

    if clamped_end == start {
        let Some(buffer_edited) = buffer_edited else { return };
        let num_bytes = buffer.num_bytes();
        let insert_newline = Replacement::insertion(num_bytes - 1, "\n");
        buffer_edited.enqueue(Edit {
            made_by: AgentId::UNKNOWN,
            replacements: smallvec_inline![insert_newline],
        });
        buffer_edited.trigger();
        return;
    }

    // Enqueue the re-insertion of the newline that this deletion deletes.
    if let Some(buffer_edited) = buffer_edited {
        let len_after_edit = buffer.num_bytes() - deletion_len;
        let re_insert_newline = Replacement::insertion(len_after_edit, "\n");
        buffer_edited.enqueue(Edit {
            made_by: AgentId::UNKNOWN,
            replacements: smallvec_inline![re_insert_newline],
        });
    }

    apply_replacement_whose_deletion_ends_before_fixeol(
        buffer,
        start..clamped_end,
        deletion_len,
        "",
        buffer_edited,
    );
}

impl From<api::Buffer> for BufferId {
    #[inline]
    fn from(buf: api::Buffer) -> Self {
        Self(buf.handle())
    }
}

impl Hash for BufferId {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_i32(self.0);
    }
}

impl nohash::IsEnabled for BufferId {}

impl From<BufferId> for api::Buffer {
    #[inline]
    fn from(buf_id: BufferId) -> Self {
        buf_id.0.into()
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Point")
            .field(&self.newline_offset)
            .field(&self.byte_offset)
            .finish()
    }
}

impl PartialOrd for Point {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Point {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.newline_offset
            .cmp(&other.newline_offset)
            .then(self.byte_offset.cmp(&other.byte_offset))
    }
}

impl From<Point> for api::types::ExtmarkPosition {
    #[inline]
    fn from(point: Point) -> Self {
        Self::ByTuple((point.newline_offset, point.byte_offset))
    }
}
