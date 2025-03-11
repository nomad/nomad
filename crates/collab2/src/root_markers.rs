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
                    "couldn't match marker with file or directory at {:?}: {}",
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
