//! TODO: docs.

use core::convert::Infallible;
use std::borrow::Cow;

use abs_path::{AbsPath, AbsPathBuf};
use auth::AuthState;
use collab_project::fs::{FileMut, NodeMut};
use collab_project::{Project, ProjectBuilder};
use collab_server::client as collab_client;
use collab_types::{PeerId, puff};
use editor::command::ToCompletionFn;
use editor::module::{AsyncAction, Module};
use editor::shared::{MultiThreaded, Shared};
use editor::{Access, Buffer, Context, Cursor, Editor};
use either::Either;
use fs::walk::FsExt;
use fs::{Directory, File as _, Fs, Metadata, Node, Symlink};
use futures_util::AsyncReadExt;
use fxhash::FxHashMap;
use puff::directory::LocalDirectoryId;
use puff::file::LocalFileId;

use crate::collab::Collab;
use crate::config::Config;
use crate::editors::CollabEditor;
use crate::event_stream::{EventStream, EventStreamBuilder};
use crate::leave::StopChannels;
use crate::pausable_stream::PausableStream;
use crate::peers::RemotePeers;
use crate::progress::{ProgressReporter, StartState};
use crate::project::{self, IdMaps};
use crate::root_markers;
use crate::session::{Session, SessionInfos, Sessions};

/// TODO: docs.
pub type ProjectFilter<Ed> =
    Either<<Ed as CollabEditor>::ProjectFilter, AllButOne<<Ed as Editor>::Fs>>;

type Markers = root_markers::GitDirectory;

/// The `Action` used to start a new collaborative editing session.
#[derive(cauchy::Clone)]
pub struct Start<Ed: CollabEditor> {
    auth_state: AuthState,
    config: Shared<Config>,
    sessions: Sessions<Ed>,
    stop_channels: StopChannels<Ed>,
}

