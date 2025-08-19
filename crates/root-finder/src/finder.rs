use abs_path::{AbsPath, AbsPathBuf};
use fs::{self, Directory, File, Metadata, Symlink};
use futures_util::StreamExt;

use crate::{FindRootError, Marker};

/// TODO: docs.
pub struct Finder<Fs> {
    fs: Fs,
}

impl<Fs: fs::Fs> Finder<Fs> {
    /// TODO: docs.
    pub async fn find_root<P, M>(
        &self,
        start_from: P,
        marker: M,
    ) -> Result<Option<AbsPathBuf>, FindRootError<Fs>>
    where
        P: AsRef<AbsPath>,
        M: Marker,
    {
        let node = self
            .fs
            .node_at_path(start_from.as_ref())
            .await
            .map_err(FindRootError::NodeAtStartPath)?
            .ok_or(FindRootError::StartPathNotFound)?;

        let mut dir = match node {
            fs::FsNode::Directory(dir) => dir,
            fs::FsNode::File(file) => {
                file.parent().await.map_err(FindRootError::FileParent)?
            },
            fs::FsNode::Symlink(symlink) => {
                symlink.parent().await.map_err(FindRootError::SymlinkParent)?
            },
        };

        loop {
            if contains_marker(&dir, &marker).await? {
                return Ok(Some(dir.path().to_owned()));
            }

            match dir.parent().await.map_err(FindRootError::DirParent)? {
                Some(new_parent) => dir = new_parent,
                None => return Ok(None),
            }
        }
    }

    /// TODO: docs.
    pub fn new(fs: Fs) -> Self {
        Self { fs }
    }
}

async fn contains_marker<Fs: fs::Fs>(
    dir: &Fs::Directory,
    marker: &impl Marker,
) -> Result<bool, FindRootError<Fs>> {
    let mut metas = dir.list_metas().await.map_err(FindRootError::ListDir)?;

    while let Some(meta_res) = metas.next().await {
        let meta = meta_res.map_err(FindRootError::ReadMetadata)?;
        let node_name = meta.name().map_err(FindRootError::MetadataName)?;
        let node_kind = meta.node_kind();
        if marker.matches(node_name, node_kind) {
            return Ok(true);
        }
    }

    Ok(false)
}
