use ed::backend::{BackgroundExecutor, LocalExecutor};
use ed::fs;

use crate::executor::Executor;
use crate::fs::MockFs;

/// TODO: docs.
pub trait Config: 'static {
    /// TODO: docs.
    type Fs: fs::Fs;

    /// TODO: docs.
    type LocalExecutor: LocalExecutor;

    /// TODO: docs.
    type BackgroundExecutor: BackgroundExecutor;

    /// TODO: docs.
    fn fs(&mut self) -> Self::Fs;

    /// TODO: docs.
    fn local_executor(&mut self) -> &mut Self::LocalExecutor;

    /// TODO: docs.
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor;
}

#[derive(Default)]
pub struct DefaultConfig {
    pub fs: MockFs,
    pub executor: Executor,
}

impl Config for DefaultConfig {
    type Fs = MockFs;
    type LocalExecutor = Executor;
    type BackgroundExecutor = Executor;

    fn fs(&mut self) -> Self::Fs {
        self.fs.clone()
    }
    fn local_executor(&mut self) -> &mut Self::LocalExecutor {
        &mut self.executor
    }
    fn background_executor(&mut self) -> &mut Self::BackgroundExecutor {
        &mut self.executor
    }
}

impl From<MockFs> for DefaultConfig {
    fn from(fs: MockFs) -> Self {
        Self { fs, executor: Default::default() }
    }
}
