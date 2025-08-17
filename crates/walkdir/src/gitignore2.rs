use std::{io, process};

use abs_path::{AbsPath, AbsPathBuf};
use ed::executor::BackgroundSpawner;
use ed::fs::os;
use ed::{Editor, fs};

use crate::Filter;

/// A [`Filter`] that filters out nodes based on the various exclusion rules
/// used by Git.
#[derive(Clone)]
pub struct GitIgnore {
    /// A sender used to send messages to the background task.
    message_tx: flume::Sender<Message>,
}

/// The type of error that can occur when creating the [`GitIgnore`] filter.
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
pub enum GitIgnoreCreateError {
    /// The path given to [`GitIgnore::new`] doesn't point to a Git repository.
    #[display("the path {_0:?} does not point to a Git repository")]
    InvalidRepoPath(AbsPathBuf),

    /// Running the `git check-ignore` command failed.
    #[display("Running {cmd:?} failed: {_0}", cmd = GitIgnore::command())]
    CommandFailed(io::Error),
}

/// The type of error that can occur when using the [`GitIgnore`] filter.
#[derive(cauchy::Debug, derive_more::Display, cauchy::Error)]
pub enum GitIgnoreFilterError {
    /// The name corresponding to a node's metadata could not be obtained.
    #[display("{_0}")]
    NodeName(fs::MetadataNameError),

    /// The path given to [`GitIgnore::should_filter`] does not exist.
    #[display("the path {_0:?} does not exist")]
    PathDoesNotExist(AbsPathBuf),

    /// The path is outside the repository.
    #[display("the path {path:?} is outside the repository at {repo_path:?}")]
    PathOutsideRepo {
        /// The path given to [`GitIgnore::should_filter`].
        path: AbsPathBuf,

        /// The repo's path.
        repo_path: AbsPathBuf,
    },
}

enum Message {
    /// A request to check if a path is ignored, together with a sender that
    /// the background task can use to send the result back.
    CheckIgnore {
        node_path: AbsPathBuf,
        result_tx: flume::Sender<Result<bool, GitIgnoreFilterError>>,
    },

    /// All instances of the [`GitIgnore`] filter have been dropped, so the
    /// background task can be stopped.
    Stop,
}

impl GitIgnore {
    /// Creates a new `GitIgnore` filter.
    pub async fn new<Ed>(
        _repo_path: AbsPathBuf,
        _bg_spawner: &mut impl BackgroundSpawner,
    ) -> Result<Self, GitIgnoreCreateError>
    where
        Ed: Editor<Fs = os::OsFs>,
    {
        todo!();
    }

    fn command() -> process::Command {
        let mut cmd = process::Command::new("git");

        // See https://git-scm.com/docs/git-check-ignore#_options for more
        // infos on the options used here.
        cmd.arg("check-ignore")
            .arg("--stdin")
            .arg("--non-matching")
            .arg("--verbose")
            .arg("-z");

        cmd
    }
}

// We're shelling out to Git to get the list of ignored files, so this can only
// be a filter on a real filesystem.
impl Filter<os::OsFs> for GitIgnore {
    type Error = GitIgnoreFilterError;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl fs::Metadata<Fs = os::OsFs>,
    ) -> Result<bool, Self::Error> {
        let node_name =
            node_meta.name().map_err(GitIgnoreFilterError::NodeName)?;

        let (result_tx, result_rx) = flume::bounded(1);

        let message = Message::CheckIgnore {
            node_path: dir_path.join(node_name),
            result_tx,
        };

        self.message_tx
            .send(message)
            .expect("background task hasn't been stopped");

        result_rx
            .recv_async()
            .await
            .expect("background task hasn't been stopped")
    }
}
