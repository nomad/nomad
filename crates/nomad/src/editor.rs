use core::ops::AddAssign;

use collab_fs::{AbsUtf8PathBuf, Fs};

use crate::{Buffer, Spawner};

/// TODO: docs.
pub trait Editor: Sized + 'static {
    /// TODO: docs.
    type Api: Default + AddAssign<Self::ModuleApi>;

    /// TODO: docs.
    type Buffer<'ed>: Buffer<Self>;

    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type ModuleApi;

    /// TODO: docs.
    type Spawner: Spawner;

    /// TODO: docs.
    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn fs(&self) -> Self::Fs;

    /// TODO: docs.
    fn get_buffer(
        &mut self,
        id: <Self::Buffer<'_> as Buffer<Self>>::Id,
    ) -> Option<Self::Buffer<'_>>;

    /// TODO: docs.
    fn log_dir(&self) -> AbsUtf8PathBuf;

    /// TODO: docs.
    fn spawner(&self) -> Self::Spawner;
}
