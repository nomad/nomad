//! TODO: docs.

use core::convert::Infallible;
use core::fmt;
use core::marker::PhantomData;
use std::sync::Arc;

use auth::AuthInfos;
use concurrent_queue::{ConcurrentQueue, PushError};
use eerie::{PeerId, Replica, ReplicaBuilder};
use nvimx2::action::AsyncAction;
use nvimx2::command::ToCompletionFn;
use nvimx2::fs::{self, AbsPath, AbsPathBuf, Directory, FsNodeKind, Metadata};
use nvimx2::notify::{self, Name};
use nvimx2::{AsyncCtx, ByteOffset, Shared};
use walkdir::{Either, WalkDir, WalkError, WalkErrorKind};

use crate::backend::{CollabBackend, StartArgs};
use crate::collab::Collab;
use crate::config::Config;
use crate::leave::StopChannels;
use crate::project::{NewProjectArgs, OverlappingProjectError, Projects};
use crate::session::{NewSessionArgs, Session};

/// The `Action` used to start a new collaborative editing session.
pub struct Start<B: CollabBackend> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    projects: Projects<B>,
    stop_channels: StopChannels<B>,
}

impl<B: CollabBackend> AsyncAction<B> for Start<B> {
    const NAME: Name = "start";

    type Args = ();

    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), StartError<B>> {
        let auth_infos = self
            .auth_infos
            .with(|infos| infos.as_ref().cloned())
            .ok_or(StartError::UserNotLoggedIn)?;

        let buffer_id = ctx.with_ctx(|ctx| {
            ctx.current_buffer()
                .map(|buf| buf.id())
                .ok_or(StartError::NoBufferFocused)
        })?;

        let project_root = B::search_project_root(buffer_id, ctx)
            .await
            .map_err(StartError::SearchProjectRoot)?;

        let project_guard = self
            .projects
            .new_guard(project_root)
            .map_err(StartError::OverlappingProject)?;

        if !B::confirm_start(project_guard.root(), ctx).await {
            return Ok(());
        }

        let project_name = project_guard
            .root()
            .node_name()
            .ok_or(StartError::ProjectRootIsFsRoot)?;

        let start_args = StartArgs {
            auth_infos: &auth_infos,
            project_name,
            server_address: &self.config.with(|c| c.server_address.clone()),
        };

        let sesh_infos = B::start_session(start_args, ctx)
            .await
            .map_err(StartError::StartSession)?;

        let replica = read_replica(
            sesh_infos.local_peer.id(),
            project_guard.root().to_owned(),
            ctx,
        )
        .await
        .map_err(StartError::ReadReplica)?;

        let project_handle = project_guard.activate(NewProjectArgs {
            host_id: sesh_infos.host_id,
            local_peer: sesh_infos.local_peer,
            replica,
            remote_peers: sesh_infos.remote_peers,
            session_id: sesh_infos.session_id,
        });

        let session = Session::new(NewSessionArgs {
            project_handle,
            server_rx: sesh_infos.server_rx,
            server_tx: sesh_infos.server_tx,
            stop_rx: self.stop_channels.insert(sesh_infos.session_id),
        });

        ctx.spawn_local(async move |ctx| {
            if let Err(err) = session.run(ctx).await {
                ctx.emit_err(err);
            }
        })
        .detach();

        Ok(())
    }
}

async fn read_replica<B>(
    peer_id: PeerId,
    project_root: AbsPathBuf,
    ctx: &mut AsyncCtx<'_, B>,
) -> Result<Replica, ReadReplicaError<B>>
where
    B: CollabBackend,
{
    enum PushNode {
        File(AbsPathBuf, ByteOffset),
        Directory(AbsPathBuf),
    }

    let fs = ctx.fs();
    let res = async move {
        let op_queue = Arc::new(ConcurrentQueue::unbounded());
        let op_queue2 = Arc::clone(&op_queue);
        let handler = async move |entry: walkdir::DirEntry<'_, _>| {
            let op = match entry.node_kind() {
                FsNodeKind::File => {
                    PushNode::File(entry.path(), entry.byte_len())
                },
                FsNodeKind::Directory => PushNode::Directory(entry.path()),
                FsNodeKind::Symlink => return Ok(()),
            };
            match op_queue2.push(op) {
                Ok(()) => Ok(()),
                Err(PushError::Full(_)) => unreachable!("unbounded"),
                Err(PushError::Closed(_)) => unreachable!("never closed"),
            }
        };
        fs.for_each::<_, Infallible>(&project_root, handler).await?;
        let mut builder = ReplicaBuilder::new(peer_id);
        while let Ok(op) = op_queue.pop() {
            let _ = match op {
                PushNode::File(path, len) => {
                    builder.push_file(path, len.into_u64())
                },
                PushNode::Directory(path) => builder.push_directory(path),
            };
        }
        Ok::<_, walkdir::ForEachError<_, _>>(builder)
    };

    let mut builder = match res.await {
        Ok(builder) => builder,
        Err(err) => match err.kind {
            Either::Left(left) => {
                return Err(ReadReplicaError::Walk(WalkError {
                    dir_path: err.dir_path,
                    kind: left,
                }));
            },
            Either::Right(_infallible) => unreachable!(),
        },
    };

    // Update the lengths of the open buffers.
    //
    // FIXME: what if a buffer was edited and already closed?
    ctx.for_each_buffer(|buffer| {
        if let Some(mut file) = <&AbsPath>::try_from(&*buffer.name())
            .ok()
            .and_then(|buffer_path| builder.file_mut(buffer_path))
        {
            file.set_len(buffer.byte_len().into());
        }
    });

    Ok(builder.build())
}

