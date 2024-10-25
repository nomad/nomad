mod detach_buffer_actions;
mod peer_tooltip;
mod register_buffer_actions;
mod session_ctx;
mod sync_cursor;
mod sync_replacement;

use detach_buffer_actions::DetachBufferActions;
use futures_util::{
    pin_mut,
    select,
    FutureExt,
    Sink,
    SinkExt,
    Stream,
    StreamExt,
};
use nomad::autocmds::{BufAdd, BufUnload};
use nomad::ctx::NeovimCtx;
use nomad::{Action, BufferId, Event, Shared};
use nomad_server::Message;
use register_buffer_actions::RegisterBufferActions;
use session_ctx::SessionCtx;
use sync_cursor::SyncCursor;
use sync_replacement::SyncReplacement;

/// TODO: docs.
pub(crate) struct Session {
    neovim_ctx: NeovimCtx<'static>,
    session_ctx: Shared<SessionCtx>,
}

impl Session {
    pub(crate) fn new() -> Self {
        todo!();
    }

    pub(crate) async fn run<Tx, Rx>(&mut self, remote_tx: Tx, remote_rx: Rx)
    where
        Tx: Sink<Message, Error = core::convert::Infallible>,
        Rx: Stream<Item = Message>,
    {
        let (local_tx, local_rx) = flume::unbounded();

        let mut register_buffer_actions = RegisterBufferActions {
            message_tx: local_tx.clone(),
            session_ctx: self.session_ctx.clone(),
        };

        let detach_buffer_actions = DetachBufferActions {
            message_tx: local_tx,
            session_ctx: self.session_ctx.clone(),
        };

        for buffer_id in BufferId::opened() {
            register_buffer_actions.register_actions(buffer_id);
        }

        BufAdd::new(register_buffer_actions)
            .register(self.neovim_ctx.reborrow());

        BufUnload::new(detach_buffer_actions)
            .register(self.neovim_ctx.reborrow());

        pin_mut!(remote_rx);
        pin_mut!(remote_tx);

        loop {
            select! {
                remote_message = remote_rx.next().fuse() => {
                    if let Some(remote_message) = remote_message {
                        println!("{:?}", remote_message);
                    }
                },
                local_message = local_rx.recv_async().fuse() => {
                    if let Ok(local_message) = local_message {
                        remote_tx
                            .send(local_message)
                            .await
                            .expect("Infallible");
                    }
                },
            }
        }
    }
}
