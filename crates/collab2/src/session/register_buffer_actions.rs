use std::collections::hash_map::Entry;

use collab_server::message::Message;
use nomad::autocmds::BufAddArgs;
use nomad::buf_attach::BufAttach;
use nomad::ctx::NeovimCtx;
use nomad::events::CursorEvent;
use nomad::{
    action_name,
    Action,
    ActionName,
    BufferId,
    Event,
    Shared,
    ShouldDetach,
};
use smallvec::SmallVec;
use tracing::error;

use super::{
    PeerSelection,
    PeerTooltip,
    Project,
    SyncCursor,
    SyncReplacement,
};
use crate::Collab;

pub(super) struct RegisterBufferActions {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) project: Shared<Project>,
}

impl RegisterBufferActions {
    #[allow(clippy::too_many_lines)]
    pub(super) fn register_actions(&mut self, buffer_id: BufferId) {
        self.project.with_mut(|project| {
            let neovim_ctx = project.neovim_ctx.reborrow();

            // Check if the buffer is a text file.
            let Some(text_file_ctx) = neovim_ctx
                .into_buffer(buffer_id)
                .and_then(|ctx| ctx.into_text_buffer())
                .and_then(|ctx| ctx.into_text_file())
            else {
                return;
            };

            // Check if the buffer is in the project root.
            if !text_file_ctx
                .as_file()
                .path()
                .starts_with(project.project_root.as_str())
            {
                return;
            }

            let should_detach = Shared::new(ShouldDetach::No);

            match project.buffer_actions.entry(buffer_id) {
                Entry::Vacant(entry) => {
                    entry.insert(should_detach.clone());
                },
                Entry::Occupied(_) => {
                    error!(
                        "trying to register actions on a buffer that's \
                         already tracked"
                    );
                    return;
                },
            }

            let text_buffer_ctx = text_file_ctx.as_text_buffer();

            BufAttach::new(SyncReplacement {
                message_tx: self.message_tx.clone(),
                project: self.project.clone(),
                should_detach: should_detach.clone(),
            })
            .register(text_buffer_ctx.reborrow());

            CursorEvent::new(SyncCursor {
                message_tx: self.message_tx.clone(),
                project: self.project.clone(),
                should_detach: should_detach.clone(),
            })
            .register((**text_buffer_ctx).reborrow());

            let new_tooltips = project
                .replica
                .cursors()
                .filter_map(|cursor| {
                    let peer = project
                        .remote_peers
                        .get(&cursor.owner().id())
                        .cloned()?;
                    let file_id = cursor.file().id();
                    let buffer_ctx = project.buffer_of_file_id(file_id)?;
                    if buffer_ctx.buffer_id() != buffer_id {
                        return None;
                    }
                    let peer_tooltip = PeerTooltip::create(
                        peer,
                        cursor.byte_offset().into(),
                        buffer_ctx,
                    );
                    Some((cursor.id(), peer_tooltip))
                })
                .collect::<SmallVec<[_; 4]>>();

            for (cursor_id, tooltip) in new_tooltips {
                project.remote_tooltips.insert(cursor_id, tooltip);
            }

            let new_selections = project
                .replica
                .selections()
                .filter_map(|selection| {
                    if !project
                        .remote_peers
                        .contains_key(&selection.owner().id())
                    {
                        return None;
                    }
                    let file_id = selection.file().id();
                    let buffer_ctx = project.buffer_of_file_id(file_id)?;
                    if buffer_ctx.buffer_id() != buffer_id {
                        return None;
                    }
                    let selection_range = {
                        let r = selection.byte_range();
                        r.start.into()..r.end.into()
                    };
                    let peer_selection =
                        PeerSelection::create(selection_range, buffer_ctx);
                    Some((selection.id(), peer_selection))
                })
                .collect::<SmallVec<[_; 4]>>();

            for (selection_id, selection) in new_selections {
                project.remote_selections.insert(selection_id, selection);
            }
        });
    }
}

impl Action for RegisterBufferActions {
    const NAME: ActionName = action_name!("register-buffer-actions");
    type Args = BufAddArgs;
    type Docs = ();
    type Module = Collab;
    type Return = ();

    fn execute(&mut self, args: Self::Args, _: NeovimCtx<'static>) {
        self.register_actions(args.buffer_id);
    }

    fn docs(&self) {}
}
