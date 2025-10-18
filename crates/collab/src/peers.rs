//! Contains types used to track the state of remote peers in a
//! [`Project`](crate::project::Project).

use core::ops::Deref;
use std::collections::hash_map;

use collab_project::text::CursorId;
use collab_types::{Peer, PeerId};
use editor::{Access, Shared};
use fxhash::FxHashMap;

/// TODO: docs.
#[derive(Debug, Default, Clone)]
pub struct RemotePeers {
    /// A map of all the remote peers currently in a session.
    inner: Shared<FxHashMap<PeerId, RemotePeer>>,
}

/// TODO: docs.
#[derive(Debug, Clone)]
pub struct RemotePeer {
    inner: Peer,
    // - when a remote cursor is created;
    // - when a remote cursor is moved;
    // - when a remote cursor is removed;
    main_cursor_id: Option<CursorId>,
}

impl RemotePeers {
    /// Creates a new [`RemotePeers`] instance from the given peers.
    pub fn new(
        peers: impl IntoIterator<Item = Peer>,
        proj: &collab_project::Project,
    ) -> Self {
        let map = peers
            .into_iter()
            .map(|peer| RemotePeer::new(peer, proj))
            .map(|remote_peer| (remote_peer.id, remote_peer))
            .collect();

        Self { inner: Shared::new(map) }
    }

    /// Calls the given function on all the remote peers.
    pub(crate) fn for_each(&self, mut fun: impl FnMut(&RemotePeer)) {
        self.with(|map| {
            for peer in map.values() {
                fun(peer);
            }
        });
    }

    pub(crate) fn find(
        &self,
        mut fun: impl FnMut(&RemotePeer) -> bool,
    ) -> Option<RemotePeer> {
        self.with(|map| {
            map.values().find_map(|peer| fun(peer).then(|| peer.clone()))
        })
    }

    pub(crate) fn find_map<T>(
        &self,
        fun: impl FnMut(&RemotePeer) -> Option<T>,
    ) -> Option<T> {
        self.with(|map| map.values().find_map(fun))
    }

    /// Returns the [`Peer`] with the given ID, if any.
    pub(crate) fn get(&self, peer_id: PeerId) -> Option<RemotePeer> {
        self.inner.with(|inner| inner.get(&peer_id).cloned())
    }

    #[track_caller]
    pub(crate) fn insert(&self, peer: Peer, proj: &collab_project::Project) {
        self.inner.with_mut(|inner| match inner.entry(peer.id) {
            hash_map::Entry::Vacant(vacant) => {
                vacant.insert(RemotePeer::new(peer, proj));
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
    pub(crate) fn remove(&self, peer_id: PeerId) -> RemotePeer {
        self.inner.with_mut(|inner| match inner.remove(&peer_id) {
            Some(peer) => peer,
            None => panic!("no peer with ID {:?} exists", peer_id),
        })
    }
}

impl RemotePeer {
    /// This is defined as the position of the cursor with the smallest
    /// [`CursorId`]. IDs [owned](CursorId::owner) by the same peer are [`Ord`]ered
    /// by their creation time, with the smallest ID representing the oldest
    /// cursor.
    ///
    /// This approach should allow us to track a peer's "main" cursor, even in
    /// editors that support multiple cursors.
    pub fn main_cursor_id(&self) -> Option<CursorId> {
        self.main_cursor_id
    }

    pub(crate) fn into_inner(self) -> Peer {
        self.inner
    }

    fn new(peer: Peer, proj: &collab_project::Project) -> Self {
        let main_cursor_id = proj
            .cursors()
            .filter_map(|cur| (cur.owner() == peer.id).then_some(cur.id()))
            .min();

        Self { inner: peer, main_cursor_id }
    }
}

impl Access<FxHashMap<PeerId, RemotePeer>> for RemotePeers {
    fn with<R>(
        &self,
        fun: impl FnOnce(&FxHashMap<PeerId, RemotePeer>) -> R,
    ) -> R {
        self.inner.with(fun)
    }
}

impl Deref for RemotePeer {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<RemotePeer> for Peer {
    fn from(peer: RemotePeer) -> Self {
        peer.inner
    }
}
