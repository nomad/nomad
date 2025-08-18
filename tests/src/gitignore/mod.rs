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
fn gitignore_1() {
    futures_lite::future::block_on(async {
        let repo = GitRepository::init(mock::fs! {
            "a.txt": "",
            "b.txt": "",
            ".gitignore": "a.txt",
        })
        .await;

        assert!(
            repo.is_ignored(repo.path().join(node!("a.txt"))).await.unwrap(),
        );

        assert!(
            !repo.is_ignored(repo.path().join(node!("b.txt"))).await.unwrap(),
        );
    });
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn gitignore_2() {
    futures_lite::future::block_on(async {
        let repo = GitRepository::init(mock::fs! {
            "a.txt": "",
            "b.txt": "",
            ".gitignore": "a.txt",
        })
        .await;

        // Change the .gitignore file.
        std::fs::write(repo.path().join(node!(".gitignore")), "b.txt")
            .unwrap();

        // Now 'b.txt' should be ignored, and 'a.txt' should not.

        assert!(
            repo.is_ignored(repo.path().join(node!("b.txt"))).await.unwrap(),
        );

        assert!(
            !repo.is_ignored(repo.path().join(node!("a.txt"))).await.unwrap(),
        );
    });
}

struct GitRepository {
    dir: TempDir,
    gitignore: GitIgnore,
}

impl GitRepository {
    async fn init(fs: mock::fs::MockFs) -> Self {
        let tempdir =
            OsFs::default().tempdir().await.expect("couldn't create tempdir");

        tempdir
            .replicate_from(&fs.root())
            .await
            .expect("couldn't replicate from mock fs");

        Command::new("git")
            .arg("init")
            .current_dir(tempdir.path())
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
            gitignore: GitIgnore::new(
                tempdir.path(),
                &mut ThreadPool::default(),
            )
            .unwrap(),
            dir: tempdir,
        }
    }

    async fn is_ignored(
        &self,
        path: impl AsRef<AbsPath>,
    ) -> Result<bool, GitIgnoreFilterError> {
        self.gitignore.is_ignored(path.as_ref()).await
    }
}

impl Deref for GitRepository {
    type Target = TempDir;

    fn deref(&self) -> &Self::Target {
        &self.dir
    }
}
