//! TODO: docs.

use core::cell::RefCell;
use core::marker::PhantomData;
use std::sync::{Arc, Mutex};

use auth::AuthInfos;
use collab_project::{Project, ProjectBuilder};
use collab_server::message::{Peer, PeerId};
use collab_server::{SessionIntent, client};
use concurrent_queue::{ConcurrentQueue, PushError};
use ed::action::AsyncAction;
use ed::backend::{Backend, Buffer};
use ed::command::ToCompletionFn;
use ed::fs::{
    self,
    AbsPath,
    AbsPathBuf,
    Directory,
    File,
    Fs,
    FsNode,
    Metadata,
    MetadataNameError,
    NodeKind,
};
use ed::notify::{self, Name};
use ed::{AsyncCtx, ByteOffset, Shared};
use futures_util::AsyncReadExt;
use smol_str::ToSmolStr;
use walkdir::WalkDir;

use crate::backend::CollabBackend;
use crate::collab::Collab;
use crate::config::Config;
use crate::leave::StopChannels;
use crate::project::{NewProjectArgs, OverlappingProjectError, Projects};
use crate::root_markers;
use crate::session::{EventRx, Session};

type Markers = root_markers::GitDirectory;

/// The `Action` used to start a new collaborative editing session.
#[derive(cauchy::Clone)]
pub struct Start<B: CollabBackend> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    projects: Projects<B>,
    stop_channels: StopChannels<B>,
}

impl<B: CollabBackend> AsyncAction<B> for Start<B> {
    const NAME: Name = "start";

    type Args = ();

    #[allow(clippy::too_many_lines)]
    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<(), StartError<B>> {
        let auth_infos =
            self.auth_infos.cloned().ok_or(StartError::UserNotLoggedIn)?;

        let buffer_id = ctx.with_ctx(|ctx| {
            ctx.current_buffer()
                .map(|buf| buf.id())
                .ok_or(StartError::NoBufferFocused)
        })?;

        let project_root = search_project_root(buffer_id, ctx)
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

        let server_addr = self.config.with(|c| c.server_address.clone());

        let (reader, writer) = B::connect_to_server(server_addr, ctx)
            .await
            .map_err(StartError::ConnectToServer)?
            .split();

        let peer_handle = auth_infos.handle().clone();

        let knock = collab_server::Knock::<B::ServerConfig> {
            auth_infos: auth_infos.into(),
            session_intent: SessionIntent::StartNew(project_name.to_owned()),
        };

        let welcome = client::Knocker::new(reader, writer)
            .knock(knock)
            .await
            .map_err(StartError::Knock)?;

        let (project, event_rx) =
            read_replica2(project_guard.root(), welcome.peer_id, ctx)
                .await
                .map_err(StartError::ReadProject)?;

        let project_handle = project_guard.activate(NewProjectArgs {
            host_id: welcome.host_id,
            peer_handle,
            remote_peers: welcome.other_peers,
            project,
            session_id: welcome.session_id,
        });

        let session = Session {
            event_rx,
            message_rx: welcome.rx,
            message_tx: welcome.tx,
            project_handle,
            stop_rx: self
                .stop_channels
                .insert(welcome.session_id)
                .into_stream(),
        };

        ctx.spawn_local(async move |ctx| {
            if let Err(err) = session.run(ctx).await {
                ctx.emit_err(err);
            }
        })
        .detach();

        Ok(())
    }
}

async fn read_replica2<B: CollabBackend>(
    project_root: &AbsPath,
    peer_id: PeerId,
    ctx: &mut AsyncCtx<'_, B>,
) -> Result<(Project, EventRx<B>), ReadProjectError2<B>> {
    let root = match ctx.fs().node_at_path(project_root).await {
        Ok(Some(node)) => node,
        _ => todo!(),
    };

    let mut event_rx = EventRx::<B>::new(&root, ctx);

    event_rx.watch(&root, ctx);

    let replica = match &root {
        FsNode::Directory(dir) => {
            // let worktree = Arc::new(Mutex::new(Worktree::new()));
            let event_rx = RefCell::new(&mut event_rx);

            ctx.fs()
                .filter(B::fs_filter(dir.path(), ctx))
                .for_each(dir.path(), async |parent_path, meta| {
                    let node_name =
                        meta.name().map_err(VisitNodeError::NodeName)?;

                    let Some(node) = ctx
                        .fs()
                        .node_at_path(parent_path.join(&node_name))
                        .await
                        .map_err(VisitNodeError::Node)?
                    else {
                        return Ok(());
                    };

                    if node.kind().is_symlink() {
                        return Ok(());
                    }

                    event_rx.borrow_mut().watch(&node, ctx);

                    ctx.spawn_background(async move {
                        let child = match &node {
                            FsNode::Directory(_) => {
                                // eerie::Directory::new(node_name)
                            },
                            FsNode::File(_file) => {
                                // let file_contents = file.read().await?;
                                // match str::from_utf8(&*file_contents) {
                                //     Ok(contents) => eerie::File::new_text(
                                //         node_name, contents,
                                //     ),
                                //     Err(_) => eerie::File::new_binary(
                                //         node_name, contents,
                                //     ),
                                // }
                            },
                            FsNode::Symlink(_) => {
                                unreachable!("already checked")
                            },
                        };

                        // worktree
                        //     .lock()
                        //     .dir_at_path_mut(parent_path)
                        //     .expect("parent exists")
                        //     .create_child(child);
                    })
                    .await;

                    Ok(())
                })
                .await
                .map_err(ReadProjectError2::Walk)?;

            todo!();
        },
        FsNode::File(_) => todo!(),
        FsNode::Symlink(_) => todo!(),
    };

    Ok((replica, event_rx))
}

