//! TODO: docs.

#[cfg(feature = "neovim")]
mod neovim;
#[cfg(feature = "test")]
pub mod test;

use core::fmt::Debug;

use collab_server::SessionId;
use collab_server::message::{Message, Peer, Peers};
use eerie::{PeerId, Replica};
use futures_util::{Sink, Stream};
use nvimx2::backend::{Backend, Buffer, BufferId};
use nvimx2::fs::{self, AbsPath, AbsPathBuf, FsNodeNameBuf};
use nvimx2::{AsyncCtx, notify};

use crate::config;

/// A [`Backend`] subtrait defining additional capabilities needed by the
/// actions in this crate.
pub trait CollabBackend: Backend {
    /// TODO: docs.
    type ServerRx: Stream<Item = Result<Message, Self::ServerRxError>> + Unpin;

    /// TODO: docs.
    type ServerTx: Sink<Message, Error = Self::ServerTxError> + Unpin;

    /// The type of error returned by
    /// [`copy_session_id`](CollabBackend::copy_session_id).
    type CopySessionIdError: Debug + notify::Error;

    /// The type of error returned by
    /// [`default_dir_for_remote_projects`](CollabBackend::default_dir_for_remote_projects).
    type DefaultDirForRemoteProjectsError: Debug + notify::Error;

    /// The type of error returned by [`home_dir`](CollabBackend::home_dir).
    type HomeDirError;

    /// The type of error returned by
    /// [`join_session`](CollabBackend::join_session).
    type JoinSessionError: Debug + notify::Error;

    /// The type of error returned by [`lsp_root`](CollabBackend::lsp_root).
    type LspRootError: Debug;

    /// The type of error returned by
    /// [`read_replica`](CollabBackend::read_replica).
    type ReadReplicaError: Debug + notify::Error;

    /// The type of error returned by
    /// [`search_project_root`](CollabBackend::search_project_root).
    type SearchProjectRootError: Debug + notify::Error;

    /// TODO: docs.
    type ServerTxError: Debug + notify::Error;

    /// TODO: docs.
    type ServerRxError: Debug + notify::Error;

    /// The type of error returned by
    /// [`start_session`](CollabBackend::start_session).
    type StartSessionError: Debug + notify::Error;

