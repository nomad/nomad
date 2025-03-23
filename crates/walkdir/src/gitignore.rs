use core::time::Duration;
use core::{iter, str};
use std::io;
use std::path::MAIN_SEPARATOR;
use std::process::{Command, ExitStatus};
use std::sync::Mutex;
use std::time::Instant;

use compact_str::CompactString;
use ed::fs::{self, AbsPath, AbsPathBuf, Metadata, NodeName, os};

use crate::{DirEntry, Either, Filter};

/// TODO: docs.
pub struct GitIgnore {
    inner: Mutex<GitIgnoreInner>,
}

/// TODO: docs.
#[derive(Debug, thiserror::Error)]
pub enum GitIgnoreError {
    #[error("Running {cmd:?} failed: {0}", cmd = GitIgnore::command())]
    GitCommand(io::Error),

    #[error(
        "Running {cmd:?} failed with exit code {0:?}",
        cmd = GitIgnore::command()
    )]
    FailedCommand(ExitStatus),

    #[error("{node_name:?} is not a valid node name: {err:?}")]
    NotNodeName { node_name: String, err: fs::InvalidNodeNameError },

    #[error("{path:?} is not in {dir_path:?}")]
    NotInDir { path: AbsPathBuf, dir_path: AbsPathBuf },

    #[error(
        "The stdout of {cmd:?} was not valid UTF-8",
        cmd = GitIgnore::command()
    )]
    StdoutNotUtf8,
}

struct GitIgnoreInner {
    dir_path: AbsPathBuf,
    ignored_paths: IgnoredPaths,
    last_refreshed_ignored_paths_at: Instant,
}

#[derive(Default)]
struct IgnoredPaths {
    inner: Vec<CompactString>,
}

impl GitIgnore {
    /// TODO: docs.
    const REFRESH_IGNORED_PATHS_AFTER: Duration = Duration::from_secs(10);

    /// TODO: docs.
    pub fn new(dir_path: AbsPathBuf) -> Self {
        let inner = GitIgnoreInner::new_outdated(dir_path);
        debug_assert!(inner.is_outdated());
        Self { inner: Mutex::new(inner) }
    }

    fn command() -> Command {
        let mut cmd = Command::new("git");
        cmd.args(["ls-files", "--others", "--ignored", "--exclude-standard"]);
        cmd
    }

    fn with_inner<R>(
        &self,
        f: impl FnOnce(&GitIgnoreInner) -> R,
    ) -> Result<R, GitIgnoreError> {
        let inner = &mut *self.inner.lock().expect("poisoned mutex");

        if inner.is_outdated() {
            inner.refresh()?;
        }

        Ok(f(inner))
    }
}

/// TODO: docs.
trait Path: Sized {
    /// TODO: docs.
    fn components(&self) -> impl Iterator<Item = &NodeName> + '_;

    /// TODO: docs.
    fn strip_prefix(&self, s: &AbsPath) -> Option<Self>;
}

impl GitIgnoreInner {
    fn is_ignored(&self, path: impl Path) -> Result<bool, GitIgnoreError> {
        path.strip_prefix(&self.dir_path)
            .map(|stripped| self.ignored_paths.contains(stripped.components()))
            .ok_or_else(|| GitIgnoreError::NotInDir {
                dir_path: self.dir_path.clone(),
                path: path.components().collect(),
            })
    }

    fn is_outdated(&self) -> bool {
        Instant::now() - self.last_refreshed_ignored_paths_at
            > GitIgnore::REFRESH_IGNORED_PATHS_AFTER
    }

    fn new_outdated(dir_path: AbsPathBuf) -> Self {
        let outdated_time = Instant::now()
            - GitIgnore::REFRESH_IGNORED_PATHS_AFTER
            - Duration::from_secs(1);

        Self {
            dir_path,
            ignored_paths: Default::default(),
            last_refreshed_ignored_paths_at: outdated_time,
        }
    }