impl<Ed: CollabEditor> Start<Ed> {
    /// Constructs a [`Project`] by reading the contents of the file or
    /// directory at the given path.
    #[allow(clippy::too_many_lines)]
    pub async fn read_project(
        root_path: &AbsPath,
        local_id: PeerId,
        ctx: &mut Context<Ed>,
    ) -> Result<(Project, EventStream<Ed>, IdMaps<Ed>), ReadProjectError<Ed>>
    {
        let fs = ctx.fs();

        let root_node = fs
            .node_at_path(root_path)
            .await
            .map_err(ReadProjectError::GetRoot)?
            .ok_or_else(|| {
                ReadProjectError::NoNodeAtRootPath(root_path.to_owned())
            })?;

        let (project_root, project_filter) = match root_node {
            Node::Directory(dir) => {
                let filter = Ed::project_filter(&dir, ctx)
                    .map_err(ReadProjectError::ProjectFilter)?;
                (dir, Either::Left(filter))
            },
            // The user wants to collaborate on a single file. The root must
            // always be a directory, so we just use its parent together with a
            // filter that ignores all its siblings.
            Node::File(file) => {
                let parent = file
                    .parent()
                    .await
                    .map_err(ReadProjectError::FileParent)?;
                let filter = AllButOne::<Ed::Fs> { id: file.id() };
                (parent, Either::Right(filter))
            },
            Node::Symlink(_) => {
                return Err(ReadProjectError::RootIsSymlink(
                    root_path.to_owned(),
                ));
            },
        };

        let (mut project, stream_builder, node_id_maps) = ctx
            .spawn_background(async move {
                let walker = fs.walk(&project_root).filter(project_filter);

                let mut project_builder = Project::builder(local_id);
                let project_builder_mut = Shared::new(&mut project_builder);

                let mut stream_builder =
                    EventStreamBuilder::new(&project_root);
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
                    stream_builder
                        .push_filter(walker.into_inner().into_filter()),
                    node_id_maps,
                ))
            })
            .await?;

        let mut event_stream = stream_builder.build(ctx);

        let mut id_maps = IdMaps::default();

        // Start watching the opened buffers that are part of the project.
        ctx.for_each_buffer(|mut buffer| {
            let buffer_path = buffer.path();

            let Some(path_in_proj) = buffer_path.strip_prefix(root_path)
            else {
                return;
            };

            let Some(NodeMut::File(FileMut::Text(mut file))) =
                project.node_at_path_mut(path_in_proj)
            else {
                return;
            };

            let Some(node_id) = node_id_maps.file2node.get(&file.local_id())
            else {
                return;
            };

            let buffer_id = buffer.id();
            event_stream.watch_buffer(&mut buffer, node_id.clone());
            id_maps.buffer2file.insert(buffer_id.clone(), file.local_id());
            id_maps.file2buffer.insert(file.local_id(), buffer_id);

            buffer.for_each_cursor(|mut cursor| {
                let (cursor_id, _) = file.create_cursor(cursor.byte_offset());
                event_stream.watch_cursor(&mut cursor);
                id_maps.cursor2cursor.insert(cursor.id(), cursor_id);
            });

            // buffer.for_each_selection(|selection| {
            //     let byte_range = selection.byte_range();
            //     let (selection_id, _) = file.create_selection(byte_range);
            //     id_maps
            //         .selection2selection
            //         .insert(selection.id(), selection_id);
            // });
        });

        id_maps.node2dir = node_id_maps.node2dir;
        id_maps.node2file = node_id_maps.node2file;

        Ok((project, event_stream, id_maps))
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) async fn call_inner(
        &self,
        progress_reporter: &mut impl ProgressReporter<Ed, Self>,
        ctx: &mut Context<Ed>,
    ) -> Result<SessionInfos<Ed>, StartError<Ed>> {
        let jwt = self
            .auth_state
            .with(Clone::clone)
            .ok_or(StartError::UserNotLoggedIn)?;

        let buffer_id = ctx.with_borrowed(|ctx| {
            ctx.current_buffer()
                .map(|buf| buf.id())
                .ok_or(StartError::NoBufferFocused)
        })?;

        let project_root = search_project_root(buffer_id, ctx)
            .await
            .map_err(StartError::SearchProjectRoot)?;

        if !Ed::confirm_start(&project_root, ctx).await {
            return Err(StartError::UserDidNotConfirm);
        }

        let project_name =
            project_root.node_name().ok_or(StartError::ProjectRootIsFsRoot)?;

        let server_addr = self.config.with(|c| c.server_address.clone());

        progress_reporter.report_progress(
            StartState::ConnectingToServer(server_addr.borrow()),
            ctx,
        );

        let (reader, writer) = Ed::connect_to_server(server_addr, ctx)
            .await
            .map_err(StartError::ConnectToServer)?
            .split();

        let knock = collab_client::Knock::<Ed::ServerParams> {
            auth_infos: jwt.into(),
            session_intent: collab_client::SessionIntent::StartNew(
                project_name.to_owned(),
            ),
        };

        progress_reporter.report_progress(StartState::StartingSession, ctx);

        let welcome = collab_client::knock(reader, writer, knock)
            .await
            .map_err(StartError::Knock)?;

        let local_peer = welcome.peer;

        progress_reporter.report_progress(
            StartState::ReadingProject(Cow::Borrowed(&project_root)),
            ctx,
        );

        let (project, event_stream, id_maps) =
            Self::read_project(&project_root, local_peer.id, ctx)
                .await
                .map_err(StartError::ReadProject)?;


        let remote_peers = RemotePeers::new(welcome.other_peers, &project);

        let project = project::Project {
            agent_id: event_stream.agent_id(),
            id_maps,
            inner: project,
            local_peer: local_peer.clone(),
            peer_selections: FxHashMap::default(),
            peer_tooltips: FxHashMap::default(),
            remote_peers: remote_peers.clone(),
            root_path: project_root.clone(),
        };

        let message_rx = PausableStream::new(welcome.rx);

        let session_infos = SessionInfos {
            host_id: welcome.host_id,
            local_peer,
            remote_peers,
            rx_remote: message_rx.remote(),
            project_access: Default::default(),
            project_root_path: project_root,
            session_id: welcome.session_id,
        };

        let session = Session {
            event_stream,
            message_rx,
            message_tx: welcome.tx,
            project,
            project_access: session_infos.project_access.clone(),
            stop_rx: self.stop_channels.insert(welcome.session_id),
            remove_on_drop: self.sessions.insert(session_infos.clone()),
        };

        ctx.with_namespace([
            ctx.namespace().plugin_name(),
            Collab::<Ed>::NAME,
        ])
        .spawn_local(async move |ctx| session.run(ctx).await)
        .detach();

        Ok(session_infos)
    }
}

impl<Ed: CollabEditor> AsyncAction<Ed> for Start<Ed> {
    const NAME: &str = "start";

    type Args = ();

    async fn call(&mut self, _: Self::Args, ctx: &mut Context<Ed>) {
        let mut progress_reporter =
            <Ed::ProgressReporter as ProgressReporter<Ed, Self>>::new(ctx);

        match self.call_inner(&mut progress_reporter, ctx).await {
            Ok(session_infos) => {
                ProgressReporter::<Ed, Self>::report_success(
                    progress_reporter,
                    (),
                    ctx,
                );
                Ed::on_session_started(&session_infos, ctx).await;
            },
            Err(StartError::UserDidNotConfirm) => {
                ProgressReporter::<Ed, Self>::report_cancellation(
                    progress_reporter,
                    ctx,
                );
            },
            Err(start_error) => {
                ProgressReporter::<Ed, Self>::report_error(
                    progress_reporter,
                    start_error,
                    ctx,
                );
            },
        }
    }
}

