use editor::{AccessMut, Edit, Editor, Replacement};
use nohash::IntMap as NoHashMap;
use smallvec::smallvec_inline;

use crate::Neovim;
use crate::buffer::{BufferExt, BufferId, NeovimBuffer, Point};
use crate::events::{Callbacks, Event, EventKind, Events};
use crate::oxi::api;
use crate::utils::CallbackExt;

#[derive(Debug, Clone, Copy)]
pub(crate) struct OnBytes(pub(crate) BufferId);

impl Event for OnBytes {
    type Args<'a> = (NeovimBuffer<'a>, &'a Edit);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = ();

    #[inline]
    fn container<'ev>(
        &self,
        _events: &'ev mut Events,
    ) -> Self::Container<'ev> {
        todo!();
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::OnBytes(*self)
    }

    #[inline]
    fn register(
        &self,
        _: &Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
    ) {
        let buffer_id = self.0;

        let callback = (move |args: api::opts::OnBytesArgs| {
            nvim.with_mut(|nvim| {
                let Some(callbacks) = nvim
                    .events
                    .on_buffer_edited
                    .get(&buffer_id)
                    .map(|cbs| cbs.cloned())
                else {
                    return true;
                };

                let edited_by = nvim.events.agent_ids.edited_buffer.take();

                let should_extend_end_by_one = nvim
                    .buffers_state
                    .on_bytes_replacement_extend_deletion_end_by_one
                    .take();

                let should_start_at_next_line = nvim
                    .buffers_state
                    .on_bytes_replacement_insertion_starts_at_next_line
                    .take();

                let buf = api::Buffer::from(buffer_id);

                let Some(mut buffer) = nvim.buffer(buffer_id) else {
                    tracing::error!(
                        buffer_name = %buf.name(),
                        "OnBytes triggered for an invalid buffer",
                    );
                    return true;
                };

                let replacement = replacement_of_on_bytes(
                    buf,
                    args,
                    should_extend_end_by_one,
                    should_start_at_next_line,
                );

                let edit = Edit {
                    made_by: edited_by,
                    replacements: smallvec_inline![replacement],
                };

                for callback in callbacks {
                    callback((buffer.reborrow(), &edit));
                }

                false
            })
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::Buffer::from(buffer_id)
            .attach(
                false,
                &api::opts::BufAttachOpts::builder()
                    .on_bytes(callback)
                    .build(),
            )
            .expect("couldn't attach to buffer");
    }

    #[inline]
    fn unregister((): Self::RegisterOutput) {}
}

/// Converts the arguments given to the
/// [`on_bytes`](api::opts::BufAttachOptsBuilder::on_bytes) callback into
/// the corresponding [`Replacement`].
#[inline]
fn replacement_of_on_bytes(
    buffer: impl BufferExt,
    args: api::opts::OnBytesArgs,
    should_extend_end_by_one: bool,
    should_start_at_next_line: bool,
) -> Replacement {
    let (
        _bytes,
        buf,
        _changedtick,
        start_row,
        start_col,
        start_offset,
        _old_end_row,
        _old_end_col,
        old_end_len,
        new_end_row,
        new_end_col,
        new_end_len,
    ) = args;

    debug_assert_eq!(buf, buffer.buffer());

    let should_extend_start =
        should_extend_end_by_one && should_start_at_next_line;

    let deletion_start = start_offset + should_extend_start as usize;

    let deletion_end =
        start_offset + old_end_len + should_extend_end_by_one as usize;

    // Fast path for pure deletions.
    if new_end_len == 0 {
        return Replacement::deletion(deletion_start..deletion_end);
    }

    let mut insertion_start =
        Point { newline_offset: start_row, byte_offset: start_col };

    if should_start_at_next_line {
        insertion_start.newline_offset += 1;
        insertion_start.byte_offset = 0;
    }

    let insertion_end = Point {
        newline_offset: start_row + new_end_row,
        byte_offset: start_col * (new_end_row == 0) as usize + new_end_col,
    };

    Replacement::new(
        deletion_start..deletion_end,
        &*buffer.get_text_in_point_range(insertion_start..insertion_end),
    )
}
