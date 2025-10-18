//! Contains types used to track the state of remote peers in a
//! [`Project`](crate::project::Project).

use std::collections::hash_map;

use abs_path::AbsPathBuf;
use collab_project::text::CursorId;
use collab_types::puff::file::LocalFileId;
use collab_types::{Peer, PeerId};
use editor::{Access, ByteOffset, Editor, Shared};
use fxhash::FxHashMap;

/// TODO: docs.
#[derive(Debug, Default, Clone)]
pub struct RemotePeers {
    /// A map of all the remote peers currently in a session.
    inner: Shared<FxHashMap<PeerId, Peer>>,
}

/// The position of a remote peer in a project.
///
/// This is defined as the position of the cursor with the smallest
/// [`CursorId`]. IDs [owned](CursorId::owner) by the same peer are [`Ord`]ered
/// by their creation time, with the smallest ID representing the oldest
/// cursor.
///
/// This approach should allow us to track a peer's "main" cursor, even in
/// editors that support multiple cursors.
#[derive(cauchy::Clone)]
pub(crate) struct PeerPosition<Ed: Editor> {
    /// The ID of the buffer corresponding to the file the cursor is in, or
    /// `None` if that file is not currently opened in the editor.
    pub(crate) buffer_id: Option<Ed::BufferId>,

    /// The cursor's ID.
    pub(crate) cursor_id: CursorId,

    /// The cursor's offset in the file/buffer.
    pub(crate) cursor_offset: ByteOffset,

    /// The ID of the file the cursor is in.
    pub(crate) file_id: LocalFileId,

    /// The path of the file the cursor is in.
    pub(crate) file_path: AbsPathBuf,
}

impl RemotePeers {
    /// Calls the given function on all the remote peers.
    pub(crate) fn for_each(&self, mut fun: impl FnMut(&Peer)) {
        self.with(|map| {
            for peer in map.values() {
                fun(peer);
            }
        });
    }

    /// Returns the [`Peer`] with the given ID, if any.
    pub(crate) fn get(&self, peer_id: PeerId) -> Option<Peer> {
        self.inner.with(|inner| inner.get(&peer_id).cloned())
    }

    #[track_caller]
    pub(crate) fn insert(&self, peer: Peer) {
        self.inner.with_mut(|inner| match inner.entry(peer.id) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(peer);
            },
            hash_map::Entry::Occupied(occupied) => {
                panic!(
                    "peer with ID {:?} already exists: {:?}",
                    peer.id,
                    occupied.get()
                )
            },
        });
    }

    #[track_caller]
    pub(crate) fn remove(&self, peer_id: PeerId) -> Peer {
        self.inner.with_mut(|inner| match inner.remove(&peer_id) {
            Some(peer) => peer,
            None => panic!("no peer with ID {:?} exists", peer_id),
        })
    }
}

impl Access<FxHashMap<PeerId, Peer>> for RemotePeers {
    fn with<R>(&self, fun: impl FnOnce(&FxHashMap<PeerId, Peer>) -> R) -> R {
        self.inner.with(fun)
    }
}

impl From<collab_types::Peers> for RemotePeers {
    fn from(peers: collab_types::Peers) -> Self {
        peers.into_iter().collect::<Self>()
    }
}

impl FromIterator<Peer> for RemotePeers {
    fn from_iter<T: IntoIterator<Item = Peer>>(iter: T) -> Self {
        Self {
            inner: Shared::new(
                iter.into_iter().map(|peer| (peer.id, peer)).collect(),
            ),
        }
    }
}
