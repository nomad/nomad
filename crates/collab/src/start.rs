//! TODO: docs.

use core::convert::Infallible;
use core::marker::PhantomData;

use abs_path::{AbsPath, AbsPathBuf};
use auth::AuthInfos;
use collab_project::fs::{File as ProjectFile, Node as ProjectNode};
use collab_project::{Project, ProjectBuilder};
use collab_server::{SessionIntent, client};
use collab_types::{Peer, PeerId, puff};
use ed::action::AsyncAction;
use ed::command::ToCompletionFn;
use ed::fs::{self, Directory, File, Fs, FsNode, Metadata, Symlink};
use ed::notify::{self, Name};
use ed::shared::{MultiThreaded, Shared};
use ed::{Buffer, Context, Editor};
use futures_util::AsyncReadExt;
use fxhash::FxHashMap;
use puff::directory::LocalDirectoryId;
use puff::file::LocalFileId;
use smol_str::ToSmolStr;
use walkdir::FsExt;

use crate::collab::Collab;
use crate::config::Config;
use crate::editors::CollabEditor;
use crate::event_stream::{EventStream, EventStreamBuilder};
use crate::leave::StopChannels;
use crate::project::{
    IdMaps,
    NewProjectArgs,
    OverlappingProjectError,
    Projects,
};
use crate::root_markers;
use crate::session::Session;

/// TODO: docs.
pub type ProjectFilter<B> = walkdir::Either<
    <B as CollabEditor>::ProjectFilter,
    AllButOne<<B as Editor>::Fs>,
>;

type Markers = root_markers::GitDirectory;

/// The `Action` used to start a new collaborative editing session.
#[derive(cauchy::Clone)]
pub struct Start<Ed: CollabEditor> {
    auth_infos: Shared<Option<AuthInfos>>,
    config: Shared<Config>,
    projects: Projects<Ed>,
    stop_channels: StopChannels<Ed>,
}

impl<Ed: CollabEditor> AsyncAction<Ed> for Start<Ed> {
    const NAME: Name = "start";

    type Args = ();

