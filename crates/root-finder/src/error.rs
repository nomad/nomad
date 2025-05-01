use ed::fs;

/// TODO: docs.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum FindRootError<Fs: fs::Fs> {
    /// TODO: docs.
    DirEntryName(fs::MetadataNameError),

    /// TODO: docs.
    DirParent(<Fs::Directory as fs::Directory>::ParentError),

    /// TODO: docs.
    FileParent(<Fs::File as fs::File>::ParentError),

    /// TODO: docs.
    NodeAtStartPath(Fs::NodeAtPathError),

    /// TODO: docs.
    ReadDir(<Fs::Directory as fs::Directory>::ReadError),

    /// TODO: docs.
    ReadDirEntry(<Fs::Directory as fs::Directory>::ReadEntryError),

    /// TODO: docs.
    #[display("the starting path does not exist")]
    StartPathNotFound,
}
