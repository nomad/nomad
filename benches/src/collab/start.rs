use criterion::BenchmarkGroup;
use criterion::measurement::WallTime;

pub(crate) fn benches(group: &mut BenchmarkGroup<'_, WallTime>) {
    #[cfg(feature = "neovim-repo")]
    read_neovim::from_mock_fs(group);

    #[cfg(feature = "neovim-repo")]
    read_neovim::from_real_fs(group);
}

#[cfg(feature = "neovim-repo")]
mod read_neovim {
    use collab::CollabBackend;
    use collab::mock::CollabMock;
    use criterion::BenchmarkId;
    use ed::AsyncCtx;
    use ed::fs::os::{OsDirectory, OsFs};
    use ed::fs::{Directory, Fs};
    use futures_lite::future;
    use mock::fs::MockFs;
    use mock::{BackendExt, Mock};
    use thread_pool::ThreadPool;
    use walkdir::GitIgnore;

    use super::*;

    pub(super) fn from_mock_fs(group: &mut BenchmarkGroup<'_, WallTime>) {
        CollabMock::new(
            Mock::<MockFs>::default()
                .with_background_executor(ThreadPool::new()),
        )
        .block_on(async |ctx| {
            // Replicate the Neovim repo into the root of the mock filesystem.
            ctx.fs().root().replicate_from(&neovim_repo()).await.unwrap();
            bench_read_project(ctx.fs().root(), "mock_fs", ctx, group);
        });
    }

    pub(super) fn from_real_fs(group: &mut BenchmarkGroup<'_, WallTime>) {
        CollabMock::new(
            Mock::<OsFs>::default()
                .with_background_executor(ThreadPool::new()),
        )
        .with_project_filter(|project_root| {
            GitIgnore::new(project_root.path().to_owned())
        })
        .block_on(async |ctx| {
            bench_read_project(neovim_repo(), "real_fs", ctx, group);
        });
    }

    fn neovim_repo() -> OsDirectory {
        future::block_on(async {
            OsFs::default()
                .node_at_path(crate::generated::collab::NEOVIM_REPO_PATH)
                .await
                .unwrap()
                .unwrap()
                .unwrap_directory()
        })
    }

    /// Benchmarks reading the project under the given root.
    fn bench_read_project<B: CollabBackend>(
        project_root: <B::Fs as Fs>::Directory,
        fs_name: &str,
        ctx: &mut AsyncCtx<'_, B>,
        group: &mut BenchmarkGroup<'_, WallTime>,
    ) where
        <B::Fs as Fs>::Directory: Clone,
    {
        let bench_id = BenchmarkId::new(
            "start",
            format_args!("read_neovim_from_{fs_name}"),
        );

        group.bench_function(bench_id, |b| {
            b.iter(|| {
                future::block_on(collab::start::benches::read_project(
                    project_root.clone(),
                    ctx,
                ))
                .unwrap()
            });
        });
    }
}