    #[allow(clippy::too_many_lines)]
    async fn call(
        &mut self,
        _: Self::Args,
        ctx: &mut Context<Ed>,
    ) -> Result<(), StartError<Ed>> {
        let auth_infos =
            self.auth_infos.cloned().ok_or(StartError::UserNotLoggedIn)?;

        let buffer_id = ctx.with_borrowed(|ctx| {
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

        if !Ed::confirm_start(project_guard.root(), ctx).await {
            return Ok(());
        }

        let project_name = project_guard
            .root()
            .node_name()
            .ok_or(StartError::ProjectRootIsFsRoot)?;

        let server_addr = self.config.with(|c| c.server_address.clone());

        let (reader, writer) = Ed::connect_to_server(server_addr, ctx)
            .await
            .map_err(StartError::ConnectToServer)?
            .split();

        let github_handle = auth_infos.handle().clone();

        let knock = collab_server::Knock::<Ed::ServerConfig> {
            auth_infos: auth_infos.into(),
            session_intent: SessionIntent::StartNew(project_name.to_owned()),
        };

        let welcome = client::Knocker::new(reader, writer)
            .knock(knock)
            .await
            .map_err(StartError::Knock)?;

        let (project, event_stream, id_maps) =
            read_project(project_guard.root(), welcome.peer_id, ctx)
                .await
                .map_err(StartError::ReadProject)?;

        let project_handle = project_guard.activate(NewProjectArgs {
            agent_id: event_stream.agent_id(),
            host_id: welcome.host_id,
            id_maps,
            local_peer: Peer { id: welcome.peer_id, github_handle },
            remote_peers: welcome.other_peers,
            project,
            session_id: welcome.session_id,
        });

        let session = Session {
            event_stream,
            message_rx: welcome.rx,
            message_tx: welcome.tx,
            project_handle,
            stop_rx: self.stop_channels.insert(welcome.session_id),
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

impl<Ed: CollabEditor> From<&Collab<Ed>> for Start<Ed> {
    fn from(collab: &Collab<Ed>) -> Self {
        Self {
            auth_infos: collab.auth_infos.clone(),
            config: collab.config.clone(),
            projects: collab.projects.clone(),
            stop_channels: collab.stop_channels.clone(),
        }
    }
}

impl<Ed: CollabEditor> ToCompletionFn<Ed> for Start<Ed> {
    fn to_completion_fn(&self) {}
}

/// Searches for the root of the project containing the buffer with the given
/// ID.
async fn search_project_root<Ed: CollabEditor>(
    buffer_id: Ed::BufferId,
    ctx: &mut Context<Ed>,
) -> Result<AbsPathBuf, SearchProjectRootError<Ed>> {
    if let Some(lsp_res) = Ed::lsp_root(buffer_id.clone(), ctx).transpose() {
        return lsp_res.map_err(SearchProjectRootError::Lsp);
    }

    let buffer_path = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id.clone())
            .map(|buf| buf.path().into_owned())
            .ok_or(SearchProjectRootError::InvalidBufId(buffer_id))
    })?;

    let home_dir =
        Ed::home_dir(ctx).await.map_err(SearchProjectRootError::HomeDir)?;

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

/// Constructs a [`Project`] by reading the contents of the file or directory
/// at the given path.
#[allow(clippy::too_many_lines)]
async fn read_project<Ed: CollabEditor>(
    root_path: &AbsPath,
    local_id: PeerId,
    ctx: &mut Context<Ed>,
) -> Result<
    (Project, EventStream<Ed, ProjectFilter<Ed>>, IdMaps<Ed>),
    ReadProjectError<Ed>,
> {
    let fs = ctx.fs();

    let root_node = fs
        .node_at_path(root_path)
        .await
        .map_err(ReadProjectError::GetRoot)?
        .ok_or_else(|| {
            ReadProjectError::NoNodeAtRootPath(root_path.to_owned())
        })?;

    let (project_root, project_filter) = match root_node {
        FsNode::Directory(dir) => {
            let filter = Ed::project_filter(&dir, ctx);
            (dir, walkdir::Either::Left(filter))
        },
        // The user wants to collaborate on a single file. The root must always
        // be a directory, so we just use its parent together with a filter
        // that ignores all its siblings.
        FsNode::File(file) => {
            let parent =
                file.parent().await.map_err(ReadProjectError::FileParent)?;
            let filter = AllButOne::<Ed::Fs> { id: file.id() };
            (parent, walkdir::Either::Right(filter))
        },
        FsNode::Symlink(_) => {
            return Err(ReadProjectError::RootIsSymlink(root_path.to_owned()));
        },
    };

    let (project, stream_builder, node_id_maps) = ctx
        .spawn_background(async move {
            let walker = fs.walk(&project_root).filter(project_filter);

            let mut project_builder = Project::builder(local_id);
            let project_builder_mut = Shared::new(&mut project_builder);

            let mut stream_builder = EventStreamBuilder::new(&project_root);
            let stream_builder_mut = Shared::new(&mut stream_builder);

            let mut node_id_maps = NodeIdMaps::default();
            let node_id_maps_mut = Shared::new(&mut node_id_maps);

            walker
                .for_each(async |parent_path, node_meta| {
                    read_node(
                        parent_path,
                        node_meta,
                        &project_root,
                        &project_builder_mut,
                        &stream_builder_mut,
                        &node_id_maps_mut,
                        &fs,
                    )
                    .await
                })
                .await
                .map_err(ReadProjectError::WalkRoot)?;

            Ok((
                project_builder.build(),
                stream_builder.push_filter(walker.into_inner().into_filter()),
                node_id_maps,
            ))
        })
        .await?;

    let mut event_stream = stream_builder.build(ctx);

    let mut id_maps = IdMaps::default();

    // Start watching the opened buffers that are part of the project.
    ctx.for_each_buffer(|buffer| {
        let buffer_path = buffer.path();

        let Some(path_in_proj) = buffer_path.strip_prefix(root_path) else {
            return;
        };

        let file_id = match project.node_at_path(path_in_proj) {
            Some(ProjectNode::File(ProjectFile::Text(file))) => file.id(),
            _ => return,
        };

        if let Some(node_id) = node_id_maps.file2node.get(&file_id) {
            event_stream.watch_buffer(&buffer, node_id.clone());
            id_maps.buffer2file.insert(buffer.id(), file_id);
            id_maps.file2buffer.insert(file_id, buffer.id());
        }
    });

    id_maps.node2dir = node_id_maps.node2dir;
    id_maps.node2file = node_id_maps.node2file;

    Ok((project, event_stream, id_maps))
}

/// TODO: docs.
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
async fn read_node<Fs: fs::Fs>(
    parent_path: &AbsPath,
    node_meta: Fs::Metadata,
    project_root: &Fs::Directory,
    project_builder: &Shared<&mut ProjectBuilder, MultiThreaded>,
    stream_builder: &Shared<&mut EventStreamBuilder<Fs>, MultiThreaded>,
    node_id_maps: &Shared<&mut NodeIdMaps<Fs>, MultiThreaded>,
    fs: &Fs,
) -> Result<(), ReadNodeError<Fs>> {
    let node_name = node_meta.name().map_err(ReadNodeError::NodeName)?;

    let node_path = parent_path.join(node_name);

    let Some(node) =
        fs.node_at_path(&node_path).await.map_err(ReadNodeError::GetNode)?
    else {
        return Ok(());
    };

    let path_in_project = node_path
        .strip_prefix(project_root.path())
        .expect("node is under the root dir");

    let push_res = match &node {
        FsNode::Directory(dir) => {
            stream_builder.with_mut(|builder| builder.push_directory(dir));
            project_builder
                .with_mut(|builder| builder.push_directory(path_in_project))
                .map(|dir_id| {
                    node_id_maps.with_mut(|maps| {
                        maps.node2dir.insert(dir.id(), dir_id);
                    })
                })
        },

        FsNode::File(file) => {
            stream_builder.with_mut(|builder| builder.push_file(file));

            let contents =
                file.read().await.map_err(ReadNodeError::ReadFile)?;

            match str::from_utf8(&contents) {
                Ok(contents) => project_builder.with_mut(|builder| {
                    builder.push_text_file(path_in_project, contents)
                }),
                Err(_) => project_builder.with_mut(|builder| {
                    builder.push_binary_file(path_in_project, contents)
                }),
            }
            .map(|file_id| {
                node_id_maps.with_mut(|maps| {
                    maps.file2node.insert(file_id, file.id());
                    maps.node2file.insert(file.id(), file_id);
                })
            })
        },
        FsNode::Symlink(symlink) => {
            let target_path = symlink
                .read_path()
                .await
                .map_err(ReadNodeError::ReadSymlink)?;

            project_builder
                .with_mut(|builder| {
                    builder.push_symlink(path_in_project, target_path)
                })
                .map(|file_id| {
                    node_id_maps.with_mut(|maps| {
                        maps.file2node.insert(file_id, symlink.id());
                        maps.node2file.insert(symlink.id(), file_id);
                    })
                })
        },
    };

    if push_res.is_err() {
        Err(ReadNodeError::DuplicateNodeAtPath(node_path))
    } else {
        Ok(())
    }
}

/// The type of error that can occur when [`Start`]ing a session fails.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum StartError<Ed: CollabEditor> {
    /// TODO: docs.
    ConnectToServer(Ed::ConnectToServerError),

    /// TODO: docs.
    Knock(client::KnockError<Ed::ServerConfig>),

    /// TODO: docs.
    NoBufferFocused,

    /// TODO: docs.
    OverlappingProject(OverlappingProjectError),

    /// TODO: docs.
    ProjectRootIsFsRoot,

    /// TODO: docs.
    ReadProject(ReadProjectError<Ed>),

    /// TODO: docs.
    SearchProjectRoot(SearchProjectRootError<Ed>),

    /// TODO: docs.
    UserNotLoggedIn,
}

/// The type of error that can occur when reading a [`FsNode`] fails.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum ReadNodeError<Fs: fs::Fs> {
    /// TODO: docs.
    DuplicateNodeAtPath(AbsPathBuf),

    /// TODO: docs.
    GetNode(Fs::NodeAtPathError),

    /// TODO: docs.
    NodeName(fs::MetadataNameError),

    /// TODO: docs.
    ReadFile(<Fs::File as File>::ReadError),

    /// TODO: docs.
    ReadSymlink(<Fs::Symlink as Symlink>::ReadError),
}

/// The type of error that can occur when reading a [`Project`] fails.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum ReadProjectError<Ed: CollabEditor> {
    /// TODO: docs.
    GetRoot(<Ed::Fs as Fs>::NodeAtPathError),