    /// Asks the user to confirm starting a new collaborative editing session
    /// rooted at the given path.
    fn confirm_start(
        project_root: &AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = bool>;

    /// Copies the given [`SessionId`] to the user's clipboard.
    fn copy_session_id(
        session_id: SessionId,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<(), Self::CopySessionIdError>>;

    /// TODO: docs.
    fn default_dir_for_remote_projects(
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<
        Output = Result<AbsPathBuf, Self::DefaultDirForRemoteProjectsError>,
    >;

    /// Returns the absolute path to the user's home directory.
    fn home_dir(
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::HomeDirError>>;

    /// TODO: docs.
    fn join_session(
        args: JoinArgs<'_>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<SessionInfos<Self>, Self::JoinSessionError>>;

    /// Returns the path to the root of the workspace containing the buffer
    /// with the given ID, or `None` if there's no language server attached to
    /// it.
    fn lsp_root(
        id: <Self::Buffer<'_> as Buffer>::Id,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError>;

    /// TODO: docs.
    fn read_replica(
        peer_id: PeerId,
        project_root: &AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<Replica, Self::ReadReplicaError>>;

    /// Searches for the root of the project containing the buffer with the
    /// given ID.
    fn search_project_root(
        buffer_id: BufferId<Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::SearchProjectRootError>>;

    /// Prompts the user to select one of the given `(project_root,
    /// session_id)` pairs.
    fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, SessionId)],
        action: ActionForSelectedSession,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Option<&'pairs (AbsPathBuf, SessionId)>>;

    /// TODO: docs.
    fn start_session(
        args: StartArgs<'_>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<SessionInfos<Self>, Self::StartSessionError>>;
}

/// TODO: docs
pub enum ActionForSelectedSession {
    /// TODO: docs
    CopySessionId,

    /// TODO: docs
    Leave,
}

/// TODO: docs.
#[allow(dead_code)]
pub struct StartArgs<'a> {
    /// TODO: docs.
    pub(crate) auth_infos: &'a auth::AuthInfos,

    /// TODO: docs.
    pub(crate) project_name: &'a fs::FsNodeName,

    /// TODO: docs.
    pub(crate) server_address: &'a config::ServerAddress,
}

/// TODO: docs.
pub struct JoinArgs<'a> {
    /// TODO: docs.
    pub(crate) auth_infos: &'a auth::AuthInfos,

    /// TODO: docs.
    pub(crate) session_id: SessionId,

    /// TODO: docs.
    pub(crate) server_address: &'a config::ServerAddress,
}

/// TODO: docs.
pub struct SessionInfos<B: CollabBackend> {
    /// TODO: docs.
    pub(crate) host_id: PeerId,

    /// TODO: docs.
    pub(crate) local_peer: Peer,

    /// TODO: docs.
    pub(crate) project_name: FsNodeNameBuf,

    /// TODO: docs.
    pub(crate) remote_peers: Peers,

    /// TODO: docs.
    pub(crate) server_tx: B::ServerTx,

    /// TODO: docs.
    pub(crate) server_rx: B::ServerRx,

    /// TODO: docs.
    pub(crate) session_id: SessionId,
}

#[cfg(any(feature = "neovim", feature = "test"))]
mod default_read_replica {
    use core::convert::Infallible;
    use core::fmt;
    use std::sync::Arc;

    use concurrent_queue::{ConcurrentQueue, PushError};
    use eerie::ReplicaBuilder;
    use fs::{FsNodeKind, Metadata};
    use nvimx2::ByteOffset;
    use walkdir::{Either, WalkDir, WalkError, WalkErrorKind};

    use super::*;

    pub(super) async fn read_replica<B>(
        peer_id: PeerId,
        project_root: fs::AbsPathBuf,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<Replica, Error<B>>
    where
        B: CollabBackend,
    {
        let fs = ctx.fs();
        let res = async move {
            let op_queue = Arc::new(ConcurrentQueue::unbounded());
            let op_queue2 = Arc::clone(&op_queue);
            let handler = async move |entry: walkdir::DirEntry<'_, _>| {
                let op = match entry.node_kind() {
                    FsNodeKind::File => {
                        PushNode::File(entry.path(), entry.len())
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
                    return Err(Error::Walk(WalkError {
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
            if let Some(mut file) = <&fs::AbsPath>::try_from(&*buffer.name())
                .ok()
                .and_then(|buffer_path| builder.file_mut(buffer_path))
            {
                file.set_len(buffer.byte_len().into());
            }
        });

        Ok(builder.build())
    }

    #[derive(derive_more::Debug)]
    #[debug(bound(B: CollabBackend))]
    pub enum Error<B: CollabBackend> {
        Walk(WalkError<WalkErrorKind<B::Fs>>),
    }

    enum PushNode {
        File(AbsPathBuf, ByteOffset),
        Directory(AbsPathBuf),
    }

    impl<B: CollabBackend> PartialEq for Error<B>
        where
            WalkErrorKind<B::Fs>: PartialEq,
            <<<B::Fs as fs::Fs>::Directory as fs::Directory>::Metadata as fs::Metadata>::Error: PartialEq
    {
        fn eq(&self, other: &Self) -> bool {
            use Error::*;

            match (self, other) {
                (Walk(l), Walk(r)) => l == r,
            }
        }
    }

    impl<B: CollabBackend> fmt::Display for Error<B> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Error::Walk(err) => fmt::Display::fmt(err, f),
            }
        }
    }

    impl<B: CollabBackend> notify::Error for Error<B> {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            (notify::Level::Error, notify::Message::from_display(self))
        }
    }
}

#[cfg(any(feature = "neovim", feature = "test"))]
mod default_search_project_root {
    use core::fmt;

    use super::*;

    const MARKERS: Markers = root_markers::GitDirectory;

    pub(super) type Markers = root_markers::GitDirectory;

    pub(super) async fn search<B>(
        buffer_id: BufferId<B>,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<AbsPathBuf, Error<B>>
    where
        B: CollabBackend,
    {
        if let Some(lsp_res) = B::lsp_root(buffer_id.clone(), ctx).transpose()
        {
            return lsp_res.map_err(Error::Lsp);
        }

        let buffer_name = ctx.with_ctx(|ctx| {
            ctx.buffer(buffer_id.clone())
                .ok_or(Error::InvalidBufId(buffer_id))
                .map(|buf| buf.name().into_owned())
        })?;

        let buffer_path = buffer_name
            .parse::<AbsPathBuf>()
            .map_err(|_| Error::BufNameNotAbsolutePath(buffer_name))?;

        let home_dir = B::home_dir(ctx).await.map_err(Error::HomeDir)?;

        let args = root_markers::FindRootArgs {
            marker: MARKERS,
            start_from: &buffer_path,
            stop_at: Some(&home_dir),
        };

        let mut fs = ctx.fs();

        if let Some(res) = args.find(&mut fs).await.transpose() {
            return res.map_err(Error::FindRoot);
        }

        buffer_path
            .parent()
            .map(ToOwned::to_owned)
            .ok_or(Error::CouldntFindRoot(buffer_path))
    }

    #[derive(derive_more::Debug)]
    #[debug(bound(B: CollabBackend))]
    pub enum Error<B: CollabBackend> {
        BufNameNotAbsolutePath(String),
        CouldntFindRoot(fs::AbsPathBuf),
        FindRoot(root_markers::FindRootError<B::Fs, Markers>),
        HomeDir(B::HomeDirError),
        InvalidBufId(BufferId<B>),
        Lsp(B::LspRootError),
    }

    impl<B: CollabBackend> PartialEq for Error<B>
    where
        B::BufferId: PartialEq,
        B::HomeDirError: PartialEq,
        B::LspRootError: PartialEq,
        root_markers::FindRootError<B::Fs, Markers>: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            use Error::*;

            match (self, other) {
                (BufNameNotAbsolutePath(l), BufNameNotAbsolutePath(r)) => {
                    l == r
                },
                (CouldntFindRoot(l), CouldntFindRoot(r)) => l == r,
                (FindRoot(l), FindRoot(r)) => l == r,
                (HomeDir(l), HomeDir(r)) => l == r,
                (InvalidBufId(l), InvalidBufId(r)) => l == r,
                (Lsp(l), Lsp(r)) => l == r,
                _ => false,
            }
        }
    }

    impl<B: CollabBackend> fmt::Display for Error<B>
    where
        B::HomeDirError: fmt::Display,
        B::LspRootError: fmt::Display,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Error::BufNameNotAbsolutePath(str) => {
                    write!(f, "buffer name {:?} is not an absolute path", str)
                },
                Error::CouldntFindRoot(abs_path_buf) => {
                    write!(
                        f,
                        "couldn't find project root for buffer at {:?}",
                        abs_path_buf
                    )
                },
                Error::FindRoot(err) => fmt::Display::fmt(err, f),
                Error::HomeDir(err) => fmt::Display::fmt(err, f),
                Error::InvalidBufId(buf_id) => {
                    write!(f, "there's no buffer whose ID is {:?}", buf_id)
                },
                Error::Lsp(err) => fmt::Display::fmt(err, f),
            }
        }
    }

    impl<B: CollabBackend> notify::Error for Error<B>
    where
        B::HomeDirError: fmt::Display,
        B::LspRootError: fmt::Display,
    {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            (notify::Level::Error, notify::Message::from_display(self))
        }
    }
}

#[cfg(any(feature = "neovim", feature = "test"))]
mod root_markers {
    use core::error::Error;
    use core::fmt;
    use std::borrow::Cow;

