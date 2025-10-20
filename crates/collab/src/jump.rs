//! TODO: docs.

use core::cell::OnceCell;

use collab_project::text::CursorId;
use collab_types::{GitHubHandle, PeerHandle};
use editor::command::{self, CommandArgs, CommandCompletion, CursorPosition};
use editor::module::AsyncAction;
use editor::{AgentId, ByteOffset, Context};
use smallvec::SmallVec;

use crate::collab::Collab;
use crate::editors::CollabEditor;
use crate::project::Project;
use crate::session::{NoActiveSessionError, SessionInfos, Sessions};

/// TODO: docs.
#[derive(cauchy::Clone)]
pub struct Jump<Ed: CollabEditor> {
    sessions: Sessions<Ed>,
}

impl<Ed: CollabEditor> Jump<Ed> {
    pub(crate) async fn call_inner(
        &self,
        peer_handle: PeerHandle,
    ) -> Result<(), JumpError<Ed>> {
        let mut maybe_cursor_id = None;

        let Some(sesh) = self.sessions.find(|sesh| {
            match sesh.remote_peers.find(|peer| peer.handle == peer_handle) {
                Some(peer) => {
                    maybe_cursor_id = peer.main_cursor_id();
                    true
                },
                None => false,
            }
        }) else {
            return Err(JumpError::UnknownPeer(peer_handle));
        };

        let Some(cursor_id) = maybe_cursor_id else {
            return Err(JumpError::PeerCursorNotInProject(peer_handle, sesh));
        };

        sesh.project_access
            .with(async move |proj, ctx| {
                Self::jump_to(proj, cursor_id, ctx).await
            })
            .await
            .ok_or(JumpError::UnknownPeer(peer_handle))?
            .map_err(JumpError::Jump)
    }

    pub(crate) async fn jump_to(
        proj: &Project<Ed>,
        cursor_id: CursorId,
        ctx: &mut Context<Ed>,
    ) -> Result<(), JumpToCursorError<Ed>> {
        let cursor = proj
            .inner
            .cursor(cursor_id)
            .ok_or(JumpToCursorError::UnknownId(cursor_id))?;

        let file = cursor.file();

        let agent_id = Self::agent_id(ctx);

        let buffer_id = match proj.id_maps.file2buffer.get(&file.local_id()) {
            Some(buffer_id) => buffer_id.clone(),
            // If there's no open buffer for the file the cursor is in, create
            // one.
            None => {
                let file_path_in_proj = file.path();
                let file_path = proj.root_path().concat(&file_path_in_proj);
                ctx.create_buffer(&file_path, agent_id)
                    .await
                    .map_err(JumpToCursorError::CreateBuffer)?
            },
        };

        Ed::jump_to(buffer_id, cursor.offset(), agent_id, ctx).await;

        Ok(())
    }

    fn agent_id(ctx: &mut Context<Ed>) -> AgentId {
        thread_local! {
            static AGENT_ID: OnceCell<AgentId> = const { OnceCell::new() };
        }
        AGENT_ID.with(|cell| *cell.get_or_init(|| ctx.new_agent_id()))
    }
}

impl<Ed: CollabEditor> AsyncAction<Ed> for Jump<Ed> {
    const NAME: &str = "jump";

    type Args = command::Parse<GitHubHandle>;

    async fn call(
        &mut self,
        command::Parse(github_handle): Self::Args,
        ctx: &mut Context<Ed>,
    ) {
        if let Err(err) =
            self.call_inner(PeerHandle::GitHub(github_handle)).await
        {
            Ed::on_jump_error(err, ctx);
        }
    }
}

/// The type of error that can occur when [`Jump`]ing fails.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
pub enum JumpError<Ed: CollabEditor> {
    /// Jumping failed.
    #[display("{_0}")]
    Jump(JumpToCursorError<Ed>),

    /// There are no active sessions.
    #[display("{}", NoActiveSessionError)]
    NoActiveSession,

    /// The given peer doesn't have a cursor in the project tracked by the
    /// given session.
    #[display("{_0}'s cursor is not in {}", _1.proj_name())]
    PeerCursorNotInProject(PeerHandle, SessionInfos<Ed>),

    /// There's no peer with the given handle in any of the sessions.
    #[display("There's no peer with handle '{_0}' in any of the sessions")]
    UnknownPeer(PeerHandle),
}

/// The type of error that can occur when [`jumping`](Jump::jump_to) to the
/// position of a remote peer's cursor.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
pub enum JumpToCursorError<Ed: CollabEditor> {
    /// Creating a new buffer failed.
    #[display("{_0}")]
    CreateBuffer(Ed::CreateBufferError),

    /// The given [`CursorId`] doesn't exist in the project.
    #[display("There's no cursor with ID '{_0:?}' in the project")]
    UnknownId(CursorId),
}

impl<Ed: CollabEditor> From<&Collab<Ed>> for Jump<Ed> {
    fn from(collab: &Collab<Ed>) -> Self {
        Self { sessions: collab.sessions.clone() }
    }
}

impl<Ed: CollabEditor> command::ToCompletionFn<Ed> for Jump<Ed> {
    fn to_completion_fn(&self) -> impl command::CompletionFn + 'static {
        let sessions = self.sessions.clone();

        move |command_args: CommandArgs<'_, ByteOffset>| {
            let mut completions = SmallVec::<[_; 2]>::new();

            let handle_prefix = match command_args.cursor_pos() {
                CursorPosition::InArg(arg, offset) if arg.is_first() => {
                    &arg.as_str()[..offset]
                },
                CursorPosition::BetweenArgs(prev, _) if prev.is_none() => "",
                _ => return completions,
            };

            sessions.for_each(|session_infos| {
                session_infos.remote_peers.for_each(|peer| {
                    if peer.handle.as_str().starts_with(handle_prefix) {
                        completions.push(CommandCompletion::from_str(
                            peer.handle.as_str(),
                        ));
                    }
                })
            });

            completions
        }
    }
}

impl<Ed: CollabEditor> From<NoActiveSessionError> for JumpError<Ed> {
    fn from(_: NoActiveSessionError) -> Self {
        Self::NoActiveSession
    }
}