    /// TODO: docs.
    NoNodeAtRootPath(AbsPathBuf),

    /// TODO: docs.
    FileParent(<<Ed::Fs as Fs>::File as File>::ParentError),

    /// TODO: docs.
    ReadNode(ReadNodeError<Ed::Fs>),

    /// TODO: docs.
    RootIsSymlink(AbsPathBuf),

    /// TODO: docs.
    #[allow(clippy::type_complexity)]
    WalkRoot(
        walkdir::WalkError<
            Ed::Fs,
            walkdir::Filtered<ProjectFilter<Ed>, Ed::Fs>,
            ReadNodeError<Ed::Fs>,
        >,
    ),
}

/// TODO: docs.
#[derive(cauchy::Debug, cauchy::PartialEq)]
pub enum SearchProjectRootError<Ed: CollabEditor> {
    /// TODO: docs.
    BufNameNotAbsolutePath(String),

    /// TODO: docs.
    CouldntFindRoot(AbsPathBuf),

    /// TODO: docs.
    FindRoot(root_markers::FindRootError<Ed::Fs, Markers>),

    /// TODO: docs.
    HomeDir(Ed::HomeDirError),

    /// TODO: docs.
    InvalidBufId(Ed::BufferId),

    /// TODO: docs.
    Lsp(Ed::LspRootError),
}