/// The type of error that can occur when [`Start`]ing a session fails.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum StartError<B: CollabBackend> {
    /// TODO: docs.
    NoBufferFocused,

    /// TODO: docs.
    OverlappingProject(OverlappingProjectError),

    /// TODO: docs.
    ProjectRootIsFsRoot,

    /// TODO: docs.
    ReadReplica(ReadReplicaError<B>),

    /// TODO: docs.
    SearchProjectRoot(B::SearchProjectRootError),

    /// TODO: docs.
    StartSession(B::StartSessionError),

    /// TODO: docs.
    UserNotLoggedIn,
}

/// TODO: docs.
#[derive(derive_more::Debug)]
#[debug(bound(B: CollabBackend))]
pub enum ReadReplicaError<B: CollabBackend> {
    /// TODO: docs.
    Walk(WalkError<WalkErrorKind<B::Fs>>),
}

/// TODO: docs.
pub(crate) struct UserNotLoggedInError<B>(PhantomData<B>);

/// TODO: docs.
struct NoBufferFocusedError<B>(PhantomData<B>);

impl<B: CollabBackend> Clone for Start<B> {
    fn clone(&self) -> Self {
        Self {
            auth_infos: self.auth_infos.clone(),
            config: self.config.clone(),
            stop_channels: self.stop_channels.clone(),
            projects: self.projects.clone(),
        }
    }
}

impl<B: CollabBackend> From<&Collab<B>> for Start<B> {
    fn from(collab: &Collab<B>) -> Self {
        Self {
            auth_infos: collab.auth_infos.clone(),
            config: collab.config.clone(),
            projects: collab.projects.clone(),
            stop_channels: collab.stop_channels.clone(),
        }
    }
}

impl<B: CollabBackend> ToCompletionFn<B> for Start<B> {
    fn to_completion_fn(&self) {}
}

impl<B> NoBufferFocusedError<B> {
    fn new() -> Self {
        Self(PhantomData)
    }
}

impl<B> UserNotLoggedInError<B> {
    pub(crate) fn new() -> Self {
        Self(PhantomData)
    }
}

impl<B> PartialEq for StartError<B>
where
    B: CollabBackend,
    ReadReplicaError<B>: PartialEq,
    B::SearchProjectRootError: PartialEq,
    B::StartSessionError: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use StartError::*;

        match (self, other) {
            (NoBufferFocused, NoBufferFocused) => true,
            (OverlappingProject(l), OverlappingProject(r)) => l == r,
            (ProjectRootIsFsRoot, ProjectRootIsFsRoot) => true,
            (ReadReplica(l), ReadReplica(r)) => l == r,
            (SearchProjectRoot(l), SearchProjectRoot(r)) => l == r,
            (StartSession(l), StartSession(r)) => l == r,
            (UserNotLoggedIn, UserNotLoggedIn) => true,
            _ => false,
        }
    }
}

impl<B: CollabBackend> notify::Error for StartError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::NoBufferFocused => {
                NoBufferFocusedError::<B>::new().to_message()
            },
            Self::OverlappingProject(err) => err.to_message(),
            Self::ProjectRootIsFsRoot => (
                notify::Level::Error,
                notify::Message::from_str(
                    "cannot start a new collaborative editing session at the \
                     root of the filesystem",
                ),
            ),
            Self::ReadReplica(err) => err.to_message(),
            Self::SearchProjectRoot(err) => err.to_message(),
            Self::StartSession(err) => err.to_message(),
            Self::UserNotLoggedIn => {
                UserNotLoggedInError::<B>::new().to_message()
            },
        }
    }
}

impl<B> notify::Error for NoBufferFocusedError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

impl<B: CollabBackend> PartialEq for ReadReplicaError<B>
where
    WalkErrorKind<B::Fs>: PartialEq,
    <<<B::Fs as fs::Fs>::Directory as Directory>::Metadata as Metadata>::Error:
        PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        use ReadReplicaError::*;

        match (self, other) {
            (Walk(l), Walk(r)) => l == r,
        }
    }
}

impl<B: CollabBackend> fmt::Display for ReadReplicaError<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReadReplicaError::Walk(err) => fmt::Display::fmt(err, f),
        }
    }
}

impl<B: CollabBackend> notify::Error for ReadReplicaError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Error, notify::Message::from_display(self))
    }
}

impl<B> notify::Error for UserNotLoggedInError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

#[cfg(feature = "neovim")]
mod neovim_error_impls {
    use nvimx2::neovim::Neovim;
    use smol_str::ToSmolStr;

    use super::*;

    impl notify::Error for NoBufferFocusedError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let msg = "couldn't determine path to project root. Either move \
                       the cursor to a text buffer, or pass one explicitly";
            (notify::Level::Error, notify::Message::from_str(msg))
        }
    }

    impl notify::Error for ReadReplicaError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            use ReadReplicaError::*;

            let mut msg = notify::Message::from_str("error at ");

            let err: &dyn fmt::Display = match &self {
                Walk(err) => {
                    msg.push_info(&err.dir_path);
                    &err.kind
                },
            };

            msg.push_str(": ").push_str(err.to_smolstr());

            (notify::Level::Error, msg)
        }
    }

    impl notify::Error for UserNotLoggedInError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let mut msg = notify::Message::from_str(
                "need to be logged in to collaborate. You can log in by \
                 executing ",
            );
            msg.push_expected(":Mad login");
            (notify::Level::Error, msg)
        }
    }
}
