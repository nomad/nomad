use std::io;

use collab_server::client::ClientRxError;
use collab_types::Message;
use ed::{Context, notify};
use flume::Receiver;
use futures_util::{FutureExt, SinkExt, StreamExt, pin_mut, select_biased};
use walkdir::Filter;

use crate::editors::{CollabEditor, MessageRx, MessageTx};
use crate::event_stream::{EventError, EventStream};
use crate::leave::StopRequest;
use crate::project::{ProjectHandle, SynchronizeError};

pub(crate) struct Session<Ed: CollabEditor, F: Filter<Ed::Fs>> {
    /// TODO: docs..
    pub(crate) event_stream: EventStream<Ed, F>,

    /// TODO: docs..
    pub(crate) message_rx: MessageRx<Ed>,

    /// TODO: docs..
    pub(crate) message_tx: MessageTx<Ed>,

    /// TODO: docs.
    pub(crate) project_handle: ProjectHandle<Ed>,

    /// TODO: docs.
    pub(crate) stop_rx: Receiver<StopRequest>,
}

#[derive(cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::From)]
#[display("{_0}")]
pub(crate) enum SessionError<Ed: CollabEditor, F: Filter<Ed::Fs>> {
    EventRx(#[from] EventError<Ed::Fs, F>),
    MessageRx(#[from] ClientRxError),
    #[display("the server kicked this peer out of the session")]
    MessageRxExhausted,
    MessageTx(#[from] io::Error),
    Synchronize(#[from] SynchronizeError<Ed>),
}

impl<Ed: CollabEditor, F: Filter<Ed::Fs>> Session<Ed, F> {
    pub(crate) async fn run(
        self,
        ctx: &mut Context<Ed>,
    ) -> Result<(), SessionError<Ed, F>> {
        let Self {
            mut event_stream,
            message_rx,
            message_tx,
            project_handle,
            stop_rx,
        } = self;

        pin_mut!(message_rx);
        pin_mut!(message_tx);

        let mut stop_stream = stop_rx.into_stream();

        loop {
            select_biased! {
                event_res = event_stream.next(ctx).fuse() => {
                    if let Some(message) =
                        project_handle.synchronize(event_res?, ctx).await?
                    {
                        message_tx.send(message).await?;
                    }
                },
                maybe_message_res = message_rx.next() => {
                    let message = maybe_message_res
                        .ok_or(SessionError::MessageRxExhausted)??;

                    if let Message::ProjectRequest(request) = message {
                        let response = project_handle.handle_request(request);
                        let message = Message::ProjectResponse(response);
                        message_tx.send(message).await?;
                        continue;
                    }

                    project_handle.integrate(message, ctx).await;
                },
                stop_request = stop_stream.select_next_some() => {
                    stop_request.send_stopped();
                    return Ok(())
                },
            }
        }
    }
}

impl<Ed: CollabEditor, F: Filter<Ed::Fs>> notify::Error
    for SessionError<Ed, F>
{
    fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}
