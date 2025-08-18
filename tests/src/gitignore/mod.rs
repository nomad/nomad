use core::ops::Deref;
use std::process::{Command, Stdio};

use abs_path::{AbsPath, node};
use ed::fs::Directory;
use ed::fs::os::OsFs;
use tempdir::{FsExt, TempDir};
use thread_pool::ThreadPool;
use walkdir::{GitIgnore, GitIgnoreFilterError};

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn simple() {
    let repo = GitRepository::init(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    assert!(repo.is_ignored(repo.path().join(node!("a.txt"))).unwrap());
    assert!(!repo.is_ignored(repo.path().join(node!("b.txt"))).unwrap());
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn changes_to_gitignore_are_picked_up() {
    let repo = GitRepository::init(mock::fs! {
        "a.txt": "",
        "b.txt": "",
        ".gitignore": "a.txt",
    });

    // Change the .gitignore file.
    std::fs::write(repo.path().join(node!(".gitignore")), "b.txt").unwrap();

    // Now 'b.txt' should be ignored, and 'a.txt' should not.
    assert!(repo.is_ignored(repo.path().join(node!("b.txt"))).unwrap());
    assert!(!repo.is_ignored(repo.path().join(node!("a.txt"))).unwrap());
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn slashed_dirs_are_ignored() {
    let repo = GitRepository::init(mock::fs! {
        "target": {},
        ".gitignore": "target/",
    });

    let ignored_res = repo.is_ignored(repo.path().join(node!("target")));
    assert_eq!(ignored_res, Ok(true));
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn errors_if_path_is_outside_repo() {
    let repo = GitRepository::init(mock::fs! {});
    let parent_path = repo.path().parent().unwrap();
    let err = repo.is_ignored(parent_path).unwrap_err();
    assert_eq!(
        err,
        GitIgnoreFilterError::PathOutsideRepo {
            path: parent_path.to_owned(),
            // On macOS the repo is created under /tmp, which is a symlink to
            // /private/tmp. Since git returns the canonical path in the error
            // message, we need to canonicalize it here too.
            repo_path: std::fs::canonicalize(repo.path())
                .unwrap()
                .try_into()
                .unwrap(),
        }
    );
}

struct GitRepository {
    dir: TempDir,
    gitignore: GitIgnore,
}

impl GitRepository {
    fn init(fs: mock::fs::MockFs) -> Self {
        let dir = futures_lite::future::block_on(async move {
            let tempdir = OsFs::default()
                .tempdir()
                .await
                .expect("couldn't create tempdir");

            tempdir
                .replicate_from(&fs.root())
                .await
                .expect("couldn't replicate from mock fs");

            tempdir
        });

        Command::new("git")
            .arg("init")
            .current_dir(dir.path())
            // Ignore all global config files.
            //
            // See https://stackoverflow.com/a/67512433 for more info.
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_SYSTEM", "/dev/null")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("failed to `git init` directory");

        Self {
            gitignore: GitIgnore::new(dir.path(), &mut ThreadPool::default())
                .unwrap(),
            dir,
        }
    }

    fn is_ignored(
        &self,
        path: impl AsRef<AbsPath>,
    ) -> Result<bool, GitIgnoreFilterError> {
        futures_lite::future::block_on(async {
            self.gitignore.is_ignored(path.as_ref()).await
        })
    }
}

impl Deref for GitRepository {
    type Target = TempDir;

    fn deref(&self) -> &Self::Target {
        &self.dir
    }
}
