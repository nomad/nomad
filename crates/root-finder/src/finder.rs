use abs_path::{AbsPath, AbsPathBuf};
use ed::fs::{self, Directory, File, Metadata};
use futures_util::{StreamExt, pin_mut};

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
            fs::FsNode::Symlink(_) => todo!("can't handle symlinks yet"),
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
    let entries = dir.read().await.map_err(FindRootError::ReadDir)?;

    pin_mut!(entries);

    while let Some(res) = entries.next().await {
        let entry = res.map_err(FindRootError::ReadDirEntry)?;
        let node_name = entry.name().map_err(FindRootError::DirEntryName)?;
        let node_kind = entry.node_kind();
        if marker.matches(&node_name, node_kind) {
            return Ok(true);
        }
    }

    Ok(false)
}
