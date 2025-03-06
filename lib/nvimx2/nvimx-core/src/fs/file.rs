use core::error::Error;

use crate::ByteOffset;
use crate::fs::{AbsPath, Fs};

/// TODO: docs.
pub trait File {
    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type Error: Error;

    /// TODO: docs.
    type WriteError: Error;

    /// TODO: docs.
    fn len(&self) -> impl Future<Output = Result<ByteOffset, Self::Error>>;

    /// TODO: docs.
    fn parent(&self) -> impl Future<Output = <Self::Fs as Fs>::Directory>;

    /// TODO: docs.
    fn path(&self) -> &AbsPath;

    /// TODO: docs.
    fn write<C: AsRef<[u8]>>(
        &self,
        new_contents: C,
    ) -> impl Future<Output = Result<(), Self::WriteError>>;
}
