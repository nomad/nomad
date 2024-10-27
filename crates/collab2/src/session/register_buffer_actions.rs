use std::collections::hash_map::Entry;

use nomad::autocmds::BufAddArgs;
use nomad::buf_attach::BufAttach;
use nomad::events::CursorEvent;
use nomad::{
    action_name,
    Action,
    ActionName,
    BufferId,
    ByteOffset,
    Event,
    Shared,
    ShouldDetach,
};
use nomad_server::Message;
use smallvec::SmallVec;
use tracing::error;

use super::{
    PeerSelection,
    PeerTooltip,
    SessionCtx,
    SyncCursor,
    SyncReplacement,
};
use crate::Collab;

pub(super) struct RegisterBufferActions {
    pub(super) message_tx: flume::Sender<Message>,
    pub(super) session_ctx: Shared<SessionCtx>,
}

impl RegisterBufferActions {
    pub(super) fn register_actions(&mut self, buffer_id: BufferId) {
        self.session_ctx.with_mut(|session_ctx| {
            let neovim_ctx = session_ctx.neovim_ctx.reborrow();

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
                .starts_with(session_ctx.project_root.as_str())
            {
                return;
            }

            let should_detach = Shared::new(ShouldDetach::No);

            match session_ctx.buffer_actions.entry(buffer_id) {
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
                session_ctx: self.session_ctx.clone(),
                should_detach: should_detach.clone(),
            })
            .register(text_buffer_ctx.reborrow());

            CursorEvent::new(SyncCursor {
                message_tx: self.message_tx.clone(),
                session_ctx: self.session_ctx.clone(),
                should_detach: should_detach.clone(),
            })
            .register((&**text_buffer_ctx).reborrow());

            let new_tooltips = session_ctx
                .replica
                .cursors()
                .filter_map(|cursor| {
                    if cursor.owner() == session_ctx.replica.id() {
                        return None;
                    }
                    let file_id = cursor.file().id();
                    let buffer_ctx = session_ctx.buffer_of_file_id(file_id)?;
                    if buffer_ctx.buffer_id() != buffer_id {
                        return None;
                    }
                    let peer = String::from("TODO: get peer");
                    let peer_tooltip = PeerTooltip::create(
                        peer,
                        cursor.byte_offset().into(),
                        buffer_ctx,
                    );
                    Some((cursor.id(), peer_tooltip))
                })
                .collect::<SmallVec<[_; 4]>>();

            for (cursor_id, tooltip) in new_tooltips {
                session_ctx.remote_tooltips.insert(cursor_id, tooltip);
            }

            let new_selections = session_ctx
                .replica
                .selections()
                .filter_map(|selection| {
                    if selection.owner() == session_ctx.replica.id() {
                        return None;
                    }
                    let file_id = selection.file().id();
                    let buffer_ctx = session_ctx.buffer_of_file_id(file_id)?;
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
                session_ctx.remote_selections.insert(selection_id, selection);
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

    fn execute(&mut self, args: Self::Args) {
        self.register_actions(args.buffer_id);
    }

    fn docs(&self) {}
}