impl<Ed: CollabEditor> From<&Collab<Ed>> for Start<Ed> {
    fn from(collab: &Collab<Ed>) -> Self {
        Self {
            auth_state: collab.auth_state.clone(),
            config: collab.config.clone(),
            sessions: collab.sessions.clone(),
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

/// TODO: docs.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
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
        Node::Directory(dir) => {
            stream_builder.with_mut(|builder| builder.push_directory(dir));
            project_builder
                .with_mut(|builder| builder.push_directory(path_in_project))
                .map(|dir_id| {
                    node_id_maps.with_mut(|maps| {
                        maps.node2dir.insert(dir.id(), dir_id);
                    })
                })
        },

        Node::File(file) => {
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
        Node::Symlink(symlink) => {
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
#[derive(cauchy::Debug, derive_more::Display, cauchy::PartialEq)]
#[display("{_0}")]
pub enum StartError<Ed: CollabEditor> {
    /// TODO: docs.
    ConnectToServer(Ed::ConnectToServerError),

    /// TODO: docs.
    Knock(collab_client::KnockError<Ed::ServerParams>),

    /// TODO: docs.
    #[display(
        "No buffer is focused, please move the cursor to a text buffer to \
         determine the project root"
    )]
    NoBufferFocused,

    /// TODO: docs.
    #[display(
        "Cannot start a new collaborative editing session at the root of the \
         filesystem"
    )]
    ProjectRootIsFsRoot,

    /// TODO: docs.
    ReadProject(ReadProjectError<Ed>),

    /// TODO: docs.
    SearchProjectRoot(SearchProjectRootError<Ed>),

    /// TODO: docs.
    #[display("The user didn't confirm starting a new session")]
    UserDidNotConfirm,

    /// TODO: docs.
    #[display("The user is not logged in")]
    UserNotLoggedIn,
}

/// The type of error that can occur when reading a [`fs::Node`] fails.
#[derive(cauchy::Debug, derive_more::Display, cauchy::PartialEq)]
#[display("{_0}")]
pub enum ReadNodeError<Fs: fs::Fs> {
    /// TODO: docs.
    #[display("read two nodes at the same path: {_0}")]
    DuplicateNodeAtPath(AbsPathBuf),

    /// TODO: docs.
    GetNode(Fs::NodeAtPathError),

    /// TODO: docs.
    NodeName(fs::MetadataNameError),

    /// TODO: docs.
    ReadFile(<Fs::File as fs::File>::ReadError),

    /// TODO: docs.
    ReadSymlink(<Fs::Symlink as Symlink>::ReadError),
}

/// The type of error that can occur when reading a [`Project`] fails.
#[derive(cauchy::Debug, derive_more::Display, cauchy::PartialEq)]
#[display("{_0}")]
pub enum ReadProjectError<Ed: CollabEditor> {
    /// TODO: docs.
    GetRoot(<Ed::Fs as Fs>::NodeAtPathError),

    /// TODO: docs.
    #[display("no file or directory at the project root: {_0}")]
    NoNodeAtRootPath(AbsPathBuf),

    /// TODO: docs.
    FileParent(<<Ed::Fs as Fs>::File as fs::File>::ParentError),

    /// The project filter couldn't be created.
    ProjectFilter(Ed::ProjectFilterError),

    /// TODO: docs.
    ReadNode(ReadNodeError<Ed::Fs>),

    /// TODO: docs.
    #[display("path to project root points to a symlink: {_0}")]
    RootIsSymlink(AbsPathBuf),

    /// TODO: docs.
    #[allow(clippy::type_complexity)]
    WalkRoot(
        fs::walk::WalkError<
            Ed::Fs,
            fs::walk::Filtered<ProjectFilter<Ed>, Ed::Fs>,
            ReadNodeError<Ed::Fs>,
        >,
    ),
}

/// TODO: docs.
#[derive(cauchy::Debug, derive_more::Display, cauchy::PartialEq)]
#[display("{_0}")]
pub enum SearchProjectRootError<Ed: CollabEditor> {
    /// TODO: docs.
    #[display("Couldn't determine project root for buffer at {_0}")]
    CouldntFindRoot(AbsPathBuf),

    /// TODO: docs.
    FindRoot(root_markers::FindRootError<Ed::Fs, Markers>),

    /// TODO: docs.
    HomeDir(Ed::HomeDirError),

    /// TODO: docs.
    #[display("There's no buffer with ID {_0:?}")]
    InvalidBufId(Ed::BufferId),

    /// TODO: docs.
    Lsp(Ed::LspRootError),
}

/// A [`fs::filter::Filter`] that filters out every node but one.
pub struct AllButOne<Fs: fs::Fs> {
    id: Fs::NodeId,
}

#[derive(cauchy::Default)]
struct NodeIdMaps<Fs: fs::Fs> {
    file2node: FxHashMap<LocalFileId, Fs::NodeId>,
    node2dir: FxHashMap<Fs::NodeId, LocalDirectoryId>,
    node2file: FxHashMap<Fs::NodeId, LocalFileId>,
}

impl<Fs: fs::Fs> fs::filter::Filter<Fs> for AllButOne<Fs> {
    type Error = Infallible;

    async fn should_filter(
        &self,
        _: &AbsPath,
        node_meta: &impl Metadata<Fs = Fs>,
    ) -> Result<bool, Self::Error> {
        Ok(node_meta.id() != self.id)
    }
}
