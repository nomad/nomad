use core::marker::PhantomData;
use std::collections::hash_map::Entry;

use collab_server::SessionId;
use collab_server::message::{Peer, Peers};
use eerie::Replica;
use fxhash::FxHashMap;
use nvimx2::fs::{AbsPath, AbsPathBuf, Fs};
use nvimx2::{AsyncCtx, Shared, notify};
use smol_str::ToSmolStr;

use crate::CollabBackend;

/// TODO: docs.
pub struct Project<B: CollabBackend> {
    host: Peer,
    local_peer: Peer,
    projects: Projects<B>,
    remote_peers: Peers,
    replica: Replica,
    root: AbsPathBuf,
    session_id: SessionId,
}

/// TODO: docs.
#[derive(Debug, PartialEq)]
pub struct OverlappingProjectError {
    pub(crate) existing_root: AbsPathBuf,
    pub(crate) new_root: AbsPathBuf,
}

pub struct NoActiveSessionError<B>(PhantomData<B>);

pub(crate) struct Projects<B: CollabBackend> {
    map: Shared<FxHashMap<SessionId, ProjectState<B>>>,
}

pub(crate) struct ProjectGuard<B: CollabBackend> {
    root: AbsPathBuf,
    projects: Projects<B>,
    session_id: SessionId,
}

pub(crate) struct NewProjectArgs {
    pub(crate) host: Peer,
    pub(crate) local_peer: Peer,
    pub(crate) remote_peers: Peers,
    pub(crate) replica: Replica,
}

enum ProjectState<B: CollabBackend> {
    Active(Shared<Project<B>>),
    Joining(ProjectGuard<B>),
    Starting(ProjectGuard<B>),
}

impl<B: CollabBackend> Project<B> {
    /// TODO: docs.
    pub fn is_host(&self) -> bool {
        self.local_peer.id() == self.host.id()
    }

    /// TODO: docs.
    pub(crate) async fn flush(
        &self,
        _project_root: &<B::Fs as Fs>::Directory,
        _fs: B::Fs,
    ) {
    }
}

impl<B: CollabBackend> Projects<B> {
    pub(crate) fn get(
        &self,
        session_id: SessionId,
    ) -> Option<Shared<Project<B>>> {
        self.map.with(|map| match map.get(&session_id)? {
            ProjectState::Active(project) => Some(project.clone()),
            _ => None,
        })
    }

    pub(crate) fn join_guard(
        &self,
        project_root: AbsPathBuf,
        session_id: SessionId,
    ) -> Result<ProjectGuard<B>, OverlappingProjectError> {
        let guard = ProjectGuard {
            root: project_root,
            projects: self.clone(),
            session_id,
        };

        let state = ProjectState::Joining(guard.priv_clone());

        self.map.with_mut(|map| {
            let prev = map.insert(session_id, state);
            assert!(prev.is_none());
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
    pub(crate) fn activate(self, args: NewProjectArgs) -> Shared<Project<B>> {
        let project = Shared::new(Project {
            host: args.host,
            local_peer: args.local_peer,
            projects: self.projects.clone(),
            remote_peers: args.remote_peers,
            replica: args.replica,
            root: self.root.clone(),
            session_id: self.session_id,
        });

        self.projects.map.with_mut(|map| {
            map.insert(self.session_id, ProjectState::Active(project.clone()));
        });

        project
    }

    pub(crate) fn root(&self) -> &AbsPath {
        &self.root
    }

    fn priv_clone(&self) -> Self {
        Self {
            root: self.root.clone(),
            projects: self.projects.clone(),
            session_id: self.session_id,
        }
    }
}

impl<B> NoActiveSessionError<B> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<B: CollabBackend> Drop for Project<B> {
    fn drop(&mut self) {
        self.projects.map.with_mut(|map| {
            map.remove(&self.session_id);
        });
    }
}

impl<B: CollabBackend> Default for Projects<B> {
    fn default() -> Self {
        Self { map: Default::default() }
    }
}

impl<B: CollabBackend> Clone for Projects<B> {
    fn clone(&self) -> Self {
        Self { map: self.map.clone() }
    }
}

impl<B: CollabBackend> Drop for ProjectGuard<B> {
    fn drop(&mut self) {
        self.projects.map.with_mut(|map| {
            let Entry::Occupied(entry) = map.entry(self.session_id) else {
                unreachable!()
            };

            if matches!(
                entry.get(),
                ProjectState::Joining(_) | ProjectState::Starting(_)
            ) {
                entry.remove();
            }
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
