//! TODO: docs.

use core::fmt;
use core::marker::PhantomData;

use collab_project::PeerId;
use collab_project::fs::{DirectoryId, FileId};
use collab_server::message::{GitHubHandle, Message, Peer, Peers};
use ed::backend::{AgentId, Backend};
use ed::fs::{self, AbsPath, AbsPathBuf};
use ed::{AsyncCtx, Shared, notify};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use smol_str::ToSmolStr;

use crate::CollabBackend;
use crate::backend::{ActionForSelectedSession, SessionId};
use crate::event::Event;

/// TODO: docs.
pub struct Project<B: CollabBackend> {
    agent_id: AgentId,
    host_id: PeerId,
    peer_handle: GitHubHandle,
    project: collab_project::Project,
    _remote_peers: Peers,
    root_path: AbsPathBuf,
    session_id: SessionId<B>,
}

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct ProjectHandle<B: CollabBackend> {
    inner: Shared<Project<B>>,
    is_dropping_last_instance: Shared<bool>,
    projects: Projects<B>,
}

/// TODO: docs.
#[derive(Debug, PartialEq)]
pub struct OverlappingProjectError {
    /// TODO: docs.
    pub existing_root: AbsPathBuf,

    /// TODO: docs.
    pub new_root: AbsPathBuf,
}

/// TODO: docs.
pub struct NoActiveSessionError<B>(PhantomData<B>);

#[derive(cauchy::Clone, cauchy::Default)]
pub(crate) struct Projects<B: CollabBackend> {
    active: Shared<FxHashMap<SessionId<B>, ProjectHandle<B>>>,
    starting: Shared<FxHashSet<AbsPathBuf>>,
}

pub(crate) struct ProjectGuard<B: CollabBackend> {
    root: AbsPathBuf,
    projects: Projects<B>,
}

pub(crate) struct NewProjectArgs<B: CollabBackend> {
    pub(crate) host_id: PeerId,
    pub(crate) id_maps: IdMaps<B>,
    pub(crate) peer_handle: GitHubHandle,
    pub(crate) remote_peers: Peers,
    pub(crate) project: collab_project::Project,
    pub(crate) session_id: SessionId<B>,
}

#[derive(cauchy::Default)]
pub(crate) struct IdMaps<B: Backend> {
    pub(crate) buffer2file: FxHashMap<B::BufferId, FileId>,
    pub(crate) file2buffer: FxHashMap<FileId, B::BufferId>,
    pub(crate) node2dir: FxHashMap<<B::Fs as fs::Fs>::NodeId, DirectoryId>,
    pub(crate) node2file: FxHashMap<<B::Fs as fs::Fs>::NodeId, FileId>,
}

impl<B: CollabBackend> Project<B> {
    /// TODO: docs.
    pub fn is_host(&self) -> bool {
        self.project.peer_id() == self.host_id
    }

    /// TODO: docs.
    pub(crate) fn integrate_message(
        &mut self,
        _msg: Message,
        _ctx: &AsyncCtx<'_, B>,
    ) {
        todo!();
    }

    /// TODO: docs.
    pub(crate) fn synchronize_event(&mut self, _event: Event<B>) -> Message {
        todo!();
    }
}

impl<B: CollabBackend> ProjectHandle<B> {
    /// TODO: docs.
    pub fn root(&self) -> AbsPathBuf {
        self.with(|proj| proj.root_path.clone())
    }

    /// TODO: docs.
    pub fn session_id(&self) -> SessionId<B> {
        self.with(|proj| proj.session_id)
    }

    /// TODO: docs.
    pub fn with<R>(&self, fun: impl FnOnce(&Project<B>) -> R) -> R {
        self.inner.with(fun)
    }

    /// TODO: docs.
    pub fn with_mut<R>(&self, fun: impl FnOnce(&mut Project<B>) -> R) -> R {
        self.inner.with_mut(fun)
    }
}

impl<B: CollabBackend> Projects<B> {
    pub(crate) fn get(
        &self,
        session_id: SessionId<B>,
    ) -> Option<ProjectHandle<B>> {
        self.active.with(|map| map.get(&session_id).cloned())
    }

    pub(crate) fn new_guard(
        &self,
        project_root: AbsPathBuf,
    ) -> Result<ProjectGuard<B>, OverlappingProjectError> {
        fn overlaps(l: &AbsPath, r: &AbsPath) -> bool {
            l.starts_with(r) || r.starts_with(l)
        }

        let conflicting_root = self
            .active
            .with(|map| {
                map.values().find_map(|handle| {
                    handle.with(|proj| {
                        overlaps(&proj.root_path, &project_root)
                            .then(|| proj.root_path.clone())
                    })
                })
            })
            .or_else(|| {
                self.starting.with(|roots| {
                    roots
                        .iter()
                        .find(|root| overlaps(root, &project_root))
                        .cloned()
                })
            });

        if let Some(conflicting_root) = conflicting_root {
            return Err(OverlappingProjectError {
                existing_root: conflicting_root,
                new_root: project_root,
            });
        }

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
        action: ActionForSelectedSession,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<Option<(AbsPathBuf, SessionId<B>)>, NoActiveSessionError<B>>
    {
        let active_sessions = self.active.with(|map| {
            map.iter()
                .map(|(session_id, handle)| {
                    let root = handle.with(|proj| proj.root_path.clone());
                    (root, *session_id)
                })
                .collect::<SmallVec<[_; 1]>>()
        });

        let session = match &*active_sessions {
            [] => return Err(NoActiveSessionError::new()),
            [single] => single,
            sessions => match B::select_session(sessions, action, ctx).await {
                Some(session) => session,
                None => return Ok(None),
            },
        };

        Ok(Some(session.clone()))
    }

    fn insert(&self, project: Project<B>) -> ProjectHandle<B> {
        let session_id = project.session_id;
        let handle = ProjectHandle {
            inner: Shared::new(project),
            is_dropping_last_instance: Shared::new(false),
            projects: self.clone(),
        };
        self.active.with_mut(|map| {
            let prev = map.insert(session_id, handle.clone());
            assert!(prev.is_none());
        });
        handle
    }
}

impl<B: CollabBackend> ProjectGuard<B> {
    pub(crate) fn activate(self, args: NewProjectArgs<B>) -> ProjectHandle<B> {
        self.projects.starting.with_mut(|set| {
            assert!(set.remove(&self.root));
        });

        self.projects.insert(Project {
            agent_id: todo!(),
            host_id: args.host_id,
            peer_handle: args.peer_handle,
            _remote_peers: args.remote_peers,
            project: args.project,
            root_path: self.root.clone(),
            session_id: args.session_id,
        })
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

impl<B: CollabBackend> Drop for ProjectHandle<B> {
    fn drop(&mut self) {
        if self.inner.strong_count() == 2
            && !self.is_dropping_last_instance.copied()
        {
            self.is_dropping_last_instance.set(true);

            self.projects.active.with_mut(|map| {
                map.remove(&self.session_id());
            });
        }
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

impl<B> fmt::Debug for NoActiveSessionError<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("NoActiveSessionError")
    }
}

impl<B> notify::Error for NoActiveSessionError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        let msg = "there's no active collaborative editing session";
        (notify::Level::Error, notify::Message::from_str(msg))
    }
}
