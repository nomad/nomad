use core::marker::PhantomData;

use abs_path::AbsPath;
use ed::fs::{self, Directory};

use crate::walkdir::ForEachHandler;
use crate::{Filter, Filtered, WalkDir, WalkError};

/// TODO: docs.
pub trait FsExt: fs::Fs {
    /// TODO: docs.
    #[inline]
    fn walk<'dir>(
        &self,
        dir: &'dir Self::Directory,
    ) -> Walker<'dir, Self, Self> {
        Walker::new(self.clone(), dir)
    }
}

/// TODO: docs.
pub struct Walker<'dir, W, Fs> {
    inner: W,
    dir_path: &'dir AbsPath,
    fs: PhantomData<Fs>,
}

impl<'dir, W, Fs> Walker<'dir, W, Fs>
where
    W: WalkDir<Fs>,
    Fs: fs::Fs,
{
    /// TODO: docs.
    #[inline]
    pub fn filter<F>(self, filter: F) -> Walker<'dir, Filtered<F, W>, Fs>
    where
        F: Filter<Fs>,
    {
        Walker {
            inner: self.inner.filter(filter),
            dir_path: self.dir_path,
            fs: self.fs,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn for_each<Err: Send>(
        &self,
        handler: impl ForEachHandler<Fs, Err>,
    ) -> impl Future<Output = Result<(), WalkError<Fs, W, Err>>> + Send
    where
        W: Sync,
    {
        use send_future::SendFuture;
        async move { self.inner.for_each(self.dir_path, handler).send().await }
    }

    /// TODO: docs.
    #[inline]
    pub fn into_inner(self) -> W {
        self.inner
    }

    /// TODO: docs.
    #[inline]
    pub fn new(inner: W, dir: &'dir <Fs as fs::Fs>::Directory) -> Self {
        Self { inner, dir_path: dir.path(), fs: PhantomData }
    }
}

impl<Fs: fs::Fs> FsExt for Fs {}
