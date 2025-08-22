use crate::{BackgroundSpawner, LocalSpawner};

/// TODO: docs.
pub trait Executor {
    /// TODO: docs.
    type LocalSpawner: LocalSpawner;

    /// TODO: docs.
    type BackgroundSpawner: BackgroundSpawner;

    /// TODO: docs.
    fn run<Fut: Future>(
        &mut self,
        future: Fut,
    ) -> impl Future<Output = Fut::Output> + use<Self, Fut>;

    /// TODO: docs.
    fn local_spawner(&mut self) -> &mut Self::LocalSpawner;

    /// TODO: docs.
    fn background_spawner(&mut self) -> &mut Self::BackgroundSpawner;
}
