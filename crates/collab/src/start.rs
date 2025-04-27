//! TODO: docs.

use core::marker::PhantomData;

use abs_path::{AbsPath, AbsPathBuf};
use auth::AuthInfos;
use collab_project::{Project, ProjectBuilder};
use collab_server::message::PeerId;
use collab_server::{SessionIntent, client};
use ed::AsyncCtx;
use ed::action::AsyncAction;
use ed::backend::Buffer;
use ed::command::ToCompletionFn;
use ed::fs::{self, Directory, File, Fs, FsNode, Metadata, Symlink};
use ed::notify::{self, Name};
use ed::shared::{MultiThreaded, Shared};
use futures_util::AsyncReadExt;
use smol_str::ToSmolStr;
use walkdir::FsExt;

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
            read_project(project_guard.root(), welcome.peer_id, ctx)
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

/// Constructs a [`Project`] by reading the contents of the directory at the
/// given path.
async fn read_project<B: CollabBackend>(
    project_root: &AbsPath,
    local_id: PeerId,
    ctx: &mut AsyncCtx<'_, B>,
) -> Result<(Project, EventRx<B>), ReadProjectError<B>> {
    let fs = ctx.fs();

    let root_node = fs
        .node_at_path(project_root)
        .await
        .map_err(ReadProjectError::GetRoot)?
        .ok_or_else(|| {
            ReadProjectError::NoNodeAtRootPath(project_root.to_owned())
        })?;

    let root_dir = match root_node {
        FsNode::Directory(dir) => dir,
        FsNode::File(_) => todo!(),
        FsNode::Symlink(_) => todo!(),
    };

    let fs_filter = B::project_filter(&root_dir, ctx);

    let event_rx = EventRx::<B>::new(&root_dir, ctx);

    let (project, _fs_filter) = ctx
        .spawn_background(async move {
            let walker = fs.walk(&root_dir).filter(fs_filter);
            let project_root = root_dir.path();
            let mut project_builder = Project::builder(local_id);
            let builder_mut = Shared::new(&mut project_builder);

            walker
                .for_each(async |parent_path, node_meta| {
                    read_node(
                        project_root,
                        parent_path,
                        node_meta,
                        &builder_mut,
                        &fs,
                    )
                    .await
                })
                .await
                .map_err(ReadProjectError::WalkRoot)?;

            Ok((project_builder.build(), walker.into_inner().into_filter()))
        })
        .await?;

    Ok((project, event_rx))
}

/// TODO: docs.
async fn read_node<Fs: fs::Fs>(
    project_root: &AbsPath,
    parent_path: &AbsPath,
    node_meta: Fs::Metadata,
    project_builder: &Shared<&mut ProjectBuilder, MultiThreaded>,
    fs: &Fs,
) -> Result<(), ReadNodeError<Fs>> {
    let node_name = node_meta.name().map_err(ReadNodeError::NodeName)?;

    let node_path = parent_path.join(&node_name);

    let Some(node) =
        fs.node_at_path(&node_path).await.map_err(ReadNodeError::GetNode)?
    else {
        return Ok(());
    };

    let path_in_project = node_path
        .strip_prefix(project_root)
        .expect("node is under the root dir");

    let _maybe_err = match node {
        FsNode::Directory(_) => project_builder
            .with_mut(|builder| builder.push_directory(path_in_project).err()),

        FsNode::File(file) => {
            let contents =
                file.read().await.map_err(ReadNodeError::ReadFile)?;

            match str::from_utf8(&contents) {
                Ok(contents) => project_builder.with_mut(|builder| {
                    builder.push_text_file(path_in_project, contents).err()
                }),
                Err(_) => project_builder.with_mut(|builder| {
                    builder.push_binary_file(path_in_project, contents).err()
                }),
            }
        },
        FsNode::Symlink(symlink) => {
            let target_path = symlink
                .read_path()
                .await
                .map_err(ReadNodeError::ReadSymlink)?;

            project_builder.with_mut(|builder| {
                builder.push_symlink(path_in_project, target_path).err()
            })
        },
    };

    Ok(())
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
    ReadProject(ReadProjectError<B>),

    /// TODO: docs.
    SearchProjectRoot(SearchProjectRootError<B>),

    /// TODO: docs.
    UserNotLoggedIn,
}

/// The type of error that can occur when [read](`read_node`)ing a node fails.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum ReadNodeError<Fs: fs::Fs> {
    /// TODO: docs.
    GetNode(Fs::NodeAtPathError),

    /// TODO: docs.
    NodeName(fs::MetadataNameError),

    /// TODO: docs.
    ReadFile(<Fs::File as File>::ReadError),

    /// TODO: docs.
    ReadSymlink(<Fs::Symlink as Symlink>::ReadError),
}

/// The type of error that can occur when [read](`read_project`)ing a project
/// fails.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum ReadProjectError<B: CollabBackend> {
    /// TODO: docs.
    GetRoot(<B::Fs as Fs>::NodeAtPathError),

    /// TODO: docs.
    NoNodeAtRootPath(AbsPathBuf),

    /// TODO: docs.
    ReadNode(ReadNodeError<B::Fs>),

    /// TODO: docs.
    WalkRoot(
        walkdir::WalkError<
            B::Fs,
            walkdir::Filtered<B::ProjectFilter, B::Fs>,
            ReadNodeError<B::Fs>,
        >,
    ),
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

impl<B: CollabBackend> notify::Error for ReadProjectError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}

impl<B> notify::Error for NoBufferFocusedError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
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
