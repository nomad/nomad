use core::error::Error;

use futures_lite::Stream;

use crate::fs::{AbsPath, Fs, FsNodeName, Metadata};

/// TODO: docs.
pub trait Directory: Sized {
    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type Metadata: Metadata<Timestamp = <Self::Fs as Fs>::Timestamp>;

    /// TODO: docs.
    type CreateDirectoryError: Error;

    /// TODO: docs.
    type CreateFileError: Error;

    /// TODO: docs.
    type ClearError: Error;

    /// TODO: docs.
    type DeleteError: Error;

    /// TODO: docs.
    type ReadEntryError: Error;

    /// TODO: docs.
    type ReadError: Error;

    /// TODO: docs.
    fn create_directory(
        &self,
        directory_name: &FsNodeName,
    ) -> impl Future<Output = Result<Self, Self::CreateDirectoryError>>;

    /// TODO: docs.
    fn create_file(
        &self,
        file_name: &FsNodeName,
    ) -> impl Future<Output = Result<<Self::Fs as Fs>::File, Self::CreateFileError>>;

    /// TODO: docs.
    fn clear(&self) -> impl Future<Output = Result<(), Self::ClearError>>;

    /// TODO: docs.
    fn delete(self) -> impl Future<Output = Result<(), Self::DeleteError>>;

    /// TODO: docs.
    #[inline]
    fn name(&self) -> Option<&FsNodeName> {
        self.path().node_name()
    }

    /// TODO: docs.
    fn parent(
        &self,
    ) -> impl Future<Output = Option<<Self::Fs as Fs>::Directory>>;

    /// TODO: docs.
    fn path(&self) -> &AbsPath;

    /// TODO: docs.
    fn read(
        &self,
    ) -> impl Future<
        Output = Result<
            impl Stream<Item = Result<Self::Metadata, Self::ReadEntryError>>
            + use<Self>,
            Self::ReadError,
        >,
    >;
}
