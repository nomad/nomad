use core::ops::Range;
use core::time::Duration;
use core::{iter, str};
use std::io;
use std::path::MAIN_SEPARATOR;
use std::process::{Command, ExitStatus};
use std::sync::Mutex;
use std::time::Instant;

use compact_str::CompactString;
use ed::fs::{
    self,
    AbsPath,
    AbsPathBuf,
    Metadata,
    MetadataNameError,
    NodeName,
    os,
};

use crate::{Either, Filter};

/// TODO: docs.
pub struct GitIgnore {
    inner: Mutex<GitIgnoreInner>,
}

/// TODO: docs.
#[derive(Debug, derive_more::Display, cauchy::Error)]
pub enum GitIgnoreError {
    #[display(
        "Running {cmd:?} failed with exit code {_0:?}",
        cmd = GitIgnore::command()
    )]
    FailedCommand(ExitStatus),

    #[display("Running {cmd:?} failed: {_0}", cmd = GitIgnore::command())]
    GitCommand(io::Error),

    #[display("{node_name:?} is not a valid node name: {err:?}")]
    NotNodeName { node_name: String, err: fs::InvalidNodeNameError },

    #[display("{path:?} is not in {dir_path:?}")]
    NotInDir { path: AbsPathBuf, dir_path: AbsPathBuf },

    #[display(
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
            if let Some(result) = cursor.seek(component) {
                return matches!(result, SeekResult::FoundAt(_));
            }
        }
        false
    }

    fn insert(&mut self, path: &str) -> Result<(), GitIgnoreError> {
        assert!(!path.is_empty());

        let path = path.trim_matches(MAIN_SEPARATOR);
        let mut cursor = Cursor::new(self);
        let mut components = path.split(MAIN_SEPARATOR);

        let idx = loop {
            let Some(component) = components.next() else {
                let range = cursor.matched_range().expect(
                    "path is not empty, so Cursor::seek() must've been \
                     called at least once",
                );
                self.inner.splice(range, iter::once(path.into()));
                return Ok(());
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
    paths: &'a [CompactString],

    /// The index of the first element of `paths` in the original slice
    /// constructed in [`Cursor::new`], or `None` if [`Cursor::seek`] has never
    /// been called.
    start_idx: Option<usize>,

    /// The number of leading bytes in each element of `paths` that have
    /// already been matched by previous calls to [`Cursor::seek`].
    num_bytes_already_matched: usize,
}

enum SeekResult {
    /// TODO: docs.
    FoundAt(#[allow(dead_code)] usize),

    /// TODO: docs.
    InsertAt(usize),
}

impl<'a> Cursor<'a> {
    fn first_component(&self, path: &'a str) -> &'a str {
        path[self.num_bytes_already_matched..]
            .split(MAIN_SEPARATOR)
            .next()
            .expect("path is not empty")
    }

    fn matched_range(&self) -> Option<Range<usize>> {
        self.start_idx.map(|start_idx| start_idx..start_idx + self.paths.len())
    }

    fn new(paths: &'a IgnoredPaths) -> Self {
        Self {
            paths: paths.inner.as_slice(),
            start_idx: None,
            num_bytes_already_matched: 0,
        }
    }

    fn seek(&mut self, node_name: &NodeName) -> Option<SeekResult> {
        // Look for the index of the 1st path whose first component matches the
        // node_name.
        let idx_match = match self
            .paths
            .binary_search_by(|path| self.first_component(path).cmp(node_name))
        {
            Ok(idx) => idx,
            Err(idx) => {
                return Some(SeekResult::InsertAt(
                    self.start_idx.unwrap_or(0) + idx,
                ));
            },
        };

        // Binary search may not return the first match when multiple matches
        // exist.
        //
        // Example: with paths ["a/a", "a/b", "a/c"] and node_name "a", the
        // search might return index 1, but the first match is at index 0.

        let num_matches_backward = self.paths[..idx_match]
            .iter()
            .rev()
            .take_while(|path| self.first_component(path) == node_name)
            .count();

        let num_matches_forward = self.paths[idx_match + 1..]
            .iter()
            .take_while(|path| self.first_component(path) == node_name)
            .count();

        let idx_first_match = idx_match - num_matches_backward;
        let idx_last_match = idx_match + num_matches_forward;

        let new_start_idx = self.start_idx.unwrap_or(0) + idx_first_match;
        self.start_idx = Some(new_start_idx);
        self.paths = &self.paths[idx_first_match..idx_last_match + 1];

        if let [path] = self.paths {
            if &path[self.num_bytes_already_matched..] == node_name {
                return Some(SeekResult::FoundAt(new_start_idx));
            }
        }

        self.num_bytes_already_matched +=
            node_name.len() + MAIN_SEPARATOR.len_utf8();

        None
    }
}

// We're shelling out to Git to get the list of ignored files, so this can only
// be a filter on a real filesystem.
impl Filter<os::OsFs> for GitIgnore {
    type Error = Either<MetadataNameError, GitIgnoreError>;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl Metadata<Fs = os::OsFs>,
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

        let node_name = node_meta.name().map_err(Either::Left)?;
        let path = Concat(dir_path, node_name);
        self.with_inner(|inner| inner.is_ignored(path))
            .map_err(Either::Right)?
            .map_err(Either::Right)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn ignored_paths_1() {
        let paths = IgnoredPaths::from_lines(["a/", "a/foo.txt"]);
        assert!(paths.contains_str("a"));
        assert!(paths.contains_str("a/bar.txt"));
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn ignored_paths_2() {
        let paths = IgnoredPaths::from_lines(["a/foo.txt", "a/bar.txt"]);
        assert!(!paths.contains_str("a"));
        assert!(!paths.contains_str("a/baz.txt"));
        assert!(paths.contains_str("a/foo.txt"));
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn ignored_paths_3() {
        let paths = IgnoredPaths::from_lines(["a/b/foo.txt", "a/b/c/foo.txt"]);
        assert!(!paths.contains_str("a/b/"));
        assert!(!paths.contains_str("a/b/bar.txt"));
        assert!(paths.contains_str("a/b/foo.txt"));
        assert!(paths.contains_str("a/b/c/foo.txt"));
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn ignored_paths_4() {
        let paths = IgnoredPaths::from_lines(["abc"]);
        assert!(!paths.contains_str("a"));
        assert!(!paths.contains_str("ab"));
        assert!(!paths.contains_str("a/bc"));
        assert!(!paths.contains_str("ab/c"));
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn ignored_paths_5() {
        let mut array = ["a/foo.txt", "a/bar.txt", "a/baz.txt"];
        let paths = IgnoredPaths::from_lines(array);
        array.sort();
        assert_eq!(paths.inner, array);
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn ignored_paths_6() {
        let paths = IgnoredPaths::from_lines([
            "a/foo.txt",
            "a/bar.txt",
            "a/baz.txt",
            "a/",
        ]);
        assert_eq!(paths.inner, ["a"]);
    }

    #[test]
    #[cfg_attr(target_os = "windows", ignore)]
    fn ignored_paths_7() {
        let paths = IgnoredPaths::from_lines(["a/foo.txt", "a/foo.txt/"]);
        assert_eq!(paths.inner, ["a/foo.txt"]);
    }

    impl IgnoredPaths {
        fn contains_str(&self, s: &str) -> bool {
            self.contains(
                s.trim_matches(MAIN_SEPARATOR).split(MAIN_SEPARATOR).map(
                    |component| <&NodeName>::try_from(component).unwrap(),
                ),
            )
        }

        fn from_lines<'a>(lines: impl IntoIterator<Item = &'a str>) -> Self {
            let mut this = Self::default();
            for line in lines {
                this.insert(line).unwrap();
            }
            this
        }
    }
}
