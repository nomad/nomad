/// TODO: docs.
#[derive(
    cauchy::Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq,
)]
#[display("{_0}")]
pub enum FindRootError<Fs: fs::Fs> {
    /// TODO: docs.
    DirParent(<Fs::Directory as fs::Directory>::ParentError),

    /// TODO: docs.
    FileParent(<Fs::File as fs::File>::ParentError),

    /// TODO: docs.
    ListDir(<Fs::Directory as fs::Directory>::ListError),

    /// TODO: docs.
    MetadataName(fs::MetadataNameError),

    /// TODO: docs.
    NodeAtStartPath(Fs::NodeAtPathError),

    /// TODO: docs.
    ReadMetadata(<Fs::Directory as fs::Directory>::ReadMetadataError),

    /// TODO: docs.
    #[display("the starting path does not exist")]
    StartPathNotFound,

    /// TODO: docs.
    SymlinkParent(<Fs::Symlink as fs::Symlink>::ParentError),
}