    fn refresh(&mut self) -> Result<(), GitIgnoreError> {
        let output =
            match GitIgnore::command().current_dir(&self.dir_path).output() {
                Ok(out) => out,

                // The `git` executable is not in `$PATH`. This probably means
                // the user is not using Git, which probably means the
                // directory is not in a Git repository.
                Err(err) if err.kind() == io::ErrorKind::NotFound => {
                    return Ok(());
                },

                Err(err) => return Err(GitIgnoreError::GitCommand(err)),
            };

        if !output.status.success() {
            // TODO: check if the reason is because the directory is not in a
            // Git repository.
            return Err(GitIgnoreError::FailedCommand(output.status));
        }

        let stdout = str::from_utf8(&output.stdout)
            .map_err(|_| GitIgnoreError::StdoutNotUtf8)?;

        self.ignored_paths.clear();

        for line in stdout.lines() {
            if let Err(err) = self.ignored_paths.insert(line) {
                self.ignored_paths.clear();
                return Err(err);
            }
        }

        self.last_refreshed_ignored_paths_at = Instant::now();

        Ok(())
    }
}

impl IgnoredPaths {
    fn clear(&mut self) {
        self.inner.clear();
    }

    fn contains<'a>(&self, path: impl Iterator<Item = &'a NodeName>) -> bool {
        let mut cursor = Cursor::new(self);
        for component in path {
            if let Some(SeekResult::FoundAt(_)) = cursor.seek(component) {
                return true;
            }
        }
        false
    }

    fn insert(&mut self, path: &str) -> Result<(), GitIgnoreError> {
        let mut cursor = Cursor::new(self);
        let mut components = path.split(MAIN_SEPARATOR);

        let idx = loop {
            let Some(component) = components.next() else {
                break cursor.insert_at();
            };

            let component =
                <&NodeName>::try_from(component).map_err(|err| {
                    GitIgnoreError::NotNodeName {
                        node_name: component.to_owned(),
                        err,
                    }
                })?;

            if let Some(status) = cursor.seek(component) {
                match status {
                    SeekResult::FoundAt(_) => return Ok(()),
                    SeekResult::InsertAt(idx) => break idx,
                }
            }
        };

        self.inner.insert(idx, path.into());

        Ok(())
    }
}

struct Cursor<'a> {
    _paths: &'a [CompactString],
    insert_at: Option<usize>,
}

enum SeekResult {
    /// TODO: docs.
    FoundAt(usize),

    /// TODO: docs.
    InsertAt(usize),
}

impl<'a> Cursor<'a> {
    #[track_caller]
    fn insert_at(&self) -> usize {
        match self.insert_at {
            Some(idx) => idx,
            None => panic!("Cursor::seek was never called"),
        }
    }

    fn new(paths: &'a IgnoredPaths) -> Self {
        Self { _paths: paths.inner.as_slice(), insert_at: None }
    }

    fn seek(&mut self, _node_name: &NodeName) -> Option<SeekResult> {
        todo!()
    }
}

// We're shelling out to Git to get the list of ignored files, so this can only
// be a filter on a real filesystem.
impl Filter<os::OsFs> for GitIgnore {
    type Error =
        Either<<DirEntry<os::OsFs> as Metadata>::NameError, GitIgnoreError>;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        entry: &DirEntry<os::OsFs>,
    ) -> Result<bool, Self::Error> {
        struct Concat<'a>(&'a AbsPath, &'a NodeName);

        impl Path for Concat<'_> {
            fn components(&self) -> impl Iterator<Item = &NodeName> + '_ {
                let &Self(parent, name) = self;
                parent.components().chain(iter::once(name))
            }
            fn strip_prefix(&self, s: &AbsPath) -> Option<Self> {
                let &Self(parent, name) = self;
                parent.strip_prefix(s).map(|parent| Self(parent, name))
            }
        }

        let entry_name = entry.name().await.map_err(Either::Left)?;
        let path = Concat(dir_path, &entry_name);
        self.with_inner(|inner| inner.is_ignored(path))
            .map_err(Either::Right)?
            .map_err(Either::Right)
    }
}
