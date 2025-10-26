//! TODO: docs.

use core::ops::Deref;
use core::ptr::NonNull;
use std::borrow::Cow;
use std::io;

use abs_path::AbsPathBuf;
use auth::AuthState;
use collab_project::Project;
use collab_project::fs::{
    Directory as ProjectDirectory,
    File as ProjectFile,
    Node,
};
use collab_server::client::{self, MessageFragment};
use collab_types::{Message, MessageId, PeerId, ProjectRequest, puff};
use editor::command::{self, ToCompletionFn};
use editor::module::{AsyncAction, Module};
use editor::shared::{MultiThreaded, Shared};
use editor::{Access, Context};
use either::Either;
use fs::{Directory, File, Fs, Symlink};
use futures_util::{AsyncReadExt, SinkExt, StreamExt, future, stream};
use fxhash::FxHashMap;
use puff::directory::LocalDirectoryId;
use puff::file::LocalFileId;

use crate::collab::Collab;
use crate::config::Config;
use crate::editors::{CollabEditor, SessionId, Welcome};
use crate::event_stream::EventStreamBuilder;
use crate::pausable_stream::PausableStream;
use crate::peers::RemotePeers;
use crate::progress::{JoinState, ProgressReporter};
use crate::project::{self, IdMaps};
use crate::session::{Session, SessionInfos, Sessions};

/// The `Action` used to join an existing collaborative editing session.
#[derive(cauchy::Clone)]
pub struct Join<Ed: CollabEditor> {
    auth_state: AuthState,
    config: Shared<Config>,
    sessions: Sessions<Ed>,
}

