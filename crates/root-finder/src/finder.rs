use std::io;

use e31e::fs::{AbsPath, AbsPathBuf};
use futures_util::{pin_mut, StreamExt};

use crate::Marker;

/// TODO: docs.
pub struct Finder;

impl Finder {
    /// TODO: docs.
    pub async fn find_root<T: Marker, F>(
        start_from: &AbsPath,
        marker: &T,
        // fs: &F,
    ) -> io::Result<Option<AbsPathBuf>> {
        // let mut dir = match start_from.parent() {
        //     Some(dir) => dir.to_owned(),
        //     None => {
        //         let root = AbsPathBuf::root();
        //         debug_assert_eq!(start_from, &*root);
        //         return contains_marker(&root, marker, fs)
        //             .await
        //             .map(|contains| contains.then_some(root));
        //     },
        // };
        //
        // loop {
        //     if contains_marker(&dir, marker, fs).await? {
        //         return Ok(Some(dir));
        //     }
        //     if !dir.pop() {
        //         return Ok(None);
        //     }
        // }
        todo!();
    }
}

async fn contains_marker(
    dir: &AbsPath,
    marker: &impl Marker,
    // fs: &impl Fs,
) -> io::Result<bool> {
    // let entries = fs.read_dir(dir).await?;
    // pin_mut!(entries);
    //
    // let mut path = dir.to_owned();
    // while let Some(entry) = entries.next().await {
    //     let file_name = fs.file_name(&entry).await?;
    //     path.push(file_name.as_str());
    //     let metadata = fs.metadata(&entry).await?;
    //     if marker.matches(&path, &metadata, fs).await? {
    //         return Ok(true);
    //     }
    //     path.pop();
    // }
    //
    // Ok(false)
    todo!();
}