    use futures_util::stream::{self, StreamExt};
    use futures_util::{pin_mut, select};
    use nvimx2::fs::{self, Directory, File, Symlink};
    use nvimx2::notify;
    use smol_str::ToSmolStr;

    pub struct FindRootArgs<'a, M> {
        /// The marker used to determine if a directory is the root.
        pub(super) marker: M,

        /// The path to the first directory to search for markers in.
        ///
        /// If this points to a file, the search will start from its parent.
        pub(super) start_from: &'a fs::AbsPath,

        /// The path to the last directory to search for markers in, if any.
        ///
        /// If set and no root marker is found within it, the search is cut
        /// short instead of continuing with its parent.
        pub(super) stop_at: Option<&'a fs::AbsPath>,
    }

    pub struct GitDirectory;

    pub trait RootMarker<Fs: fs::Fs> {
        type Error: Error;

        fn matches(
            &self,
            dir_entry: &<Fs::Directory as fs::Directory>::Metadata,
        ) -> impl Future<Output = Result<bool, Self::Error>>;
    }

    #[derive(derive_more::Debug)]
    #[debug(bounds(Fs: fs::Fs, M: RootMarker<Fs>))]
    pub struct FindRootError<Fs: fs::Fs, M: RootMarker<Fs>> {
        /// The path to the file or directory at which the error occurred.
        pub path: fs::AbsPathBuf,

