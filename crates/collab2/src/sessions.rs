use core::marker::PhantomData;

use collab_server::SessionId;
use nvimx2::{Shared, fs, notify};
use smallvec::SmallVec;
use smol_str::ToSmolStr;

/// TODO: docs.
#[derive(Default, Clone)]
pub(crate) struct Sessions {
    inner: Shared<SessionsInner>,
}

/// A guard making sure no new session is started whose root would overlap
/// (i.e. be either an ancestor or a descendant of) its [`root`](Self::root).
pub(crate) struct SessionGuard<State> {
    inner: SessionsGuardInner,
    _state: PhantomData<State>,
}

/// TODO: docs.
#[derive(Copy, Clone)]
pub(crate) enum SessionState {
    Active(SessionId),
    Starting,
}

pub(crate) struct Active;

pub(crate) struct Starting;

#[derive(Default)]
struct SessionsInner {
    sessions: SmallVec<[(fs::AbsPathBuf, SessionState); 2]>,
}

struct SessionsGuardInner {
    root: fs::AbsPathBuf,
    sessions: Sessions,
}

/// TODO: docs.
pub struct OverlappingSessionError {
    pub(crate) existing_root: fs::AbsPathBuf,
    pub(crate) existing_state: SessionState,
    pub(crate) new_root: fs::AbsPathBuf,
}

impl Sessions {
    pub(crate) fn start_guard(
        &self,
        root: fs::AbsPathBuf,
    ) -> Result<SessionGuard<Starting>, OverlappingSessionError> {
        self.insert(root.clone(), SessionState::Starting)?;
        Ok(SessionGuard {
            inner: SessionsGuardInner { root, sessions: self.clone() },
            _state: PhantomData,
        })
    }

    fn insert(
        &self,
        root: fs::AbsPathBuf,
        session_state: SessionState,
    ) -> Result<(), OverlappingSessionError> {
        self.inner.with_mut(|inner| inner.insert(root, session_state))
    }
}

impl<State> SessionGuard<State> {
    pub(crate) fn root(&self) -> &fs::AbsPath {
        &self.inner.root
    }

    fn with_state<R>(&self, fun: impl FnOnce(&mut SessionState) -> R) -> R {
        self.inner
            .sessions
            .inner
            .with_mut(|inner| fun(inner.state_mut(self.root())))
    }
}

impl SessionGuard<Starting> {
    pub(crate) fn into_active(
        self,
        session_id: SessionId,
    ) -> SessionGuard<Active> {
        self.with_state(|state| *state = SessionState::Active(session_id));
        SessionGuard { inner: self.inner, _state: PhantomData }
    }
}

impl SessionsInner {
    #[track_caller]
    fn idx_of(&self, root: &fs::AbsPath) -> usize {
        self.sessions
            .iter()
            .position(|(existing_root, _)| &**existing_root == root)
            .unwrap_or_else(|| panic!("no session at {root:?}"))
    }

    fn insert(
        &mut self,
        root: fs::AbsPathBuf,
        state: SessionState,
    ) -> Result<(), OverlappingSessionError> {
        for &(ref existing_root, existing_state) in &self.sessions {
            if root.starts_with(existing_root)
                || existing_root.starts_with(&root)
            {
                return Err(OverlappingSessionError {
                    existing_root: existing_root.clone(),
                    existing_state,
                    new_root: root.clone(),
                });
            }
        }
        self.sessions.push((root, state));
        Ok(())
    }

    #[track_caller]
    fn remove(&mut self, root: &fs::AbsPath) {
        self.sessions.swap_remove(self.idx_of(root));
    }

    #[track_caller]
    fn state_mut(&mut self, root: &fs::AbsPath) -> &mut SessionState {
        let session_idx = self.idx_of(root);
        let (_, state) = &mut self.sessions[session_idx];
        state
    }
}

impl Drop for SessionsGuardInner {
    fn drop(&mut self) {
        self.sessions.inner.with_mut(|inner| inner.remove(&self.root));
    }
}

impl notify::Error for OverlappingSessionError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        msg.push_str("cannot start a new session at ")
            .push_info(self.new_root.to_smolstr())
            .push_str(", another one is already ")
            .push_str(match self.existing_state {
                SessionState::Active(_) => "running",
                SessionState::Starting => "starting",
            })
            .push_str(" at ")
            .push_info(self.existing_root.to_smolstr())
            .push_str(" (sessions cannot overlap)");
        (notify::Level::Error, msg)
    }
}
