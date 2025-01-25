#[cfg(feature = "neovim")]
mod neovim;

use collab_server::message::{Message, PeerId};
use futures_util::{Sink, Stream};
use nvimx2::backend::{Backend, Buffer, BufferId};
use nvimx2::fs::{self, AbsPathBuf};
use nvimx2::{AsyncCtx, notify};

use crate::config;

/// A [`Backend`] subtrait defining additional capabilities needed by the
/// actions in this crate.
pub trait CollabBackend:
    Backend<Buffer: CollabBuffer<Self>, Fs: CollabFs>
{
    /// The type of error returned by
    /// [`search_project_root`](CollabBackend::search_project_root).
    type SearchProjectRootError: notify::Error;

    /// TODO: docs.
    type ServerTx: Sink<Message, Error = Self::ServerTxError>;

    /// TODO: docs.
    type ServerRx: Stream<Item = Result<Message, Self::ServerRxError>>;

    /// TODO: docs.
    type ServerTxError: notify::Error;

    /// TODO: docs.
    type ServerRxError: notify::Error;

    /// The type of error returned by
    /// [`start_session`](CollabBackend::start_session).
    type StartSessionError: notify::Error;

    /// Asks the user to confirm starting a new collaborative editing session
    /// rooted at the given path.
    fn confirm_start(
        project_root: &fs::AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = bool>;

    /// Searches for the root of the project containing the buffer with the
    /// given ID.
    fn search_project_root(
        buffer_id: BufferId<Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::SearchProjectRootError>>;

    /// TODO: docs.
    fn start_session(
        args: StartArgs<'_>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<StartInfos<Self>, Self::StartSessionError>>;
}

/// A [`Buffer`] subtrait defining additional capabilities needed by the
/// actions in this crate.
pub trait CollabBuffer<B: CollabBackend>: Buffer<B> {
    /// The type of error returned by [`lsp_root`](CollabBuffer::lsp_root).
    type LspRootError;

    /// Returns the path to the root of the workspace containing the buffer
    /// with the given ID, or `None` if there's no language server attached to
    /// it.
    fn lsp_root(
        buffer_id: BufferId<B>,
        ctx: &mut AsyncCtx<'_, B>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError>;
}

/// A [`Fs`](fs::Fs) subtrait defining additional capabilities needed by the
/// actions in this crate.
pub trait CollabFs: fs::Fs {
    /// The type of error returned by [`CollabFs`](CollabFs::home_dir).
    type HomeDirError;

    /// Returns the absolute path to the user's home directory.
    fn home_dir(
        &mut self,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::HomeDirError>>;
}

/// TODO: docs.
pub struct StartArgs<'a> {
    /// TODO: docs.
    pub(crate) _server_address: &'a config::ServerAddress,

    /// TODO: docs.
    pub(crate) _auth_infos: &'a auth::AuthInfos,

    /// TODO: docs.
    pub(crate) _project_root: &'a fs::AbsPath,
}

/// TODO: docs.
pub struct StartInfos<B: CollabBackend> {
    /// TODO: docs.
    pub(crate) _peer_id: PeerId,

    /// TODO: docs.
    pub(crate) _server_tx: B::ServerTx,

    /// TODO: docs.
    pub(crate) _server_rx: B::ServerRx,
}

#[cfg(feature = "neovim")]
mod default_search_project_root {
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
        if let Some(lsp_res) =
            B::Buffer::lsp_root(buffer_id.clone(), ctx).transpose()
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

        let mut fs = ctx.fs();

        let home_dir = fs.home_dir().await.map_err(Error::HomeDir)?;

        let args = root_markers::FindRootArgs {
            marker: MARKERS,
            start_from: &buffer_path,
            stop_at: Some(&home_dir),
        };

        if let Some(res) = args.find(&mut fs).await.transpose() {
            return res.map_err(Error::FindRoot);
        }

        buffer_path
            .parent()
            .map(ToOwned::to_owned)
            .ok_or(Error::CouldntFindRoot(buffer_path))
    }

    pub(super) enum Error<B: CollabBackend> {
        BufNameNotAbsolutePath(String),
        CouldntFindRoot(fs::AbsPathBuf),
        FindRoot(root_markers::FindRootError<B::Fs, Markers>),
        HomeDir(<B::Fs as CollabFs>::HomeDirError),
        InvalidBufId(BufferId<B>),
        Lsp(<B::Buffer as CollabBuffer<B>>::LspRootError),
    }
}

#[cfg(feature = "neovim")]
mod root_markers {
    use core::fmt;
    use std::borrow::Cow;

    use futures_util::stream::{self, StreamExt};
    use futures_util::{pin_mut, select};
    use nvimx2::fs::{self, DirEntry};
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
        type Error;

        fn matches(
            &self,
            dir_entry: &Fs::DirEntry,
        ) -> impl Future<Output = Result<bool, Self::Error>>;
    }

    pub struct FindRootError<Fs: fs::Fs, M: RootMarker<Fs>> {
        /// The path to the file or directory at which the error occurred.
        pub path: fs::AbsPathBuf,

        /// The kind of error that occurred.
        pub kind: FindRootErrorKind<Fs, M>,
    }

    pub enum FindRootErrorKind<Fs: fs::Fs, M: RootMarker<Fs>> {
        DirEntry(DirEntryError<Fs>),
        Marker { dir_entry_name: Option<fs::FsNodeNameBuf>, err: M::Error },
        NodeAtStartPath(Fs::NodeAtPathError),
        ReadDir(Fs::ReadDirError),
        StartPathNotFound,
    }

    pub enum DirEntryError<Fs: fs::Fs> {
        Access(Fs::DirEntryError),
        Name(<Fs::DirEntry as fs::DirEntry>::NameError),
        NodeKind(<Fs::DirEntry as fs::DirEntry>::NodeKindError),
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
            let node_kind = fs
                .node_at_path(self.start_from)
                .await
                .map_err(FindRootErrorKind::NodeAtStartPath)
                .and_then(|maybe_node| {
                    maybe_node.ok_or(FindRootErrorKind::StartPathNotFound)
                })
                .map_err(|kind| FindRootError {
                    path: self.start_from.to_owned(),
                    kind,
                })?
                .kind();

            let mut dir = match node_kind {
                fs::FsNodeKind::Directory => self.start_from,
                fs::FsNodeKind::File => self
                    .start_from
                    .parent()
                    .expect("path is of file, so it must have a parent"),
                fs::FsNodeKind::Symlink => todo!("can't handle symlinks yet"),
            }
            .to_owned();

            loop {
                if self.contains_marker(&dir, fs).await? {
                    return Ok(Some(dir));
                }
                if self.stop_at == Some(&*dir) {
                    return Ok(None);
                }
                if !dir.pop() {
                    return Ok(None);
                }
            }
        }

        async fn contains_marker<Fs>(
            &self,
            dir_path: &fs::AbsPath,
            fs: &mut Fs,
        ) -> Result<bool, FindRootError<Fs, M>>
        where
            Fs: fs::Fs,
            M: RootMarker<Fs>,
        {
            let read_dir = fs
                .read_dir(dir_path)
                .await
                .map_err(|err| FindRootError {
                    path: dir_path.to_owned(),
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
                                path: dir_path.to_owned(),
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
                                        .ok()
                                        .map(|name| name.into_owned());
                                    Err(FindRootError {
                                        path: dir_path.to_owned(),
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
            dir_entry: &Fs::DirEntry,
        ) -> Result<bool, Self::Error> {
            Ok(dir_entry.name().await.map_err(DirEntryError::Name)?.as_ref()
                == ".git"
                && dir_entry
                    .is_directory()
                    .await
                    .map_err(DirEntryError::NodeKind)?)
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

    impl<Fs: fs::Fs> fmt::Display for DirEntryError<Fs> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                DirEntryError::Access(err) => err.fmt(f),
                DirEntryError::Name(err) => err.fmt(f),
                DirEntryError::NodeKind(err) => err.fmt(f),
            }
        }
    }
}
