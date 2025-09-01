use editor::{AccessMut, AgentId, Edit, Editor, Replacement, Shared};
use nohash::IntMap as NoHashMap;
use smallvec::{SmallVec, smallvec_inline};

use crate::Neovim;
use crate::buffer::{BufferExt, BufferId, NeovimBuffer, Point};
use crate::events::{AutocmdId, Callbacks, Event, EventKind, Events};
use crate::option::UneditableEndOfLine;
use crate::oxi::api;
use crate::utils::CallbackExt;

const TRIGGER_AUTOCMD_PATTERN: &str = "BufferEditedEventTrigger";

#[derive(Debug, Copy, Clone)]
pub(crate) struct BufferEdited(pub(crate) BufferId);

/// The output of the [`BufferEdited::register`] method.
#[derive(Debug, Clone)]
pub(crate) struct BufferEditedRegisterOutput {
    autocmd_ids: [AutocmdId; 4],
    buffer_id: BufferId,
    queued_edits: Shared<SmallVec<[Edit; 2]>>,
}

impl BufferEditedRegisterOutput {
    pub(crate) fn enqueue(&self, edit: Edit) {
        self.queued_edits.with_mut(|vec| vec.push(edit));
    }

    pub(crate) fn trigger(&self) {
        let opts = api::opts::ExecAutocmdsOpts::builder()
            .buffer(self.buffer_id.into())
            .modeline(false)
            .patterns(TRIGGER_AUTOCMD_PATTERN)
            .build();

        api::exec_autocmds(["User"], &opts).expect("couldn't exec autocmd");
    }
}

impl Event for BufferEdited {
    type Args<'a> = (NeovimBuffer<'a>, &'a Edit);
    type Container<'ev> = &'ev mut NoHashMap<BufferId, Callbacks<Self>>;
    type RegisterOutput = BufferEditedRegisterOutput;

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_edited
    }

    #[inline]
    fn key(&self) -> BufferId {
        self.0
    }

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::BufferEdited(*self)
    }

    #[inline]
    fn register(
        &self,
        events: &Events,
        mut nvim: impl AccessMut<Neovim> + Clone + 'static,
    ) -> Self::RegisterOutput {
        let buffer_id = self.0;
        let queued_edits = Shared::<SmallVec<_>>::default();

        let mut on_edit = {
            let queued_edits = queued_edits.clone();
            move |edit: Edit| {
                nvim.with_mut(|nvim| {
                    let Some(mut buffer) = nvim.buffer(buffer_id) else {
                        panic!(
                            "callback triggered for an invalid buffer{}",
                            api::Buffer::from(buffer_id)
                                .get_name()
                                .map(|name| format!(": {name}"))
                                .unwrap_or_default()
                        );
                    };

                    let Some(callbacks) = buffer
                        .nvim
                        .events
                        .on_buffer_edited
                        .get(&buffer_id)
                        .map(|cbs| cbs.cloned())
                    else {
                        return true;
                    };

                    let queued_edits = queued_edits.take();

                    for callback in callbacks {
                        callback((buffer.reborrow(), &edit));
                        for edit in &queued_edits {
                            callback((buffer.reborrow(), &edit));
                        }
                    }

                    false
                })
            }
        };

        let on_bytes = {
            let queued_edits = queued_edits.clone();
            let mut on_edit = on_edit.clone();
            move |args: api::opts::OnBytesArgs| {
                let edit = queued_edits
                    .with_mut(|vec| (!vec.is_empty()).then(|| vec.remove(0)))
                    .unwrap_or_else(|| Edit {
                        made_by: AgentId::UNKNOWN,
                        replacements: smallvec_inline![
                            replacement_of_on_bytes(args)
                        ],
                    });

                on_edit(edit)
            }
        }
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        api::Buffer::from(buffer_id)
            .attach(
                false,
                &api::opts::BufAttachOpts::builder()
                    .on_bytes(on_bytes)
                    .build(),
            )
            .expect("couldn't attach to buffer");

        let on_fixeol_changed = {
            let mut on_edit = on_edit.clone();
            move |buffer: api::Buffer, old_value, new_value| {
                debug_assert!(BufferId::from(buffer.clone()) == buffer_id);

                let num_bytes = buffer.num_bytes();

                // Eol-settings don't apply on empty buffers.
                if num_bytes == 0 {
                    return false;
                }

                let replacement = match (old_value, new_value) {
                    // The trailing newline was deleted.
                    (true, false) => {
                        Replacement::deletion(num_bytes..num_bytes + 1)
                    },
                    (false, true) => {
                        Replacement::insertion(num_bytes - 1, "\n")
                    },
                    // The old value is the same as the new one.
                    _ => return false,
                };

                on_edit(Edit {
                    made_by: AgentId::UNKNOWN,
                    replacements: smallvec_inline![replacement],
                })
            }
        };

        let autocmd_ids = UneditableEndOfLine::on_set_on(
            buffer_id,
            events.augroup_id,
            on_fixeol_changed,
        );

        let on_manual_trigger = {
            let queued_edits = queued_edits.clone();
            move |_: api::types::AutocmdCallbackArgs| {
                queued_edits
                    .with_mut(|vec| (!vec.is_empty()).then(|| vec.remove(0)))
                    .map(|first| on_edit(first))
                    .unwrap_or(false)
            }
        }
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        let autocmd_id = api::create_autocmd(
            ["User"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .patterns([TRIGGER_AUTOCMD_PATTERN])
                .buffer(buffer_id.into())
                .callback(on_manual_trigger)
                .build(),
        )
        .expect("couldn't create autocmd");

        BufferEditedRegisterOutput {
            autocmd_ids: [
                autocmd_ids.0,
                autocmd_ids.1,
                autocmd_ids.2,
                autocmd_id,
            ],
            buffer_id,
            queued_edits,
        }
    }

    #[inline]
    fn unregister(output: Self::RegisterOutput) {
        for autocmd_id in output.autocmd_ids {
            let _ = api::del_autocmd(autocmd_id);
        }
    }
}

/// Converts the arguments given to the
/// [`on_bytes`](api::opts::BufAttachOptsBuilder::on_bytes) callback into
/// the corresponding [`Replacement`].
#[inline]
fn replacement_of_on_bytes(args: api::opts::OnBytesArgs) -> Replacement {
    let (
        _bytes,
        buffer,
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

    let deletion_start = start_offset;

    let deletion_end = start_offset + old_end_len;

    // Fast path for pure deletions.
    if new_end_len == 0 {
        return Replacement::deletion(deletion_start..deletion_end);
    }

    let insertion_start =
        Point { newline_offset: start_row, byte_offset: start_col };

    let insertion_end = Point {
        newline_offset: start_row + new_end_row,
        byte_offset: start_col * (new_end_row == 0) as usize + new_end_col,
    };

    Replacement::new(
        deletion_start..deletion_end,
        &*buffer.get_text_in_point_range(insertion_start..insertion_end),
    )
}
