#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use std::collections::HashSet;

use abs_path::AbsPathBuf;
use ed::fs::Directory;
use ed::fs::os::{OsDirectory, OsFs};
use ed::mock;

#[test]
#[cfg_attr(not(feature = "git-in-PATH"), ignore = "git is not in $PATH")]
fn gitignore_1() {
    let repo = <OsDirectory as GitRepository>::create(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    repo.init();

    assert_eq!(repo.non_ignored_paths(), paths(["/b.txt", "/.gitignore"]));
}

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
#[cfg_attr(feature = "git-in-PATH", ignore = "git is in $PATH")]
fn gitignore_is_ignored_if_git_is_not_in_path() {
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
    fn create(fs: mock::fs::MockFs) -> Self;

    /// `git init`s the repository.
    fn init(&self);

    /// Returns a `HashSet` containing the paths of all non-gitignored files
    /// and directories in the repository, relative to its root.
    fn non_ignored_paths(&self) -> HashSet<AbsPathBuf>;
}

impl GitRepository for OsDirectory {
    fn create(_fs: mock::fs::MockFs) -> Self {
        todo!();
    }

    fn init(&self) {
        std::process::Command::new("git")
            .arg("init")
            .current_dir(self.path())
            // Ignore all global config files.
            //
            // See https://stackoverflow.com/a/67512433 for more info.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .status()
            .expect("failed to `git init` directory");
    }

    fn non_ignored_paths(&self) -> HashSet<AbsPathBuf> {
        use futures_util::StreamExt;
        use walkdir::FsExt;

        futures_executor::block_on(async move {
            OsFs::default()
                .walk(self)
                .filter(walkdir::GitIgnore::new(self.path().to_owned()))
                .paths()
                .map(Result::unwrap)
                .map(|path| path.strip_prefix(self.path()).unwrap().to_owned())
                .collect::<HashSet<_>>()
                .await
        })
    }
}
