use std::process;

use abs_path::AbsPathBuf;

/// The type of error that can occur when using the [`GitIgnore`] filter.
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq)]
pub enum IgnoreError {
    /// The given path does not exist.
    #[display("the path {_0:?} does not exist")]
    PathDoesNotExist(AbsPathBuf),

    /// The path is outside the repository whose path was given to
    /// [`GitIgnore::new`].
    #[display("the path {path:?} is outside the repository at {repo_path:?}")]
    PathOutsideRepo {
        /// The path that is outside the repository.
        path: AbsPathBuf,

        /// The repo's path.
        repo_path: AbsPathBuf,
    },

    /// The `git check-ignore` process has exited.
    #[display(
        "the 'git check-ignore ..' process has exited{}",
        _0.map_or(Default::default(), |status| format!(" with status {status}"))
    )]
    ProcessExited(Option<process::ExitStatus>),
}

impl IgnoreError {
    pub(crate) fn parse_stderr_line(line: &str) -> Option<Self> {
        let line = line.trim_end();
        Self::parse_path_does_not_exist(line)
            .or_else(|| Self::parse_path_outside_repo(line))
    }

    fn parse_path_does_not_exist(line: &str) -> Option<Self> {
        line.strip_prefix("fatal: Invalid path '")
            .and_then(|rest| rest.strip_suffix("': No such file or directory"))
            .and_then(|path| path.parse::<AbsPathBuf>().ok())
            .map(Self::PathDoesNotExist)
    }

    fn parse_path_outside_repo(line: &str) -> Option<Self> {
        let (left, right) = line.split_once("' is outside repository at '")?;
        let (_, path) = left.split_once(": '")?;
        let repo_path = right.strip_suffix('\'')?;
        Some(Self::PathOutsideRepo {
            path: path.parse().ok()?,
            repo_path: repo_path.parse().ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use abs_path::path;

    use super::*;

    #[test]
    fn parse_stderr_1() {
        let stderr =
            "fatal: /foo: '/foo' is outside repository at '/foo/bar'\n";
        let err = IgnoreError::parse_stderr_line(stderr).unwrap();
        assert_eq!(
            err,
            IgnoreError::PathOutsideRepo {
                path: path!("/foo").to_owned(),
                repo_path: path!("/foo/bar").to_owned(),
            }
        )
    }

    #[test]
    fn parse_stderr_2() {
        let stderr =
            "fatal: Invalid path '/foo/bar': No such file or directory\n";
        let err = IgnoreError::parse_stderr_line(stderr).unwrap();
        assert_eq!(
            err,
            IgnoreError::PathDoesNotExist(path!("/foo/bar").to_owned())
        )
    }
}
