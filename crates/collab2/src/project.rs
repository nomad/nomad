use core::marker::PhantomData;

use collab_server::SessionId;
use eerie::Replica;
use fxhash::FxHashMap;
use nvimx2::fs::{AbsPathBuf, Fs};
use nvimx2::{AsyncCtx, Shared, notify};
use smol_str::ToSmolStr;

use crate::CollabBackend;

/// TODO: docs.
pub struct Project<B: CollabBackend> {
    projects: Projects<B>,
    _replica: Replica,
    _root: AbsPathBuf,
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
    map: Shared<FxHashMap<SessionId, Shared<Project<B>>>>,
}

pub(crate) struct NewProjectArgs {
    pub(crate) replica: Replica,
    pub(crate) root: AbsPathBuf,
    pub(crate) session_id: SessionId,
}

impl<B: CollabBackend> Project<B> {
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
        self.map.with(|map| map.get(&session_id).cloned())
    }

    pub(crate) fn insert(
        &self,
        args: NewProjectArgs,
    ) -> Result<Shared<Project<B>>, OverlappingProjectError> {
        let project = Shared::new(Project {
            projects: self.clone(),
            _replica: args.replica,
            _root: args.root,
            session_id: args.session_id,
        });

        self.map.with_mut(|map| {
            map.insert(args.session_id, project.clone());
        });

        Ok(project)
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
