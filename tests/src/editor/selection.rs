use core::mem;
use core::ops::Range;

use editor::{Buffer, Context, Editor, Selection};
use futures_util::stream::FusedStream;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum SelectionEvent {
    Created(Range<usize>),
    Moved(Range<usize>),
    Removed,
}

impl SelectionEvent {
    /// Returns a never-ending [`Stream`] of [`SelectionEvent`]s.
    #[track_caller]
    pub(crate) fn new_stream<Ed: Editor>(
        ctx: &mut Context<Ed>,
    ) -> impl FusedStream<Item = Self> + Unpin + use<Ed> {
        let (tx, rx) = flume::unbounded();
        let editor = ctx.editor();

        let buffer_id = ctx.with_borrowed(|ctx| {
            ctx.current_buffer().expect("no current buffer").id()
        });

        mem::forget(ctx.on_selection_created(
            move |selection, _created_by| {
                if selection.buffer_id() != buffer_id {
                    return;
                }

                let byte_range = selection.byte_range();
                let _ = tx.send(Self::Created(byte_range));

                let tx2 = tx.clone();
                mem::forget(selection.on_moved(
                    move |selection, _moved_by| {
                        let byte_range = selection.byte_range();
                        let _ = tx2.send(Self::Moved(byte_range));
                    },
                    editor.clone(),
                ));

                let tx2 = tx.clone();
                mem::forget(selection.on_removed(
                    move |_selection_id, _removed_by| {
                        let _ = tx2.send(Self::Removed);
                    },
                    editor.clone(),
                ));
            },
        ));

        rx.into_stream()
    }
}
