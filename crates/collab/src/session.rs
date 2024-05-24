use core::future::ready;

use cola::Replica;
use collab_client::messages::{FileKind, InboundMessage};
use futures::{pin_mut, select as race, FutureExt, StreamExt};
use nomad::prelude::{Buffer, EditorId, Get, NvimBuffer};

use crate::config::ConnectorError;
use crate::{Config, Convert, SessionId};

/// TODO: docs
pub(crate) struct Session {
    /// TODO: docs
    buffer: Buffer,

    /// TODO: docs
    editor_id: EditorId,

    /// TODO: docs
    receiver: collab_client::Receiver,

    /// TODO: docs
    sender: collab_client::Sender,

    /// TODO: docs
    session_id: SessionId,
}

impl Session {
    /// Returns the [`SessionId`] of the session, which is unique to each
    /// session and can be sent to other peers to join the session.
    pub(crate) fn id(&self) -> SessionId {
        self.session_id
    }

    /// TODO: docs
    pub async fn join(
        config: &Get<Config>,
        session_id: SessionId,
    ) -> Result<Self, JoinError> {
        let peer_id = collab_client::messages::PeerId::new_random();

        let (sender, receiver, session) = config
            .get()
            .connector()?
            .peer_id(peer_id)
            .join(session_id.into())
            .await?;

        let FileKind::Document(doc) = session.project().root().kind() else {
            unreachable!();
        };

        let Ok(replica) = Replica::decode(peer_id.as_u64(), doc.replica())
        else {
            unreachable!();
        };

        Ok(Self {
            buffer: Buffer::create(doc.text(), replica),
            editor_id: EditorId::generate(),
            session_id,
            receiver,
            sender,
        })
    }

    /// TODO: docs
    async fn handle_inbound(&mut self, msg: InboundMessage) {
        let buffer = &mut self.buffer;

        let id = self.editor_id;

        match msg {
            InboundMessage::RemoteDeletion(deletion) => {
                // TODO: don't clone.
                buffer.edit(deletion.crdt().clone(), id);
            },
            InboundMessage::RemoteInsertion(insertion) => {
                // TODO: don't clone.
                buffer.edit((insertion.crdt().clone(), insertion.text()), id);
            },
            InboundMessage::SessionRequest(request) => {
                request.send(buffer.snapshot().convert());
            },
            _ => {},
        }
    }

    /// TODO: docs
    pub async fn run(&mut self) -> Result<(), RunError> {
        let editor_id = self.editor_id;

        let edits = self
            .buffer
            .edits()
            .filter(|edit| ready(edit.applied_by() != editor_id));

        pin_mut!(edits);

        loop {
            race! {
                maybe_edit = edits.next().fuse() => {
                    let Some(edit) = maybe_edit else { return Ok(()) };
                    self.sender.send(edit.convert())?;
                },
                maybe_msg = self.receiver.recv().fuse() => {
                    let Ok(msg) = maybe_msg else { return Ok(()) };
                    self.handle_inbound(msg).await;
                },
            }
        }
    }

    /// TODO: docs
    pub async fn start(config: &Get<Config>) -> Result<Self, StartError> {
        let peer_id = collab_client::messages::PeerId::new_random();

        let (sender, receiver, session_id) =
            config.get().connector()?.peer_id(peer_id).start().await?;

        Ok(Self {
            buffer: Buffer::from_id(peer_id.as_u64(), NvimBuffer::current()),
            editor_id: EditorId::generate(),
            receiver,
            sender,
            session_id: session_id.into(),
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error(transparent)]
    Connection(#[from] collab_client::Error),

    #[error(transparent)]
    Connector(#[from] ConnectorError),
}

#[derive(Debug, thiserror::Error)]
pub enum StartError {
    #[error(transparent)]
    Connection(#[from] collab_client::Error),

    #[error(transparent)]
    Connector(#[from] ConnectorError),
}

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error(transparent)]
    Collab(#[from] collab_client::Error),
}
