use core::marker::PhantomData;

use flume::Receiver;
use futures_util::{FutureExt, SinkExt, StreamExt, pin_mut, select};
use nvimx2::{AsyncCtx, Shared, notify};

use crate::Project;
use crate::backend::CollabBackend;
use crate::leave::StopSession;

pub(crate) struct Session<B: CollabBackend> {
    args: NewSessionArgs<B>,
}

pub(crate) struct NewSessionArgs<B: CollabBackend> {
    /// TODO: docs.
    pub(crate) _project: Shared<Project<B>>,

    /// TODO: docs..
    pub(crate) server_rx: B::ServerRx,

    /// TODO: docs..
    pub(crate) server_tx: B::ServerTx,

    /// TODO: docs.
    pub(crate) stop_rx: Receiver<StopSession>,
}

pub(crate) enum RunSessionError<B: CollabBackend> {
    Rx(B::ServerRxError),
    RxExhausted(RxExhaustedError<B>),
    Tx(B::ServerTxError),
}

pub(crate) struct RxExhaustedError<B>(PhantomData<B>);

impl<B: CollabBackend> Session<B> {
    pub(crate) fn new(args: NewSessionArgs<B>) -> Self {
        Self { args }
    }

    pub(crate) async fn run(
        self,
        _ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), RunSessionError<B>> {
        let NewSessionArgs { stop_rx, server_rx, server_tx, .. } = self.args;

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

                _ = stop_rx.recv_async() => {
                    return Ok(());
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
