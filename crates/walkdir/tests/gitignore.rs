#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use core::fmt;
use std::collections::HashSet;

use abs_path::{AbsPathBuf, node, path};
use ed::fs::Directory;
use ed::fs::os::OsFs;
use ed::mock;
use tempdir::TempDir;
use walkdir::GitIgnore;

#[test]
#[cfg_attr(not(feature = "git-in-PATH"), ignore = "git is not in $PATH")]
fn gitignore_1() {
    let repo = <TempDir as GitRepository>::create(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    repo.init();

    assert_eq!(
        repo.non_ignored_paths().remove_git_dir(),
        ["/b.txt", "/.gitignore"]
    );
}

#[test]
#[cfg_attr(not(feature = "git-in-PATH"), ignore = "git is not in $PATH")]
fn gitignore_is_ignored_if_not_in_git_repo() {
    let repo = <TempDir as GitRepository>::create(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    assert_eq!(repo.non_ignored_paths(), ["/a.txt", "/b.txt", "/.gitignore"]);
}

#[test]
#[cfg_attr(feature = "git-in-PATH", ignore = "git is in $PATH")]
fn gitignore_is_ignored_if_git_is_not_in_path() {
    let repo = <TempDir as GitRepository>::create(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    assert_eq!(repo.non_ignored_paths(), ["/a.txt", "/b.txt", "/.gitignore"]);
}

#[test]
#[cfg_attr(not(feature = "git-in-PATH"), ignore = "git is not in $PATH")]
fn gitignore_cache_is_refreshed_after_expiration() {
    let repo = <TempDir as GitRepository>::create(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    repo.init();

    let gitignore = GitIgnore::new(repo.path().to_owned());

    assert_eq!(
        repo.non_ignored_paths_with_gitignore(&gitignore).remove_git_dir(),
        ["/b.txt", "/.gitignore"]
    );

    // Change the .gitignore file.
    std::fs::write(repo.path().join(node!(".gitignore")), "b.txt").unwrap();

    // We won't react to the change until the GitIgnore cache expires.
    std::thread::sleep(GitIgnore::REFRESH_IGNORED_PATHS_AFTER / 2);
    assert_eq!(
        repo.non_ignored_paths_with_gitignore(&gitignore).remove_git_dir(),
        ["/b.txt", "/.gitignore"]
    );

    // Now the cache will be refreshed and we'll detect the change.
    std::thread::sleep(GitIgnore::REFRESH_IGNORED_PATHS_AFTER / 2);
    assert_eq!(
        repo.non_ignored_paths_with_gitignore(&gitignore).remove_git_dir(),
        ["/a.txt", "/.gitignore"]
    );
}

trait GitRepository: Directory {
    /// Creates a directory from the given [`mock::fs::MockFs`].
    ///
    /// Note that the returned directory will not be initialized as a Git
    /// repository. To do so, call [`Self::init`].
    fn create(fs: mock::fs::MockFs) -> Self;

    /// `git init`s the repository.
    fn init(&self);

    /// Returns the paths of all non-gitignored files and directories in the
    /// repository, relative to its root.
    fn non_ignored_paths(&self) -> NonIgnoredPaths {
        self.non_ignored_paths_with_gitignore(&GitIgnore::new(
            self.path().to_owned(),
        ))
    }

    /// Same as [`Self::non_ignored_paths`], but uses the given [`GitIgnore`]
    /// instance instead of creating a new one.
    fn non_ignored_paths_with_gitignore(
        &self,
        gitignore: &GitIgnore,
    ) -> NonIgnoredPaths;
}

impl GitRepository for TempDir {
    fn create(fs: mock::fs::MockFs) -> Self {
        use tempdir::FsExt;

        futures_executor::block_on(async move {
            let tempdir = OsFs::default()
                .tempdir()
                .await
                .expect("couldn't create tempdir");

            tempdir
                .replicate_from(&fs.root())
                .await
                .expect("couldn't replicate from mock fs");

            tempdir
        })
    }

    fn init(&self) {
        use std::process::{Command, Stdio};
        Command::new("git")
            .arg("init")
            .current_dir(self.path())
            // Ignore all global config files.
            //
            // See https://stackoverflow.com/a/67512433 for more info.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("failed to `git init` directory");
    }

    fn non_ignored_paths_with_gitignore(
        &self,
        gitignore: &GitIgnore,
    ) -> NonIgnoredPaths {
        use futures_util::StreamExt;
        use walkdir::FsExt;

        futures_executor::block_on(async move {
            NonIgnoredPaths {
                inner: OsFs::default()
                    .walk(self)
                    .filter(gitignore)
                    .paths()
                    .map(Result::unwrap)
                    .map(|path| {
                        path.strip_prefix(self.path()).unwrap().to_owned()
                    })
                    .collect::<HashSet<_>>()
                    .await,
            }
        })
    }
}

struct NonIgnoredPaths {
    inner: HashSet<AbsPathBuf>,
}

impl NonIgnoredPaths {
    /// Removes all the paths of files and directories in the `/.git`
    /// directory.
    fn remove_git_dir(mut self) -> Self {
        self.inner.retain(|path| !path.starts_with(path!("/.git")));
        self
    }
}

impl<Paths, Path> PartialEq<Paths> for NonIgnoredPaths
where
    Paths: IntoIterator<Item = Path> + Clone,
    Path: AsRef<str>,
{
    fn eq(&self, other: &Paths) -> bool {
        let other = other
            .clone()
            .into_iter()
            .map(|path| {
                path.as_ref().parse::<AbsPathBuf>().expect("invalid path")
            })
            .collect::<HashSet<_>>();
        self.inner == other
    }
}

impl fmt::Debug for NonIgnoredPaths {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set()
            .entries(self.inner.iter().map(AsRef::<str>::as_ref))
            .finish()
    }
}