enum ReadProjectError2<B: CollabBackend> {
    Walk(WalkError2<B>),
}

/// TODO: docs.
pub type WalkError2<B> = walkdir::WalkError<
    <B as Backend>::Fs,
    walkdir::Filtered<<B as CollabBackend>::FsFilter, <B as Backend>::Fs>,
    VisitNodeError<B>,
>;

/// TODO: docs.
pub enum VisitNodeError<B: CollabBackend> {
    /// TODO: docs.
    Node(<<B as Backend>::Fs as fs::Fs>::NodeAtPathError),

    /// TODO: docs.
    NodeName(MetadataNameError),
}

// async fn read_replica<B>(
//     peer_id: PeerId,
//     project_root: AbsPathBuf,
//     ctx: &mut AsyncCtx<'_, B>,
// ) -> Result<Replica, ReadReplicaError<B>>
// where
//     B: CollabBackend,
// {
//     enum PushNode {
//         File(AbsPathBuf, ByteOffset),
//         Directory(AbsPathBuf),
//     }
//
//     let fs = ctx.fs();
//     let res = async move {
//         let op_queue = Arc::new(ConcurrentQueue::unbounded());
//         let op_queue2 = Arc::clone(&op_queue);
//         let handler =
//             async move |dir_path: &AbsPath,
//                         entry: <B::Fs as fs::Fs>::Metadata| {
//                 let entry_path = dir_path.join(entry.name()?);
//                 let op = match entry.node_kind() {
//                     NodeKind::File => {
//                         PushNode::File(entry_path, entry.byte_len())
//                     },
//                     NodeKind::Directory => PushNode::Directory(entry_path),
//                     NodeKind::Symlink => return Ok(()),
//                 };
//                 match op_queue2.push(op) {
//                     Ok(()) => Ok(()),
//                     Err(PushError::Full(_)) => unreachable!("unbounded"),
//                     Err(PushError::Closed(_)) => unreachable!("never closed"),
//                 }
//             };
//
//         fs.for_each(&project_root, handler).await?;
//
//         let mut builder = ReplicaBuilder::new(peer_id);
//         while let Ok(op) = op_queue.pop() {
//             let _ = match op {
//                 PushNode::File(path, len) => {
//                     builder.push_file(path, len.into_u64())
//                 },
//                 PushNode::Directory(path) => builder.push_directory(path),
//             };
//         }
//         Ok::<_, WalkError<B>>(builder)
//     };
//
//     let mut builder = res.await.map_err(ReadReplicaError::Walk)?;
//
//     // Update the lengths of the open buffers.
//     //
//     // FIXME: what if a buffer was edited and already closed?
//     ctx.for_each_buffer(|buffer| {
//         if let Some(mut file) = <&AbsPath>::try_from(&*buffer.name())
//             .ok()
//             .and_then(|buffer_path| builder.file_mut(buffer_path))
//         {
//             file.set_len(buffer.byte_len().into());
//         }
//     });
//
//     Ok(builder.build())
// }

/// Searches for the root of the project containing the buffer with the given
/// ID.
async fn search_project_root<B: CollabBackend>(
    buffer_id: B::BufferId,
    ctx: &mut AsyncCtx<'_, B>,
) -> Result<AbsPathBuf, SearchProjectRootError<B>> {
    if let Some(lsp_res) = B::lsp_root(buffer_id.clone(), ctx).transpose() {
        return lsp_res.map_err(SearchProjectRootError::Lsp);
    }

    let buffer_name = ctx.with_ctx(|ctx| {
        ctx.buffer(buffer_id.clone())
            .ok_or(SearchProjectRootError::InvalidBufId(buffer_id))
            .map(|buf| buf.name().into_owned())
    })?;

    let buffer_path = buffer_name.parse::<AbsPathBuf>().map_err(|_| {
        SearchProjectRootError::BufNameNotAbsolutePath(buffer_name)
    })?;

    let home_dir =
        B::home_dir(ctx).await.map_err(SearchProjectRootError::HomeDir)?;

    let args = root_markers::FindRootArgs {
        marker: root_markers::GitDirectory,
        start_from: &buffer_path,
        stop_at: Some(&home_dir),
    };

    let mut fs = ctx.fs();

    if let Some(res) = args.find(&mut fs).await.transpose() {
        return res.map_err(SearchProjectRootError::FindRoot);
    }

    buffer_path
        .parent()
        .map(ToOwned::to_owned)
        .ok_or(SearchProjectRootError::CouldntFindRoot(buffer_path))
}

