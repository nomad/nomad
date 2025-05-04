#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;

use abs_path::AbsPathBuf;
use ed::fs::os::{OsDirectory, OsFs};
use ed::mock;

#[test]
#[cfg_attr(not(feature = "git-in-PATH"), ignore = "git is not in $PATH")]
fn gitignore_is_ignored_if_not_in_git_repo() {
    let repo = <OsDirectory as GitRepository>::create(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    assert_eq!(
        repo.non_ignored_paths(),
        paths(["/a.txt", "/b.txt", "/.gitignore"])
    );
}

#[test]
#[cfg_attr(not(feature = "git-in-PATH"), ignore = "git is not in $PATH")]
fn gitignore_is_respected_if_in_git_repo() {
    let repo = <OsDirectory as GitRepository>::create(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    repo.init();

    assert_eq!(repo.non_ignored_paths(), paths(["/b.txt", "/.gitignore"]));
}

fn paths(
    paths: impl IntoIterator<Item = impl AsRef<str>>,
) -> HashSet<AbsPathBuf> {
    paths
        .into_iter()
        .map(|path| path.as_ref().parse::<AbsPathBuf>().unwrap())
        .collect()
}

trait GitRepository {
    /// Creates a directory from the given [`mock::fs::MockFs`].
    ///
    /// Note that the returned directory will not be initialized as a Git
    /// repository. To do so, call [`Self::init`].
    fn create(initial_fs: mock::fs::MockFs) -> Self;

    /// `git init`s the repository.
    fn init(&self);

    /// Returns a `HashSet` containing the paths of all non-gitignored files
    /// and directories in the repository, relative to its root.
    fn non_ignored_paths(&self) -> HashSet<AbsPathBuf>;
}

impl GitRepository for OsDirectory {
    fn create(_initial_fs: mock::fs::MockFs) -> Self {
        todo!();
    }

    fn init(&self) {
        todo!();
    }

    fn non_ignored_paths(&self) -> HashSet<AbsPathBuf> {
        use ed::fs::Directory;
        use futures_util::StreamExt;
        use walkdir::{FsExt, GitIgnore};

        futures_executor::block_on(async move {
            OsFs::default()
                .walk(self)
                .filter(GitIgnore::new(self.path().to_owned()))
                .paths()
                .map(Result::unwrap)
                .map(|path| path.strip_prefix(self.path()).unwrap().to_owned())
                .collect::<HashSet<_>>()
                .await
        })
    }
}