        /// The kind of error that occurred.
        pub kind: FindRootErrorKind<Fs, M>,
    }

    #[derive(derive_more::Debug)]
    #[debug(bounds(Fs: fs::Fs, M: RootMarker<Fs>))]
    pub enum FindRootErrorKind<Fs: fs::Fs, M: RootMarker<Fs>> {
        DirEntry(DirEntryError<Fs>),
        FollowSymlink(<Fs::Symlink as fs::Symlink>::FollowError),
        Marker { dir_entry_name: Option<fs::FsNodeNameBuf>, err: M::Error },
        NodeAtStartPath(Fs::NodeAtPathError),
        ReadDir(<Fs::Directory as fs::Directory>::ReadError),
        StartPathNotFound,
        StartsAtDanglingSymlink,
    }

    #[derive(derive_more::Debug)]
    #[debug(bound(Fs: fs::Fs))]
    pub enum DirEntryError<Fs: fs::Fs> {
        Access(<Fs::Directory as fs::Directory>::ReadEntryError),
        Name(
            <<Fs::Directory as fs::Directory>::Metadata as fs::Metadata>::NameError,
        ),
        NodeKind(
            <<Fs::Directory as fs::Directory>::Metadata as fs::Metadata>::NodeKindError,
        ),
    }

    impl<M> FindRootArgs<'_, M> {
        pub(super) async fn find<Fs>(
            self,
            fs: &mut Fs,
        ) -> Result<Option<fs::AbsPathBuf>, FindRootError<Fs, M>>
        where
            Fs: fs::Fs,
            M: RootMarker<Fs>,
        {
            let mut node = fs
                .node_at_path(self.start_from)
                .await
                .map_err(FindRootErrorKind::NodeAtStartPath)
                .and_then(|maybe_node| {
                    maybe_node.ok_or(FindRootErrorKind::StartPathNotFound)
                })
                .map_err(|kind| FindRootError {
                    path: self.start_from.to_owned(),
                    kind,
                })?;

            let mut dir = loop {
                match node {
                    fs::FsNode::Directory(dir) => break dir,
                    fs::FsNode::File(file) => break file.parent().await,
                    fs::FsNode::Symlink(symlink) => {
                        node = symlink
                            .follow_recursively()
                            .await
                            .map_err(FindRootErrorKind::FollowSymlink)
                            .and_then(|maybe_target| {
                                maybe_target.ok_or(
                                    FindRootErrorKind::StartsAtDanglingSymlink,
                                )
                            })
                            .map_err(|kind| FindRootError {
                                path: self.start_from.to_owned(),
                                kind,
                            })?;
                    },
                }
            };

            loop {
                if self.contains_marker(&dir).await? {
                    return Ok(Some(dir.path().to_owned()));
                }
                if self.stop_at == Some(dir.path()) {
                    return Ok(None);
                }
                let Some(parent) = dir.parent().await else { return Ok(None) };
                dir = parent;
            }
        }

