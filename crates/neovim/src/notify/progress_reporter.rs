use editor::context::BorrowState;
use editor::{Context, Editor};
use executor::Executor;

use crate::{Neovim, notify};

/// TODO: docs.
pub enum ProgressReporter {
    /// TODO: docs.
    NvimEcho(notify::NvimEchoProgressReporter),

    /// TODO: docs.
    NvimNotify(notify::NvimNotifyProgressReporter),
}

impl ProgressReporter {
    /// Creates a new progress reporter.
    pub fn new(ctx: &mut Context<Neovim, impl BorrowState>) -> Self {
        let namespace = ctx.namespace().clone();

        ctx.with_editor(|nvim| {
            let local_spawner = nvim.executor().local_spawner();

            if notify::NvimNotify::is_installed() {
                Self::NvimNotify(notify::NvimNotifyProgressReporter::new(
                    namespace,
                    local_spawner,
                ))
            } else {
                Self::NvimEcho(notify::NvimEchoProgressReporter::new(
                    namespace,
                    local_spawner,
                ))
            }
        })
    }

    /// TODO: docs.
    pub fn report_error(self, chunks: notify::Chunks) {
        match self {
            Self::NvimEcho(inner) => inner.report_error(chunks),
            Self::NvimNotify(inner) => inner.report_error(chunks),
        }
    }

    /// TODO: docs.
    pub fn report_progress(&self, chunks: notify::Chunks) {
        match self {
            Self::NvimEcho(inner) => inner.report_progress(chunks),
            Self::NvimNotify(inner) => inner.report_progress(chunks),
        }
    }

    /// TODO: docs.
    pub fn report_success(self, chunks: notify::Chunks) {
        match self {
            Self::NvimEcho(inner) => inner.report_success(chunks),
            Self::NvimNotify(inner) => inner.report_success(chunks),
        }
    }
}
