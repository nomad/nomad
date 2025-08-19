use core::error::Error;
use core::fmt;

use abs_path::{AbsPath, AbsPathBuf};
use ed::notify;
use fs::{Directory, File, MetadataNameError, Symlink};
use futures_util::select;
use futures_util::stream::{self, StreamExt};

pub struct FindRootArgs<'a, M> {
    /// The marker used to determine if a directory is the root.
    pub(super) marker: M,

    /// The path to the first directory to search for markers in.
    ///
    /// If this points to a file, the search will start from its parent.
    pub(super) start_from: &'a AbsPath,

    /// The path to the last directory to search for markers in, if any.
    ///
    /// If set and no root marker is found within it, the search is cut
    /// short instead of continuing with its parent.
    pub(super) stop_at: Option<&'a AbsPath>,
}

pub struct GitDirectory;

pub trait RootMarker<Fs: fs::Fs> {
    type Error: Error;

    fn matches(
        &self,
        metadata: &Fs::Metadata,
    ) -> impl Future<Output = Result<bool, Self::Error>>;
}

#[derive(cauchy::Debug, derive_more::Display, cauchy::PartialEq)]
#[display("{_0}")]
pub enum FindRootError<Fs: fs::Fs, M: RootMarker<Fs>> {
    DirParent(<Fs::Directory as fs::Directory>::ParentError),
    FileParent(<Fs::File as fs::File>::ParentError),
    FollowSymlink(<Fs::Symlink as fs::Symlink>::FollowError),
    Marker(M::Error),
    NodeAtStartPath(Fs::NodeAtPathError),
    ReadDir(<Fs::Directory as fs::Directory>::ListError),
    ReadMetadata(ReadMetadataError<Fs>),
    #[display("no file or directory found at {_0:?}")]
    StartPathNotFound(AbsPathBuf),
    #[display("starting point at {_0:?} is a dangling symlink")]
    StartsAtDanglingSymlink(AbsPathBuf),
}

#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum ReadMetadataError<Fs: fs::Fs> {
    Access(<Fs::Directory as fs::Directory>::ReadMetadataError),
    Name(MetadataNameError),
}

impl<M> FindRootArgs<'_, M> {
    pub(super) async fn find<Fs>(
        self,
        fs: &mut Fs,
    ) -> Result<Option<AbsPathBuf>, FindRootError<Fs, M>>
    where
        Fs: fs::Fs,
        M: RootMarker<Fs>,
    {
        let mut node = fs
            .node_at_path(self.start_from)
            .await
            .map_err(FindRootError::NodeAtStartPath)
            .and_then(|maybe_node| {
                maybe_node.ok_or(FindRootError::StartPathNotFound(
                    self.start_from.to_owned(),
                ))
            })?;

        let mut dir = loop {
            match node {
                fs::FsNode::Directory(dir) => break dir,
                fs::FsNode::File(file) => {
                    break file
                        .parent()
                        .await
                        .map_err(FindRootError::FileParent)?;
                },
                fs::FsNode::Symlink(symlink) => {
                    node = symlink
                        .follow_recursively()
                        .await
                        .map_err(FindRootError::FollowSymlink)
                        .and_then(|maybe_target| {
                            maybe_target.ok_or(
                                FindRootError::StartsAtDanglingSymlink(
                                    self.start_from.to_owned(),
                                ),
                            )
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
            let Some(parent) =
                dir.parent().await.map_err(FindRootError::DirParent)?
            else {
                return Ok(None);
            };
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
        let mut metas =
            dir.list_metas().await.map_err(FindRootError::ReadDir)?.fuse();

        let mut check_marker_matches = stream::FuturesUnordered::new();

        loop {
            select! {
                meta_res = metas.select_next_some() => {
                    let metadata = meta_res
                        .map_err(ReadMetadataError::Access)
                        .map_err(FindRootError::ReadMetadata)?;

                    let fut = async move {
                        self.marker
                            .matches(&metadata)
                            .await
                            .map_err(FindRootError::Marker)
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
    type Error = ReadMetadataError<Fs>;

    async fn matches(
        &self,
        metadata: &Fs::Metadata,
    ) -> Result<bool, Self::Error> {
        use fs::Metadata;
        Ok(metadata.name().map_err(ReadMetadataError::Name)? == ".git"
            && metadata.node_kind().is_dir())
    }
}

impl<Fs, M> notify::Error for FindRootError<Fs, M>
where
    Fs: fs::Fs,
    M: RootMarker<Fs>,
    M::Error: fmt::Display,
{
    fn to_message(&self) -> (notify::Level, notify::Message) {
        (notify::Level::Error, notify::Message::from_display(self))
    }
}