        async fn contains_marker<Fs: fs::Fs>(
            &self,
            dir: &Fs::Directory,
        ) -> Result<bool, FindRootError<Fs, M>>
        where
            M: RootMarker<Fs>,
        {
            use fs::{Directory, Metadata};
            let read_dir = dir
                .read()
                .await
                .map_err(|err| FindRootError {
                    path: dir.path().to_owned(),
                    kind: FindRootErrorKind::ReadDir(err),
                })?
                .fuse();

            pin_mut!(read_dir);

            let mut check_marker_matches = stream::FuturesUnordered::new();

            loop {
                select! {
                    read_res = read_dir.select_next_some() => {
                        let dir_entry =
                            read_res.map_err(|err| FindRootError {
                                path: dir.path().to_owned(),
                                kind: FindRootErrorKind::DirEntry(
                                    DirEntryError::Access(err),
                                ),
                            })?;

                        let fut = async move {
                            match self.marker.matches(&dir_entry).await {
                                Ok(matches) => Ok(matches),
                                Err(err) => {
                                    let dir_entry_name = dir_entry.name()
                                        .await
                                        .ok();
                                    Err(FindRootError {
                                        path: dir.path().to_owned(),
                                        kind: FindRootErrorKind::Marker {
                                            dir_entry_name,
                                            err
                                        },
                                    })
                                }
                            }
                        };

                        check_marker_matches.push(fut);
                    },

                    marker_res = check_marker_matches.select_next_some() => {
                        match marker_res {
                            Ok(false) => continue,
                            true_or_err => return true_or_err,
                        }
                    },

                    complete => return Ok(false),
                }
            }
        }
    }

    impl<Fs: fs::Fs> RootMarker<Fs> for GitDirectory {
        type Error = DirEntryError<Fs>;

        async fn matches(
            &self,
            dir_entry: &<Fs::Directory as fs::Directory>::Metadata,
        ) -> Result<bool, Self::Error> {
            use fs::Metadata;
            Ok(dir_entry.name().await.map_err(DirEntryError::Name)?.as_ref()
                == ".git"
                && dir_entry
                    .node_kind()
                    .await
                    .map(fs::FsNodeKind::is_dir)
                    .map_err(DirEntryError::NodeKind)?)
        }
    }

    impl<Fs: fs::Fs, M: RootMarker<Fs>> PartialEq for FindRootError<Fs, M>
    where
        FindRootErrorKind<Fs, M>: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            self.path == other.path && self.kind == other.kind
        }
    }

    impl<Fs, M> notify::Error for FindRootError<Fs, M>
    where
        Fs: fs::Fs,
        M: RootMarker<Fs>,
        M::Error: fmt::Display,
    {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            let mut message = notify::Message::new();

            let mut path = Cow::Borrowed(&*self.path);

            let err: &dyn fmt::Display = match &self.kind {
                FindRootErrorKind::DirEntry(err) => {
                    message.push_str("couldn't read file or directory under ");
                    err
                },
                FindRootErrorKind::FollowSymlink(err) => {
                    message.push_str("couldn't follow symlink at ");
                    err
                },
                FindRootErrorKind::Marker { dir_entry_name, err } => {
                    message.push_str(
                        "couldn't match markers with file or directory ",
                    );
                    if let Some(entry_name) = dir_entry_name {
                        message.push_str("at ");
                        let mut new_path = self.path.to_owned();
                        new_path.push(entry_name);
                        path = Cow::Owned(new_path);
                    } else {
                        message.push_str("under ");
                    }
                    err
                },
                FindRootErrorKind::NodeAtStartPath(err) => {
                    message.push_str("couldn't read file or directory at ");
                    err
                },
                FindRootErrorKind::ReadDir(err) => {
                    message.push_str("couldn't read directory at ");
                    err
                },
                FindRootErrorKind::StartsAtDanglingSymlink => {
                    message
                        .push_str("no file or directory found at ")
                        .push_info(path.to_smolstr());
                    return (notify::Level::Error, message);
                },
                FindRootErrorKind::StartPathNotFound => {
                    message
                        .push_str("no file or directory found at ")
                        .push_info(path.to_smolstr());
                    return (notify::Level::Error, message);
                },
            };

            message
                .push_info(path.to_smolstr())
                .push_str(": ")
                .push_str(err.to_smolstr());

            (notify::Level::Error, message)
        }
    }

    impl<Fs: fs::Fs, M: RootMarker<Fs>> PartialEq for FindRootErrorKind<Fs, M>
    where
        DirEntryError<Fs>: PartialEq,
        <Fs::Symlink as fs::Symlink>::FollowError: PartialEq,
        M::Error: PartialEq,
        Fs::NodeAtPathError: PartialEq,
        <Fs::Directory as fs::Directory>::ReadError: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            use FindRootErrorKind::*;

            match (self, other) {
                (DirEntry(l), DirEntry(r)) => l == r,
                (FollowSymlink(l), FollowSymlink(r)) => l == r,
                (
                    Marker { dir_entry_name: l, err: l_err },
                    Marker { dir_entry_name: r, err: r_err },
                ) => l == r && l_err == r_err,
                (NodeAtStartPath(l), NodeAtStartPath(r)) => l == r,
                (ReadDir(l), ReadDir(r)) => l == r,
                (StartPathNotFound, StartPathNotFound) => true,
                (StartsAtDanglingSymlink, StartsAtDanglingSymlink) => true,
                _ => false,
            }
        }
    }

    impl<Fs: fs::Fs> PartialEq for DirEntryError<Fs>
    where
        <Fs::Directory as fs::Directory>::ReadEntryError: PartialEq,
        <<Fs::Directory as fs::Directory>::Metadata as fs::Metadata>::NameError: PartialEq,
        <<Fs::Directory as fs::Directory>::Metadata as fs::Metadata>::NodeKindError: PartialEq,
    {
        fn eq(&self, other: &Self) -> bool {
            use DirEntryError::*;

            match (self, other) {
                (Access(l), Access(r)) => l == r,
                (Name(l), Name(r)) => l == r,
                (NodeKind(l), NodeKind(r)) => l == r,
                _ => false,
            }
        }
    }

    impl<Fs: fs::Fs, M: RootMarker<Fs>> fmt::Display for FindRootError<Fs, M> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.kind {
                FindRootErrorKind::DirEntry(err) => {
                    write!(
                        f,
                        "couldn't read file or directory at {:?}: {}",
                        self.path, err
                    )
                },
                FindRootErrorKind::FollowSymlink(err) => {
                    write!(
                        f,
                        "couldn't follow symlink at {:?}: {}",
                        self.path, err
                    )
                },
                FindRootErrorKind::Marker { dir_entry_name, err } => {
                    let path = match dir_entry_name {
                        Some(name) => {
                            let mut path = self.path.clone();
                            path.push(name);
                            path
                        },
                        None => self.path.clone(),
                    };
                    write!(
                        f,
                        "couldn't match marker with file or directory at \
                         {:?}: {}",
                        path, err
                    )
                },
                FindRootErrorKind::NodeAtStartPath(err) => {
                    write!(
                        f,
                        "couldn't read file or directory at {:?}: {}",
                        self.path, err
                    )
                },
                FindRootErrorKind::ReadDir(err) => {
                    write!(
                        f,
                        "couldn't read directory at {:?}: {}",
                        self.path, err
                    )
                },
                FindRootErrorKind::StartPathNotFound => {
                    write!(f, "no file or directory found at {:?}", self.path)
                },
                FindRootErrorKind::StartsAtDanglingSymlink => {
                    write!(
                        f,
                        "starting point at {:?} is a dangling symlink",
                        self.path
                    )
                },
            }
        }
    }

    impl<Fs: fs::Fs> fmt::Display for DirEntryError<Fs> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DirEntryError::Access(err) => err.fmt(f),
                DirEntryError::Name(err) => err.fmt(f),
                DirEntryError::NodeKind(err) => err.fmt(f),
            }
        }
    }

    impl<Fs: fs::Fs> Error for DirEntryError<Fs> {}
}
