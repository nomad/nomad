use std::collections::VecDeque;
use std::io::{self, BufRead, Read, Write};
use std::process;
use std::sync::{Arc, OnceLock};

use abs_path::{AbsPath, AbsPathBuf};
use ed::executor::{BackgroundSpawner, Task};
use ed::fs::{self, os};
use futures_util::{StreamExt, select_biased};

use crate::Filter;

/// A [`Filter`] that filters out nodes based on the various exclusion rules
/// used by Git.
#[derive(Clone)]
pub struct GitIgnore {
    /// A sender used to send [`CheckRequest`]s to the background task.
    request_tx: flume::Sender<CheckRequest>,

    /// The exit status of the `git check-ignore` process, if it has exited.
    exit_status: Arc<OnceLock<io::Result<process::ExitStatus>>>,
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

    /// The `git check-ignore` process has exited.
    #[display(
        "the 'git check-ignore ..' process has exited{}",
        _0.map_or(Default::default(), |status| format!(" with status {status}"))
    )]
    ProcessExited(Option<process::ExitStatus>),
}

/// A request to check if a path is ignored, together with a sender that the
/// background task can use to send the result back.
struct CheckRequest {
    node_path: AbsPathBuf,
    result_tx: flume::Sender<Result<bool, GitIgnoreFilterError>>,
}

enum Message {
    /// Sent by the stdout task when a new line is read. The `bool` indicates
    /// whether the path (which is not included in the message) is ignored.
    FromStdout(bool),

    /// Send by the stderr task when an error occurs.
    FromStderr(GitIgnoreFilterError),
}