/// A [`walkdir::Filter`] that filters out every node but one.
pub struct AllButOne<Fs: fs::Fs> {
    id: Fs::NodeId,
}

#[derive(cauchy::Default)]
struct NodeIdMaps<Fs: fs::Fs> {
    file2node: FxHashMap<LocalFileId, Fs::NodeId>,
    node2dir: FxHashMap<Fs::NodeId, LocalDirectoryId>,
    node2file: FxHashMap<Fs::NodeId, LocalFileId>,
}

impl<Fs: fs::Fs> walkdir::Filter<Fs> for AllButOne<Fs> {
    type Error = Infallible;

    async fn should_filter(
        &self,
        _: &AbsPath,
        node_meta: &impl Metadata<Fs = Fs>,
    ) -> Result<bool, Self::Error> {
        Ok(node_meta.id() != self.id)
    }
}

/// TODO: docs.
pub(crate) struct UserNotLoggedInError<B>(PhantomData<B>);

/// TODO: docs.
struct NoBufferFocusedError<B>(PhantomData<B>);

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

impl<Ed: CollabEditor> notify::Error for StartError<Ed> {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::ConnectToServer(err) => err.to_message(),
            Self::Knock(_err) => todo!(),
            Self::NoBufferFocused => {
                NoBufferFocusedError::<Ed>::new().to_message()
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
                UserNotLoggedInError::<Ed>::new().to_message()
            },
        }
    }
}

impl<Ed: CollabEditor> notify::Error for ReadProjectError<Ed> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}

impl<B> notify::Error for NoBufferFocusedError<B> {
    default fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Off, notify::Message::new())
    }
}

impl<Ed: CollabEditor> notify::Error for SearchProjectRootError<Ed> {
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
    use neovim::Neovim;

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
            todo!();
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
                InvalidBufId(buf_id) => {
                    msg.push_str("there's no buffer whose handle is ")
                        .push_invalid(buf_id.bufnr().to_smolstr());
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

#[cfg(feature = "benches")]
pub mod benches {
    //! TODO: docs.

    use super::*;

    /// TODO: docs.
    #[inline]
    pub async fn read_project<Ed>(
        project_root: <Ed::Fs as fs::Fs>::Directory,
        ctx: &mut Context<Ed>,
    ) -> Result<(), ReadProjectError<Ed>>
    where
        Ed: CollabEditor,
    {
        super::read_project(project_root.path(), PeerId::new(1), ctx)
            .await
            .map(|_| ())
    }
}
