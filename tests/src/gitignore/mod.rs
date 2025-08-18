use core::ops::Deref;
use std::process::{Command, Stdio};
use std::{thread, time};

use abs_path::{AbsPath, node, path};
use ed::fs::Directory;
use ed::fs::os::OsFs;
use futures_lite::future;
use tempdir::{FsExt, TempDir};
use thread_pool::ThreadPool;
use walkdir::{CreateError, GitIgnore, IgnoreError};

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
        IgnoreError::PathOutsideRepo {
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

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn errors_if_path_doesnt_exist() {
    let repo = GitRepository::init(mock::fs! {});
    let path = path!("/foo/bar");
    let err = repo.is_ignored(path).unwrap_err();
    let IgnoreError::PathDoesNotExist(error_path) = err else {
        panic!("expected PathDoesNotExist error, got: {err:?}");
    };
    // Git stops at the first component that doesn't exist, so we can't assert
    // full equality here.
    assert!(path.starts_with(error_path));
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
#[cfg_attr(windows, ignore = "'kill' is not available on Windows")]
fn exit_status_is_returned_if_process_is_killed() {
    let repo = GitRepository::init(mock::fs! {});
    let gitignore_pid = repo.gitignore.process_id().unwrap();

    // Kill the process.
    Command::new("kill")
        .arg("-TERM")
        .arg(gitignore_pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to kill gitignore process");

    let err = repo.is_ignored(repo.path()).unwrap_err();
    let IgnoreError::ProcessExited(maybe_exit_status) = err else {
        panic!("expected ProcessExited error, got: {err:?}");
    };

    // Calling `is_ignored` continues returning the same exit status.
    assert_eq!(
        repo.is_ignored(repo.path()).unwrap_err(),
        IgnoreError::ProcessExited(maybe_exit_status)
    );

    // Getting the PID will now fail and return the exit status instead.
    assert_eq!(repo.gitignore.process_id().unwrap_err(), maybe_exit_status);
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn process_terminates_when_all_gitignore_instances_are_dropped() {
    let repo = GitRepository::init(mock::fs! {});
    let gitignore_2 = repo.gitignore.clone();
    let gitignore_pid = repo.gitignore.process_id().unwrap();

    // Dropping the second instance shouldn't terminate the process.
    drop(gitignore_2);
    assert!(is_process_alive(gitignore_pid));

    // Drop the repo, which will drop the last GitIgnore instance.
    drop(repo);

    // Wait a bit to ensure the process has time to terminate.
    let mut num_tries = 0;
    let num_max_tries = 10;
    let retry_internal = time::Duration::from_millis(50);
    while num_tries < num_max_tries {
        if !is_process_alive(gitignore_pid) {
            return;
        }
        num_tries += 1;
        thread::sleep(retry_internal);
    }
    panic!(
        "process should have terminated after all GitIgnore instances were \
         dropped, but it's still alive after {}ms",
        num_tries * retry_internal.as_millis() as usize
    );
}

#[test]
#[cfg_attr(git_in_PATH, ignore = "git is in $PATH")]
fn creating_gitignore_fails_if_git_is_not_in_path() {
    let tempdir = future::block_on(OsFs::default().tempdir()).unwrap();
    let mut spawner = ThreadPool::default();
    let err = GitIgnore::new(tempdir.path(), &mut spawner).unwrap_err();
    assert_eq!(err, CreateError::GitNotInPath);
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn creating_gitignore_fails_if_path_doesnt_exist() {
    let mut spawner = ThreadPool::default();
    let err = GitIgnore::new(path!("/foo/bar"), &mut spawner).unwrap_err();
    assert_eq!(err, CreateError::InvalidPath);
}

#[test]
#[cfg_attr(not(git_in_PATH), ignore = "git is not in $PATH")]
fn creating_gitignore_fails_if_not_in_git_repo() {
    let tempdir = future::block_on(OsFs::default().tempdir()).unwrap();
    let mut spawner = ThreadPool::default();
    let err = GitIgnore::new(tempdir.path(), &mut spawner).unwrap_err();
    assert_eq!(err, CreateError::PathNotInGitRepository);
}

struct GitRepository {
    dir: TempDir,
    gitignore: GitIgnore,
}

impl GitRepository {
    fn init(fs: mock::fs::MockFs) -> Self {
        let dir = future::block_on(async move {
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
    ) -> Result<bool, IgnoreError> {
        future::block_on(async {
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

/// Checks if a process with the given PID is alive.
#[track_caller]
fn is_process_alive(pid: u32) -> bool {
    Command::new("kill")
        .arg("-0")
        .arg(pid.to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("failed to check if process is alive")
        .code()
        .expect("'kill -0' should have an exit code")
        == 0
}
