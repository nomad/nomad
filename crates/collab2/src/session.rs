use core::marker::PhantomData;

use collab_server::SessionId;
use collab_server::message::{Peer, Peers};
use eerie::Replica;
use futures_util::{FutureExt, SinkExt, StreamExt, pin_mut, select};
use nvimx2::fs::AbsPathBuf;
use nvimx2::{AsyncCtx, notify};

use crate::CollabBackend;

pub(crate) struct Session<B: CollabBackend> {
    server_tx: B::ServerTx,
    server_rx: B::ServerRx,
}

pub(crate) struct NewSessionArgs<B: CollabBackend> {
    /// Whether the [`local_peer`](Self::local_peer) is the host of the
    /// session.
    pub(crate) _is_host: bool,

    /// The local [`Peer`].
    pub(crate) _local_peer: Peer,

    /// The remote [`Peers`].
    pub(crate) _remote_peers: Peers,

    /// The absolute path to the directory containing the project.
    ///
    /// The contents of the directory are assumed to be in sync with with the
    /// [`replica`](Self::replica).
    pub(crate) _project_root: AbsPathBuf,

    /// The [`replica`](Self::replica) of the project.
    ///
    /// The files and directories in it are assumed to be in sync with the
    /// contents of the [`project_root`](Self::project_root).
    pub(crate) _replica: Replica,

    /// The ID of the session.
    pub(crate) _session_id: SessionId,

    /// TODO: docs..
    pub(crate) server_tx: B::ServerTx,

    /// TODO: docs..
    pub(crate) server_rx: B::ServerRx,
}

pub(crate) enum RunSessionError<B: CollabBackend> {
    Rx(B::ServerRxError),
    RxExhausted(RxExhaustedError<B>),
    Tx(B::ServerTxError),
}

pub(crate) struct RxExhaustedError<B>(PhantomData<B>);

impl<B: CollabBackend> Session<B> {
    pub(crate) fn new(args: NewSessionArgs<B>) -> Self {
        Self { server_tx: args.server_tx, server_rx: args.server_rx }
    }

    pub(crate) async fn run(
        self,
        _ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), RunSessionError<B>> {
        let Self { server_rx, server_tx, .. } = self;
        pin_mut!(server_rx);
        pin_mut!(server_tx);

        loop {
            select! {
                maybe_msg_res = server_rx.next().fuse() => {
                    let msg = maybe_msg_res
                        .ok_or(RunSessionError::rx_exhausted())?
                        .map_err(RunSessionError::Rx)?;

                    // Echo it back. Just a placeholder for now.
                    server_tx
                        .send(msg)
                        .await
                        .map_err(RunSessionError::Tx)?;
                },
            }
        }
    }
}

impl<B: CollabBackend> RunSessionError<B> {
    fn rx_exhausted() -> Self {
        Self::RxExhausted(RxExhaustedError(PhantomData))
    }
}

impl<B: CollabBackend> notify::Error for RunSessionError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            RunSessionError::Rx(err) => err.to_message(),
            RunSessionError::RxExhausted(err) => err.to_message(),
            RunSessionError::Tx(err) => err.to_message(),
        }
    }
}

impl<B> notify::Error for RxExhaustedError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

#[cfg(feature = "neovim")]
mod neovim_error_impls {
    use nvimx2::neovim::Neovim;

    use super::*;

    impl notify::Error for RxExhaustedError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let msg = "the server kicked this peer out of the session";
            (notify::Level::Warn, notify::Message::from_str(msg))
        }
    }
}
