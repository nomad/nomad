/// The type of error that can occur when creating a new [`GitIgnore`].
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq)]
pub enum CreateError {
    /// Shelling out to `git` failed.
    #[display("{_0}")]
    CommandFailed(crate::CommandError),

    /// The `git` executable is not in the user's `$PATH`.
    #[display("the 'git' executable is not in $PATH")]
    GitNotInPath,

    /// The path given to [`GitIgnore::new`] doesn't exist.
    #[display("the path does not exist")]
    InvalidPath,

    /// The path given to [`GitIgnore::new`] doesn't point to a Git repository.
    #[display("the path does not point to a Git repository")]
    PathNotInGitRepository,
}
