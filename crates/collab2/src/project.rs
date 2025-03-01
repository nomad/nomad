use eerie::Replica;
use nvimx2::fs::Fs;

use crate::CollabBackend;

/// TODO: docs.
pub struct Project<B: CollabBackend> {
    _backend: core::marker::PhantomData<B>,
    pub(crate) replica: Replica,
}

impl<B: CollabBackend> Project<B> {
    /// TODO: docs.
    pub(crate) async fn flush(
        &self,
        _project_root: <B::Fs as Fs>::Directory,
        _fs: B::Fs,
    ) {
    }
}