impl<Ed: CollabEditor> Join<Ed> {
    #[allow(clippy::too_many_lines)]
    pub(crate) async fn call_inner(
        &self,
        session_id: SessionId<Ed>,
        progress_reporter: &mut impl ProgressReporter<Ed, Self>,
        ctx: &mut Context<Ed>,
    ) -> Result<SessionInfos<Ed>, JoinError<Ed>> {
        let jwt = self
            .auth_state
            .with(Clone::clone)
            .ok_or(JoinError::UserNotLoggedIn)?;

        let server_addr = self.config.with(|c| c.server_address.clone());

        progress_reporter.report_progress(
            JoinState::ConnectingToServer(server_addr.borrow()),
            ctx,
        );

        let (reader, writer) = Ed::connect_to_server(server_addr, ctx)
            .await
            .map_err(JoinError::ConnectToServer)?
            .split();

        let knock = client::Knock::<Ed::ServerParams> {
            auth_infos: jwt.into(),
            session_intent: client::SessionIntent::JoinExisting(session_id),
        };

        progress_reporter.report_progress(JoinState::JoiningSession, ctx);

        let mut welcome = client::knock(reader, writer, knock)
            .await
            .map_err(JoinError::Knock)?;

        let local_peer = welcome.peer.clone();

        let project_root = match self
            .config
            .with(|c| c.store_remote_projects_under.clone())
        {
            Some(remote_dir) => remote_dir,
            None => Ed::default_dir_for_remote_projects(ctx)
                .await
                .map_err(JoinError::DefaultDirForRemoteProjects)?,
        }
        .join(&welcome.project_name);

        progress_reporter.report_progress(
            JoinState::ReceivedWelcome(Cow::Borrowed(&welcome.project_name)),
            ctx,
        );

        let (project, buffered) = request_project::<Ed>(
            local_peer.id,
            &mut welcome,
            progress_reporter,
            ctx,
        )
        .await
        .map_err(JoinError::RequestProject)?;

        progress_reporter.report_progress(
            JoinState::WritingProject(Cow::Borrowed(&project_root)),
            ctx,
        );

        let (project_root, stream_builder, id_maps) =
            write_project(&project, project_root, ctx)
                .await
                .map_err(JoinError::WriteProject)?;

        let project_filter = Ed::project_filter(&project_root, ctx)
            .map_err(JoinError::ProjectFilter)?;

        let event_stream = stream_builder
            .push_filter(Either::Left(project_filter))
            .build(ctx);

        let remote_peers = RemotePeers::new(welcome.other_peers, &project);

        let project = project::Project {
            agent_id: event_stream.agent_id(),
            id_maps: id_maps.into(),
            inner: project,
            local_peer: local_peer.clone(),
            peer_cursors: FxHashMap::default(),
            peer_selections: FxHashMap::default(),
            remote_peers: remote_peers.clone(),
            root_path: project_root.path().to_owned(),
        };

        let message_rx = PausableStream::new(
            stream::iter(buffered).map(Ok).chain(welcome.rx),
        );

        let (stop_tx, stop_rx) = flume::bounded(1);

        let session_infos = SessionInfos {
            host_id: welcome.host_id,
            local_peer,
            remote_peers,
            pause_remote: message_rx.remote(),
            project_access: Default::default(),
            project_root_path: project_root.path().to_owned(),
            session_id: welcome.session_id,
            stop_tx,
        };

        let session = Session {
            event_stream,
            message_rx,
            message_tx: welcome.tx,
            project,
            project_access: session_infos.project_access.clone(),
            stop_rx,
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

impl<Ed: CollabEditor> AsyncAction<Ed> for Join<Ed> {
    const NAME: &str = "join";

    type Args = command::Parse<SessionId<Ed>>;

    async fn call(
        &mut self,
        command::Parse(session_id): Self::Args,
        ctx: &mut Context<Ed>,
    ) {
        let mut progress_reporter =
            <Ed::ProgressReporter as ProgressReporter<Ed, Self>>::new(ctx);

        match self.call_inner(session_id, &mut progress_reporter, ctx).await {
            Ok(session_infos) => {
                ProgressReporter::<Ed, Self>::report_success(
                    progress_reporter,
                    (),
                    ctx,
                );
                Ed::on_session_joined(&session_infos, ctx).await;
            },
            Err(join_error) => {
                ProgressReporter::<Ed, Self>::report_error(
                    progress_reporter,
                    join_error,
                    ctx,
                );
            },
        }
    }
}

impl<Ed: CollabEditor> From<&Collab<Ed>> for Join<Ed> {
    fn from(collab: &Collab<Ed>) -> Self {
        Self {
            auth_state: collab.auth_state.clone(),
            config: collab.config.clone(),
            sessions: collab.sessions.clone(),
        }
    }
}

impl<Ed: CollabEditor> ToCompletionFn<Ed> for Join<Ed> {
    fn to_completion_fn(&self) {}
}

/// TODO: docs.
async fn request_project<Ed: CollabEditor>(
    local_id: PeerId,
    welcome: &mut Welcome<Ed>,
    progress_reporter: &mut impl ProgressReporter<Ed, Join<Ed>>,
    ctx: &mut Context<Ed>,
) -> Result<(Project, Vec<MessageFragment>), RequestProjectError> {
    let request_id = MessageId { sender_id: local_id, message_seq: 0 };

    let request = ProjectRequest {
        request_from: welcome
            .other_peers
            .as_slice()
            .first()
            .expect("can't be empty")
            .id,
        request_id,
    };

    welcome.tx.send(Message::ProjectRequest(request)).await?;

    let mut buffered = Vec::new();

    let mut bytes_received = 0;

    loop {
        let fragment = welcome
            .rx
            .next()
            .await
            .ok_or(RequestProjectError::SessionEnded)??;

        if fragment.header.response_id().is_none_or(|id| id != request_id) {
            buffered.push(fragment);
            continue;
        }

        bytes_received += fragment.payload_len as u64;

        progress_reporter.report_progress(
            JoinState::ReceivingProject(
                bytes_received,
                fragment.header.message_len(),
            ),
            ctx,
        );

        if let Some(Message::ProjectResponse(response)) = fragment.message {
            break Ok((
                Project::decode(&response.encoded_project, local_id)?,
                buffered,
            ));
        }
    }
}

/// TODO: docs.
async fn write_project<Ed: CollabEditor>(
    project: &Project,
    root_path: AbsPathBuf,
    ctx: &mut Context<Ed>,
) -> Result<
    (
        <Ed::Fs as Fs>::Directory,
        EventStreamBuilder<Ed::Fs>,
        NodeIdMaps<Ed::Fs>,
    ),
    WriteProjectError<Ed::Fs>,
> {
    let fs = ctx.fs();

    // SAFETY: we're awaiting on the following background task and not
    // detaching it, so the pointer is guaranteed to point to a `Project`
    // for its entire duration.
    let project_ptr = unsafe { ProjectPtr::new(project) };

    ctx.spawn_background(async move {
        if let Some(node) = fs
            .node_at_path(&root_path)
            .await
            .map_err(WriteProjectError::GetNodeAtRoot)?
        {
            node.delete().await.map_err(WriteProjectError::DeleteNodeAtRoot)?
        }

        let project_root = fs
            .create_all_missing_directories(&root_path)
            .await
            .map_err(WriteProjectError::CreateRootDirectory)?;

        let mut stream_builder = EventStreamBuilder::new(&project_root);
        let stream_builder_mut = Shared::new(&mut stream_builder);

        let mut node_id_maps = NodeIdMaps::default();
        let node_id_maps_mut = Shared::new(&mut node_id_maps);

        write_children(
            project_ptr.root(),
            &project_root,
            &stream_builder_mut,
            &node_id_maps_mut,
        )
        .await?;

        Ok((project_root, stream_builder, node_id_maps))
    })
    .await
}

/// TODO: docs.
async fn write_children<Fs: fs::Fs>(
    project_dir: ProjectDirectory<'_>,
    fs_dir: &Fs::Directory,
    stream_builder: &Shared<&mut EventStreamBuilder<Fs>, MultiThreaded>,
    node_id_maps: &Shared<&mut NodeIdMaps<Fs>, MultiThreaded>,
) -> Result<(), WriteProjectError<Fs>> {
    let mut write_children = project_dir
        .children()
        .map(|node| match node {
            Node::Directory(directory) => future::Either::Left(async move {
                let dir_name = directory.try_name().expect("dir is not root");

                let dir = fs_dir
                    .create_directory(dir_name)
                    .await
                    .map_err(WriteProjectError::CreateDirectory)?;

                write_children(directory, &dir, stream_builder, node_id_maps)
                    .await
            }),
            Node::File(file) => future::Either::Right(async move {
                write_file(file, fs_dir, stream_builder, node_id_maps).await
            }),
        })
        .collect::<stream::FuturesUnordered<_>>();

    while let Some(res) = write_children.next().await {
        res?;
    }

    stream_builder.with_mut(|builder| builder.push_directory(fs_dir));

    node_id_maps.with_mut(|maps| {
        maps.node2dir.insert(fs_dir.id(), project_dir.id());
    });

    Ok(())
}

/// TODO: docs.
async fn write_file<Fs: fs::Fs>(
    file: ProjectFile<'_>,
    parent: &Fs::Directory,
    stream_builder: &Shared<&mut EventStreamBuilder<Fs>, MultiThreaded>,
    node_id_maps: &Shared<&mut NodeIdMaps<Fs>, MultiThreaded>,
) -> Result<(), WriteProjectError<Fs>> {
    let file_name = file.name();

    let node_id = match file {
        ProjectFile::Binary(binary_file) => {
            let mut file = parent
                .create_file(file_name)
                .await
                .map_err(WriteProjectError::CreateFile)?;

            file.write(binary_file.contents())
                .await
                .map_err(WriteProjectError::WriteFile)?;

            stream_builder.with_mut(|builder| builder.push_file(&file));

            file.id()
        },

        ProjectFile::Text(text_file) => {
            let mut file = parent
                .create_file(file_name)
                .await
                .map_err(WriteProjectError::CreateFile)?;

            file.write_chunks(text_file.contents().chunks())
                .await
                .map_err(WriteProjectError::WriteFile)?;

            stream_builder.with_mut(|builder| builder.push_file(&file));

            file.id()
        },

        ProjectFile::Symlink(symlink) => parent
            .create_symlink(file_name, symlink.target_path())
            .await
            .map_err(WriteProjectError::CreateSymlink)?
            .id(),
    };

    node_id_maps.with_mut(|maps| {
        maps.node2file.insert(node_id, file.id());
    });

    Ok(())
}

/// The type of error that can occur when [`Join`]ing a session fails.
#[derive(cauchy::Debug, derive_more::Display, cauchy::PartialEq)]
#[display("{_0}")]
pub enum JoinError<Ed: CollabEditor> {
    /// TODO: docs.
    ConnectToServer(Ed::ConnectToServerError),

    /// TODO: docs.
    DefaultDirForRemoteProjects(Ed::DefaultDirForRemoteProjectsError),

    /// TODO: docs.
    Knock(client::KnockError<Ed::ServerParams>),

    /// The project filter couldn't be created.
    ProjectFilter(Ed::ProjectFilterError),

    /// TODO: docs.
    RequestProject(RequestProjectError),

    /// TODO: docs.
    #[display("The user is not logged in")]
    UserNotLoggedIn,

    /// TODO: docs.
    #[display("Couldn't write project: {_0}")]
    WriteProject(WriteProjectError<Ed::Fs>),
}

/// The type of error that can occur when requesting the state of the project
/// from another peer in a session fails.
#[derive(Debug, derive_more::Display, cauchy::PartialEq, cauchy::From)]
#[display("{_0}")]
pub enum RequestProjectError {
    /// TODO: docs.
    DecodeProject(
        #[from]
        #[partial_eq(skip)]
        collab_project::DecodeError,
    ),

    /// TODO: docs.
    RecvResponse(
        #[from]
        #[partial_eq(skip)]
        client::ReceiveError,
    ),

    /// TODO: docs.
    SendRequest(
        #[from]
        #[partial_eq(skip)]
        io::Error,
    ),

    /// TODO: docs.
    #[display("The session ended before we could join it")]
    SessionEnded,
}

/// TODO: docs.
#[derive(cauchy::Debug, derive_more::Display, cauchy::PartialEq)]
#[display("{_0}")]
pub enum WriteProjectError<Fs: fs::Fs> {
    /// TODO: docs.
    CreateDirectory(<Fs::Directory as fs::Directory>::CreateDirectoryError),

    /// TODO: docs.
    CreateFile(<Fs::Directory as fs::Directory>::CreateFileError),

    /// TODO: docs.
    CreateSymlink(<Fs::Directory as fs::Directory>::CreateSymlinkError),

    /// TODO: docs.
    ClearRoot(<Fs::Directory as fs::Directory>::ClearError),

    /// TODO: docs.
    DeleteNodeAtRoot(fs::NodeDeleteError<Fs>),

    /// TODO: docs.
    CreateRootDirectory(Fs::CreateDirectoriesError),

    /// TODO: docs.
    GetNodeAtRoot(Fs::NodeAtPathError),

    /// TODO: docs.
    WriteFile(<Fs::File as fs::File>::WriteError),
}

/// A `Send` newtype around a `NonNull<Project>`.
#[derive(Clone, Copy)]
struct ProjectPtr(NonNull<Project>);

#[derive(cauchy::Default)]
struct NodeIdMaps<Fs: fs::Fs> {
    node2dir: FxHashMap<Fs::NodeId, LocalDirectoryId>,
    node2file: FxHashMap<Fs::NodeId, LocalFileId>,
}

impl ProjectPtr {
    /// SAFETY: same as [`NonNull::as_ref()`].
    unsafe fn new(proj: &Project) -> Self {
        Self(proj.into())
    }
}

impl<Ed: CollabEditor> From<NodeIdMaps<Ed::Fs>> for IdMaps<Ed> {
    fn from(node_id_maps: NodeIdMaps<Ed::Fs>) -> Self {
        Self {
            node2dir: node_id_maps.node2dir,
            node2file: node_id_maps.node2file,
            ..Default::default()
        }
    }
}

impl Deref for ProjectPtr {
    type Target = Project;

    fn deref(&self) -> &Self::Target {
        // SAFETY: up to the caller of `ProjectPtr::new`.
        unsafe { self.0.as_ref() }
    }
}

/// SAFETY: `&Project` is not aliased.
unsafe impl Send for ProjectPtr {}