/// The type of error that can occur when [`Start`]ing a session fails.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum StartError<B: CollabBackend> {
    /// TODO: docs.
    ConnectToServer(B::ConnectToServerError),

    /// TODO: docs.
    Knock(client::KnockError<B::ServerConfig>),

    /// TODO: docs.
    NoBufferFocused,

    /// TODO: docs.
    OverlappingProject(OverlappingProjectError),

    /// TODO: docs.
    ProjectRootIsFsRoot,

    /// TODO: docs.
    ReadProject(ReadProjectError2<B>),

    /// TODO: docs.
    SearchProjectRoot(SearchProjectRootError<B>),

    /// TODO: docs.
    UserNotLoggedIn,
}

/// TODO: docs.
pub type WalkError<B> = walkdir::WalkError<
    <B as Backend>::Fs,
    <B as Backend>::Fs,
    MetadataNameError,
>;

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum ReadProjectError<B: CollabBackend> {
    /// TODO: docs.
    Walk(WalkError<B>),
}

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum SearchProjectRootError<B: CollabBackend> {
    /// TODO: docs.
    BufNameNotAbsolutePath(String),

    /// TODO: docs.
    CouldntFindRoot(fs::AbsPathBuf),

    /// TODO: docs.
    FindRoot(root_markers::FindRootError<B::Fs, Markers>),

    /// TODO: docs.
    HomeDir(B::HomeDirError),

    /// TODO: docs.
    InvalidBufId(B::BufferId),

    /// TODO: docs.
    Lsp(B::LspRootError),
}

/// TODO: docs.
pub(crate) struct UserNotLoggedInError<B>(PhantomData<B>);

/// TODO: docs.
struct NoBufferFocusedError<B>(PhantomData<B>);

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

impl<B: CollabBackend> notify::Error for StartError<B> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::ConnectToServer(err) => err.to_message(),
            Self::Knock(_err) => todo!(),
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
            Self::ReadProject(err) => err.to_message(),
            Self::SearchProjectRoot(err) => err.to_message(),
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

impl<B: CollabBackend> notify::Error for ReadProjectError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        let msg = match self {
            ReadProjectError::Walk(err) => notify::Message::from_display(err),
        };
        (notify::Level::Error, msg)
    }
}

impl<B: CollabBackend> notify::Error for ReadProjectError2<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}

impl<B: CollabBackend> notify::Error for SearchProjectRootError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        use SearchProjectRootError::*;

        let mut msg = notify::Message::new();

        match self {
            BufNameNotAbsolutePath(str) => {
                msg.push_str("buffer name ")
                    .push_invalid(str)
                    .push_str(" is not an absolute path");
            },
            CouldntFindRoot(abs_path_buf) => {
                msg.push_str("couldn't find project root for buffer at ")
                    .push_info(abs_path_buf);
            },
            FindRoot(err) => {
                msg.push_str(err.to_smolstr());
            },
            HomeDir(err) => return err.to_message(),
            InvalidBufId(buf_id) => {
                msg.push_str("there's no buffer whose handle is ")
                    .push_invalid(format!("{buf_id:?}"));
            },
            Lsp(err) => return err.to_message(),
        }

        (notify::Level::Error, msg)
    }
}

impl<B> notify::Error for UserNotLoggedInError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

#[cfg(feature = "neovim")]
mod neovim_error_impls {
    use ed::neovim::Neovim;

    use super::*;

    impl notify::Error for NoBufferFocusedError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let msg = "couldn't determine path to project root. Either move \
                       the cursor to a text buffer, or pass one explicitly";
            (notify::Level::Error, notify::Message::from_str(msg))
        }
    }

    impl notify::Error for ReadProjectError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let msg = match &self {
                Self::Walk(err) => notify::Message::from_display(err),
            };

            (notify::Level::Error, msg)
        }
    }

    impl notify::Error for SearchProjectRootError<Neovim> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            use SearchProjectRootError::*;

            let mut msg = notify::Message::new();

            match &self {
                BufNameNotAbsolutePath(buf_name) => {
                    if buf_name.is_empty() {
                        msg.push_str("the current buffer's name is empty");
                    } else {
                        msg.push_str("buffer name ")
                            .push_invalid(buf_name)
                            .push_str(" is not an absolute path");
                    }
                },
                Lsp(err) => return err.to_message(),
                FindRoot(err) => return err.to_message(),
                HomeDir(err) => return err.to_message(),
                InvalidBufId(buf) => {
                    msg.push_str("there's no buffer whose handle is ")
                        .push_invalid(buf.handle().to_smolstr());
                },
                CouldntFindRoot(buffer_path) => {
                    msg.push_str("couldn't find project root for buffer at ")
                        .push_info(buffer_path.to_smolstr())
                        .push_str(", please pass one explicitly");
                },
            }

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
