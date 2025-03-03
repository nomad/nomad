use core::marker::PhantomData;

use collab_server::SessionId;
use collab_server::message::{Peer, Peers};
use eerie::Replica;
use fxhash::{FxHashMap, FxHashSet};
use nvimx2::fs::{AbsPath, AbsPathBuf};
use nvimx2::{AsyncCtx, Shared, notify};
use smol_str::ToSmolStr;

use crate::CollabBackend;

/// TODO: docs.
pub struct Project<B: CollabBackend> {
    host: Peer,
    local_peer: Peer,
    remote_peers: Peers,
    replica: Replica,
    root: AbsPathBuf,
    session_id: SessionId,
    _phantom: PhantomData<B>,
}

/// TODO: docs.
pub struct ProjectHandle<B: CollabBackend> {
    inner: Shared<Project<B>>,
    projects: Projects<B>,
}

/// TODO: docs.
#[derive(Debug, PartialEq)]
pub struct OverlappingProjectError {
    pub(crate) existing_root: AbsPathBuf,
    pub(crate) new_root: AbsPathBuf,
}

pub struct NoActiveSessionError<B>(PhantomData<B>);

pub(crate) struct Projects<B: CollabBackend> {
    active: Shared<FxHashMap<SessionId, ProjectHandle<B>>>,
    starting: Shared<FxHashSet<AbsPathBuf>>,
}

pub(crate) struct ProjectGuard<B: CollabBackend> {
    root: AbsPathBuf,
    projects: Projects<B>,
}

pub(crate) struct NewProjectArgs {
    pub(crate) host: Peer,
    pub(crate) local_peer: Peer,
    pub(crate) remote_peers: Peers,
    pub(crate) replica: Replica,
    pub(crate) session_id: SessionId,
}

impl<B: CollabBackend> Project<B> {
    /// TODO: docs.
    pub fn is_host(&self) -> bool {
        self.local_peer.id() == self.host.id()
    }
}

impl<B: CollabBackend> ProjectHandle<B> {
    /// TODO: docs.
    pub fn with<R>(&self, fun: impl FnOnce(&Project<B>) -> R) -> R {
        self.inner.with(fun)
    }
}

impl<B: CollabBackend> Projects<B> {
    pub(crate) fn get(
        &self,
        session_id: SessionId,
    ) -> Option<ProjectHandle<B>> {
        self.active.with(|map| map.get(&session_id).cloned())
    }

    pub(crate) fn new_guard(
        &self,
        project_root: AbsPathBuf,
    ) -> Result<ProjectGuard<B>, OverlappingProjectError> {
        let guard = ProjectGuard {
            root: project_root.clone(),
            projects: self.clone(),
        };

        self.starting.with_mut(|map| {
            assert!(map.insert(project_root));
        });

        Ok(guard)
    }

    pub(crate) async fn select(
        &self,
        _ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<Option<(AbsPathBuf, SessionId)>, NoActiveSessionError<B>> {
        todo!();
        // let active_sessions = self
        //     .sessions
        //     .iter()
        //     .filter_map(|(root, state)| match state {
        //         SessionState::Active(session_id) => Some((root, session_id)),
        //         _ => None,
        //     })
        //     .collect::<SmallVec<[_; 1]>>();
        //
        // let session = match &*active_sessions {
        //     [] => return Err(NoActiveSessionError::new()),
        //     [single] => single,
        //     sessions => match B::select_session(
        //         sessions,
        //         ActionForSelectedSession::CopySessionId,
        //         ctx,
        //     )
        //     .await
        //     {
        //         Some(session) => session,
        //         None => return Ok(None),
        //     },
        // };
        //
        // Ok(Some(session.clone()))
    }
}

impl<B: CollabBackend> ProjectGuard<B> {
    pub(crate) fn activate(self, args: NewProjectArgs) -> ProjectHandle<B> {
        self.projects.starting.with_mut(|set| {
            assert!(set.remove(&self.root));
        });

        let handle = ProjectHandle {
            inner: Shared::new(Project {
                host: args.host,
                local_peer: args.local_peer,
                remote_peers: args.remote_peers,
                replica: args.replica,
                root: self.root.clone(),
                session_id: args.session_id,
                _phantom: PhantomData,
            }),
            projects: self.projects.clone(),
        };

        self.projects.active.with_mut(|map| {
            map.insert(args.session_id, handle.clone());
        });

        handle
    }

    pub(crate) fn root(&self) -> &AbsPath {
        &self.root
    }
}

impl<B> NoActiveSessionError<B> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<B: CollabBackend> Clone for ProjectHandle<B> {
    fn clone(&self) -> Self {
        Self { inner: self.inner.clone(), projects: self.projects.clone() }
    }
}

impl<B: CollabBackend> Drop for ProjectHandle<B> {
    fn drop(&mut self) {
        let session_id = if self.inner.strong_count() == 2 {
            self.with(|project| project.session_id)
        } else {
            return;
        };

        self.projects.active.with_mut(|map| {
            map.remove(&session_id);
        });
    }
}

impl<B: CollabBackend> Default for Projects<B> {
    fn default() -> Self {
        Self { active: Default::default(), starting: Default::default() }
    }
}

impl<B: CollabBackend> Clone for Projects<B> {
    fn clone(&self) -> Self {
        Self { active: self.active.clone(), starting: self.starting.clone() }
    }
}

impl<B: CollabBackend> Drop for ProjectGuard<B> {
    fn drop(&mut self) {
        self.projects.starting.with_mut(|set| {
            set.remove(&self.root);
        });
    }
}

impl notify::Error for OverlappingProjectError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        msg.push_str("cannot start a new session at ")
            .push_info(self.new_root.to_smolstr())
            .push_str(", another one is already running at ")
            .push_info(self.existing_root.to_smolstr())
            .push_str(" (sessions cannot overlap)");
        (notify::Level::Error, msg)
    }
}

impl<B> notify::Error for NoActiveSessionError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        let msg = "there's no active collaborative editing session";
        (notify::Level::Error, notify::Message::from_str(msg))
    }
}
