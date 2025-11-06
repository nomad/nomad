use core::mem;

use editor::{AccessMut, AgentId, Editor, Shared};

use crate::Neovim;
use crate::buffer::{BufferExt, BufferId, NeovimBuffer};
use crate::events::{AutocmdId, Callbacks, Event, EventKind, Events};
use crate::oxi::api;
use crate::utils::CallbackExt;

#[derive(Debug, Clone, Copy)]
pub(crate) struct BufferCreated;

impl Event for BufferCreated {
    type Args<'a> = (NeovimBuffer<'a>, AgentId);
    type Container<'ev> = &'ev mut Option<Callbacks<Self>>;
    type RegisterOutput = [AutocmdId; 3];

    #[inline]
    fn container<'ev>(&self, events: &'ev mut Events) -> Self::Container<'ev> {
        &mut events.on_buffer_created
    }

    #[inline]
    fn key(&self) {}

    #[inline]
    fn kind(&self) -> EventKind {
        EventKind::BufferCreated(*self)
    }

    #[allow(clippy::too_many_lines)]
    #[inline]
    fn register(
        &self,
        events: &Events,
        mut nvim: impl AccessMut<Neovim> + 'static,
    ) -> Self::RegisterOutput {
        let just_added_buffer_id = Shared::<Option<BufferId>>::new(None);

        let on_buf_add = {
            let just_added_buffer_id = just_added_buffer_id.clone();
            move |args: api::types::AutocmdCallbackArgs| {
                just_added_buffer_id.set(Some(BufferId::from(args.buffer)));
                false
            }
        };

        let buf_add_autocmd_id = api::create_autocmd(
            ["BufAdd"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .callback(on_buf_add.into_function())
                .build(),
        )
        .expect("couldn't create autocmd");

        let old_name_was_empty = Shared::<bool>::new(false);

        let on_buf_file_pre = {
            let old_name_was_empty = old_name_was_empty.clone();
            move |args: api::types::AutocmdCallbackArgs| {
                old_name_was_empty.set(args.buffer.name().is_empty());
                false
            }
        };

        let buf_file_pre_autocmd_id = api::create_autocmd(
            ["BufFilePre"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .callback(on_buf_file_pre.into_function())
                .build(),
        )
        .expect("couldn't create autocmd");

        let callback = (move |args: api::types::AutocmdCallbackArgs| {
            let buffer_id = BufferId::from(args.buffer);

            // The buffer was created iff it was just added.
            if just_added_buffer_id.take() != Some(buffer_id) {
                return false;
            }

            // We should only treat buffer renames as creations if the old name
            // is empty. Renames from non-empty names should be skipped.
            if args.event == "BufFilePost" && !old_name_was_empty.take() {
                return false;
            }

            nvim.with_mut(|nvim| {
                let Some(mut buffer) = nvim.buffer(buffer_id) else {
                    return false;
                };

                let events = &mut buffer.nvim.events;

                let Some(callbacks) = &events.on_buffer_created else {
                    return true;
                };

                let created_by =
                    mem::take(&mut events.agent_ids.created_buffer);

                for callback in callbacks.cloned() {
                    callback((buffer.reborrow(), created_by));
                }

                false
            })
        })
        .catch_unwind()
        .map(|maybe_detach| maybe_detach.unwrap_or(true))
        .into_function();

        let autocmd_ids = api::create_autocmd(
            ["BufReadPost", "BufNewFile", "BufFilePost"],
            &api::opts::CreateAutocmdOpts::builder()
                .group(events.augroup_id)
                .callback(callback)
                .build(),
        )
        .expect("couldn't create autocmd");

        [buf_add_autocmd_id, buf_file_pre_autocmd_id, autocmd_ids]
    }

    #[inline]
    fn unregister(autocmd_ids: Self::RegisterOutput) {
        for autocmd_id in autocmd_ids {
            let _ = api::del_autocmd(autocmd_id);
        }
    }
}
