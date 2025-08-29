//! TODO: docs.

use core::cmp::Ordering;
use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::{self, Range};
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
    Shared,
};
use futures_util::FutureExt;
use smallvec::{SmallVec, smallvec_inline};

pub use crate::buffer_ext::BufferExt;
use crate::convert::Convert;
use crate::cursor::NeovimCursor;
use crate::option::{BufferLocalOpts, NeovimOption, UneditableEndOfLine};
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

#[derive(Clone, Default)]
pub(crate) struct BuffersState {
    /// TODO: docs.
    pub(crate) on_bytes_replacement_extend_deletion_end_by_one: Shared<bool>,

    /// TODO: docs.
    pub(crate) on_bytes_replacement_insertion_starts_at_next_line:
        Shared<bool>,

    /// TODO: docs.
    pub(crate) skip_next_uneditable_eol: Shared<bool>,
}

/// The 2D equivalent of a [`ByteOffset`] in a buffer.
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Point {
    /// The index of the line in the buffer.
    pub line_idx: usize,

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
        debug_assert!(byte_range.end <= self.inner.byte_len());
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
        debug_assert!(byte_range.end <= self.buffer.byte_len());
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
    pub fn new(line_idx: usize, byte_offset: usize) -> Self {
        Self { line_idx, byte_offset }
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
        self.inner.byte_len()
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

    #[allow(clippy::too_many_lines)]
    #[inline]
    fn on_edited<Fun>(
        &mut self,
        fun: Fun,
        nvim: impl AccessMut<Self::Editor> + Clone + 'static,
    ) -> events::EventHandle
    where
        Fun: FnMut(&NeovimBuffer, &Edit) + 'static,
    {
        let fun = Shared::<Fun>::new(fun);
        let buffer_id = self.id();

        let fun2 = fun.clone();
        let on_bytes_handle = self.nvim.events.insert(
            events::OnBytes(buffer_id),
            move |(this, edit)| {
                fun2.with_mut(|fun| fun(&this, edit));

                if this.has_uneditable_eol() {
                    // If the buffer has an uneditable eol, then:
                    //
                    // - if the buffer was empty and text is inserted the
                    // eol "activates", and we should notify the user that
                    // a \n was inserted;
                    //
                    // - if all the text is deleted and the buffer is now
                    // empty the eol "deactivates", and we should notify
                    // the user that a \n was deleted;

                    let buf_len = this.inner.byte_len();
                    let edit_len_delta = edit.byte_delta();
                    let edit_len_delta_abs = edit_len_delta.unsigned_abs();

                    let replacement = if edit_len_delta.is_positive()
                        && edit_len_delta_abs + 1 == buf_len
                    {
                        Replacement::insertion(edit_len_delta_abs, "\n")
                    } else if edit_len_delta.is_negative() && buf_len == 0 {
                        Replacement::deletion(0..1)
                    } else {
                        return;
                    };

                    let edit = Edit {
                        made_by: AgentId::UNKNOWN,
                        replacements: smallvec_inline![replacement],
                    };

                    fun2.with_mut(|fun| fun(&this, &edit));
                }
            },
            nvim.clone(),
        );

        // Setting/unsetting the uneditable eol behaves as if
        // deleting/inserting a trailing newline, so we need to react to it.

        let uneditable_eol_set_handle = self.nvim.events.insert(
            events::SetUneditableEndOfLine,
            move |(buf, was_set, is_set, set_by)| {
                // Ignore event if setting didn't change, if it changed for a
                // different buffer or if we were told to skip this event.
                if was_set == is_set
                    || buf.id() != buffer_id
                    || buf.nvim.buffers_state.skip_next_uneditable_eol.take()
                {
                    return;
                }

                let byte_len = buf.inner.byte_len();

                // Eol-settings don't apply on empty buffers.
                if byte_len == 0 {
                    return;
                }

                let replacement = match (was_set, is_set) {
                    // The trailing newline was deleted.
                    (true, false) => {
                        Replacement::deletion(byte_len..byte_len + 1)
                    },
                    // The trailing newline was added.
                    (false, true) => {
                        Replacement::insertion(byte_len - 1, "\n")
                    },
                    _ => unreachable!("already checked"),
                };

                let edit = Edit {
                    made_by: set_by,
                    replacements: smallvec_inline![replacement],
                };

                fun.with_mut(|fun| fun(&buf, &edit));
            },
            nvim,
        );

        on_bytes_handle.merge(uneditable_eol_set_handle)
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
        let buffer_id = self.id();
        self.nvim.events.insert(
            events::BufferRemoved(buffer_id),
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

        let is_on_bytes_subscribed_to =
            self.nvim.events.contains(&events::OnBytes(buffer_id));

        let buffers_state = self.nvim.buffers_state.clone();

        let edited_buffer_agent_id =
            self.nvim.events.agent_ids.edited_buffer.clone();

        let set_uneditable_eol_agent_id =
            self.nvim.events.agent_ids.set_uneditable_eol.clone();

        // We schedule this because editing text in the buffer will immediately
        // trigger an OnBytes event, which would panic due to a double mutable
        // borrow of Neovim.
        utils::schedule(move || {
            let mut buffer = api::Buffer::from(buffer_id);

            for replacement in replacements {
                let range = replacement.removed_range();
                let deletion_start = buffer.point_of_byte(range.start);
                let deletion_end = buffer.point_of_byte(range.end);
                replace_text_in_point_range(
                    &mut buffer,
                    deletion_start..deletion_end,
                    replacement.inserted_text(),
                    agent_id,
                    is_on_bytes_subscribed_to,
                    &buffers_state,
                    &edited_buffer_agent_id,
                    &set_uneditable_eol_agent_id,
                )
            }
        })
    }

    #[inline]
    fn schedule_focus(
        &mut self,
        _agent_id: AgentId,
    ) -> impl Future<Output = ()> + 'static {
        let buffer = self.inner.clone();

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
        utils::schedule(|| {
            api::command("write").expect("saving buffer failed");
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

/// Replaces the text in the given point range with the new text.
///
/// # Panics
///
/// Panics if the replacement is a no-op, i.e. if both the range to delete and
/// the text to insert are empty.
#[allow(clippy::too_many_arguments)]
#[track_caller]
#[inline]
fn replace_text_in_point_range(
    buffer: &mut api::Buffer,
    mut delete_range: Range<Point>,
    insert_text: &str,
    agent_id: AgentId,
    is_on_bytes_subscribed_to: bool,
    buffers_state: &BuffersState,
    edited_buffer_agent_id: &Shared<AgentId>,
    set_uneditable_eol_agent_id: &events::SetUneditableEolAgentIds,
) {
    debug_assert!(delete_range.start <= delete_range.end);
    debug_assert!(delete_range.end <= buffer.point_of_eof());
    debug_assert!(!delete_range.is_empty() || !insert_text.is_empty());

    // If the buffer has an uneditable eol, we might need to clamp the
    // points of the deleted range in the same way we do in
    // get_text_in_point_range(). See that comment for more details.

    let should_clamp_end =
        buffer.is_point_after_uneditable_eol(delete_range.end);

    if should_clamp_end {
        let end = &mut delete_range.end;
        end.line_idx -= 1;
        end.byte_offset = buffer.byte_len_of_line(end.line_idx);
    }

    let should_clamp_start = delete_range.start > delete_range.end;

    if should_clamp_start {
        // The original start was <= than the end, so if we need to clamp
        // the start it means we just clamped the end.
        debug_assert!(should_clamp_end);
        delete_range.start = delete_range.end;
    }

    // If we needed to clamp the end of the range it means the user wants
    // to delete the trailing newline or insert text after it.
    //
    // However, Neovim made the unfortunate design decision of assuming
    // that every buffer ends in `\n`, and all the buffer-editing APIs will
    // return an error if you try to set the end position of the deleted
    // range past it.
    //
    // The only way to get around this is to unset the uneditable eol,
    // which acts as if the trailing newline was deleted, even marking the
    // buffer as "modified".
    //
    // The drawback of this approach is that the trailing newline won't be
    // re-inserted the next time the buffer is saved, unless the user
    // manually re-enables either "eol", "fixeol", or both.
    //
    // While this sucks, it sucks less than not respecting the user's
    // intent.
    let should_unset_uneditable_eol = should_clamp_end;

    // If we clamped the start it means the replacement was a pure
    // insertion after the uneditable eol (e.g. buffer contains "Hello\n",
    // replacement is delete 6..6, insert "World").
    let insert_after_uneditable_eol = should_clamp_start;

    let is_on_bytes_triggered = is_on_bytes_subscribed_to
        && (!delete_range.is_empty() || !insert_text.is_empty());

    if is_on_bytes_triggered {
        edited_buffer_agent_id.set(agent_id);
    }

    if should_unset_uneditable_eol {
        // We're about to:
        //
        // 1) unset the buffer's uneditable eol setting, which will
        //    trigger the SetUneditableEndOfLine event;
        //
        // 2) call set_text(), which will trigger the OnBytes event;
        //
        // Since both events are triggered by the same replacement, the
        // edit event handlers should only be called once, so we skip the
        // next UneditableEndOfLine event if OnBytes is triggered.

        if is_on_bytes_triggered {
            edited_buffer_agent_id.set(agent_id);

            buffers_state.skip_next_uneditable_eol.set(true);

            // Extend the end of the deleted range by one byte to account
            // for having deleted the trailing newline.
            buffers_state
                .on_bytes_replacement_extend_deletion_end_by_one
                .set(true);

            if insert_after_uneditable_eol {
                // Make the inserted text start at the next line to ignore
                // the newline that we're about to re-add.
                buffers_state
                    .on_bytes_replacement_insertion_starts_at_next_line
                    .set(true);
            }
        } else {
            // OnBytes is not triggered, so set the AgentId that removed
            // the UneditableEndOfLine because we won't skip the next event
            // on it.
            set_uneditable_eol_agent_id.set(agent_id);
        }

        UneditableEndOfLine.set(false, &BufferLocalOpts::new(buffer.clone()));
    }

    let lines =
            // To insert after the uneditable eol we first had to disable it,
            // so we need to re-add a newline to the buffer to balance it out.
            insert_after_uneditable_eol.then_some("").into_iter()
            .chain(insert_text.lines())
            // If the text has a trailing newline, Neovim expects an additional
            // empty line to be included.
            .chain(insert_text.ends_with('\n').then_some(""));

    buffer
        .set_text(
            delete_range.start.line_idx..delete_range.end.line_idx,
            delete_range.start.byte_offset,
            delete_range.end.byte_offset,
            lines,
        )
        .expect("replacing text failed");
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
            .field(&self.line_idx)
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
        self.line_idx
            .cmp(&other.line_idx)
            .then(self.byte_offset.cmp(&other.byte_offset))
    }
}

impl From<Point> for api::types::ExtmarkPosition {
    #[inline]
    fn from(point: Point) -> Self {
        Self::ByTuple((point.line_idx, point.byte_offset))
    }
}
