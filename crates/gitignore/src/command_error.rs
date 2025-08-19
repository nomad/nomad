use core::fmt;
use std::io;

use crate::GitIgnore;

/// The type of error that can occur when shelling out to `git` while creating
/// a new [`GitIgnore`].
#[derive(derive_more::Display, cauchy::Error)]
#[display(
    "running {cmd:?} failed: {inner}",
    cmd = if self.failed_checking_if_in_git_repo {
        GitIgnore::is_in_repo_command()
    } else {
        GitIgnore::check_ignore_command()
    },
)]
pub struct CommandError {
    pub(crate) inner: io::Error,
    pub(crate) failed_checking_if_in_git_repo: bool,
}

impl fmt::Debug for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl PartialEq for CommandError {
    fn eq(&self, other: &Self) -> bool {
        self.failed_checking_if_in_git_repo
            == other.failed_checking_if_in_git_repo
            && self.inner.kind() == other.inner.kind()
            && self.inner.to_string() == other.inner.to_string()
    }
}
