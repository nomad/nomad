use nvimx2::backend::{Backend, BufferId};
use nvimx2::fs::AbsPathBuf;
use nvimx2::{AsyncCtx, notify};

/// TODO: docs.
pub trait CollabBackend: Backend {
    /// TODO: docs.
    type FindProjectRootError: notify::Error;

    /// TODO: docs.
    fn project_root(
        buffer_id: BufferId<Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::FindProjectRootError>>;
}

#[cfg(feature = "neovim")]
mod neovim {
    use nvimx2::neovim::Neovim;

    use super::*;

    impl CollabBackend for Neovim {
        type FindProjectRootError = core::convert::Infallible;

        async fn project_root(
            _buffer_id: BufferId<Self>,
            _ctx: &mut AsyncCtx<'_, Self>,
        ) -> Result<AbsPathBuf, Self::FindProjectRootError> {
            todo!()
        }
    }
}