impl GitIgnore {
    /// Creates a new `GitIgnore` filter.
    pub fn new(
        repo_path: &AbsPath,
        bg_spawner: &mut impl BackgroundSpawner,
    ) -> Result<Self, GitIgnoreCreateError> {
        let mut child = Self::command()
            .current_dir(repo_path)
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
            .map_err(GitIgnoreCreateError::CommandFailed)?;

        let stdin = child.stdin.take().expect("stdin handle present");
        let stdout = child.stdout.take().expect("stdout handle present");
        let stderr = child.stderr.take().expect("stderr handle present");

        let exit_status = Arc::new(OnceLock::new());
        let (request_tx, request_rx) = flume::unbounded();
        let (message_tx, message_rx) = flume::unbounded();

        bg_spawner
            .spawn({
                let exit_status = exit_status.clone();
                async move {
                    Self::event_loop(
                        child,
                        stdin,
                        request_rx,
                        message_rx,
                        exit_status,
                    )
                    .await;
                }
            })
            .detach();

        bg_spawner
            .spawn({
                let message_tx = message_tx.clone();
                async move { Self::read_from_stdout(stdout, message_tx) }
            })
            .detach();

        bg_spawner
            .spawn({
                let message_tx = message_tx.clone();
                async move { Self::read_from_stderr(stderr, message_tx) }
            })
            .detach();

        Ok(Self { request_tx, exit_status })
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

    async fn event_loop(
        mut child: process::Child,
        mut stdin: process::ChildStdin,
        request_rx: flume::Receiver<CheckRequest>,
        message_rx: flume::Receiver<Message>,
        exit_status: Arc<OnceLock<io::Result<process::ExitStatus>>>,
    ) {
        let mut request_stream = request_rx.into_stream();
        let mut message_stream = message_rx.into_stream();
        let mut result_tx_queue = VecDeque::new();

        loop {
            select_biased! {
                request = request_stream.select_next_some() => {
                    let path = request.node_path.as_str().as_bytes();

                    let write_res = stdin
                        .write_all(path)
                        .and_then(|()| stdin.write_all(b"\0"));

                    match write_res {
                        Ok(()) => {
                            result_tx_queue.push_front(request.result_tx)
                        },
                        Err(_) => {
                            // Just give up if we can't write to stdin.
                            break
                        },
                    }
                },
                message = message_stream.select_next_some() => {
                    // We can always pop from the front of the queue because
                    // 'git check-ignore' outputs paths in the same order they
                    // were sent to stdin.
                    let result_tx = result_tx_queue
                        .pop_back()
                        .expect("the queue should not be empty");

                    let result = match message {
                        Message::FromStdout(is_ignored) => Ok(is_ignored),
                        Message::FromStderr(err) => Err(err),
                    };

                    // The receiver might've been dropped, and that's ok.
                    let _ = result_tx.send(result);
                },
                complete => break,
            }
        }

        drop(stdin);

        match exit_status.set(child.wait()) {
            Ok(()) => (),
            Err(_) => unreachable!("exit status only set once"),
        }

        for result_tx in result_tx_queue {
            let _ = result_tx.send(Err(GitIgnoreFilterError::ProcessExited(
                exit_status.get().expect("just set it").as_ref().ok().copied(),
            )));
        }
    }

    /// Continuosly reads from the `stdout` of the `git check-ignore` process
    /// until it hits EOF or an error occurs.
    fn read_from_stdout(
        mut stdout: process::ChildStdout,
        message_tx: flume::Sender<Message>,
    ) {
        /// See https://git-scm.com/docs/git-check-ignore#_output for more
        /// infos on what each variant represents.
        enum ReadingState {
            Source,
            Linenum,
            Pattern,
            Pathname,
        }

        let mut state = ReadingState::Source;
        let mut buf = Vec::new();
        let mut is_ignored = false;

        loop {
            buf.clear();

            match stdout.read_to_end(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(_non_zero) => (),
            }

            let mut buf = &buf[..];

            while let Some(split_idx) = buf.iter().position(|&b| b == 0) {
                buf = &buf[split_idx + 1..];

                match state {
                    ReadingState::Source => {
                        is_ignored = split_idx == 0;
                        state = ReadingState::Linenum;
                    },
                    ReadingState::Linenum => state = ReadingState::Pattern,
                    ReadingState::Pattern => state = ReadingState::Pathname,
                    ReadingState::Pathname => {
                        state = ReadingState::Source;
                        message_tx
                            .send(Message::FromStdout(is_ignored))
                            .expect("event loop is still running");
                        is_ignored = false;
                    },
                }
            }
        }
    }

    /// Continuosly reads from the `stderr` of the `git check-ignore` process
    /// until it hits EOF or an error occurs.
    fn read_from_stderr(
        stderr: process::ChildStderr,
        message_tx: flume::Sender<Message>,
    ) {
        let mut reader = io::BufReader::new(stderr);
        let mut line = String::new();

        loop {
            line.clear();

            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_non_zero) => (),
            }

            if let Some(err) = GitIgnoreFilterError::parse_stderr_line(&line) {
                message_tx
                    .send(Message::FromStderr(err))
                    .expect("event loop is still running");
            }
        }
    }
}

impl GitIgnoreFilterError {
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

    fn parse_stderr_line(line: &str) -> Option<Self> {
        Self::parse_path_does_not_exist(line)
            .or_else(|| Self::parse_path_outside_repo(line))
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
        loop {
            if let Some(exit_status) = self.exit_status.get() {
                return Err(GitIgnoreFilterError::ProcessExited(
                    exit_status.as_ref().ok().cloned(),
                ));
            }

            let node_name =
                node_meta.name().map_err(GitIgnoreFilterError::NodeName)?;

            let (result_tx, result_rx) = flume::bounded(1);

            let request = CheckRequest {
                node_path: dir_path.join(node_name),
                result_tx,
            };

            if self.request_tx.send(request).is_err() {
                // The background task just completed. Loop again, there will
                // be an exit status set.
                continue;
            }

            match result_rx.recv_async().await {
                Ok(result) => return result,
                // The background task just completed. Loop again, there will
                // be an exit status set.
                Err(_recv_err) => continue,
            }
        }
    }
}
