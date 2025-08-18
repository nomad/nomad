use core::{fmt, mem};
use std::collections::VecDeque;
use std::io::{self, BufRead, Write};
use std::process;
use std::sync::{Arc, OnceLock};

use abs_path::{AbsPath, AbsPathBuf};
use ed::executor::{BackgroundSpawner, Task};
use ed::fs::{self, os};
use either::Either;

use crate::Filter;

/// A [`Filter`] that filters out nodes based on the various exclusion rules
/// used by Git.
#[derive(Clone, cauchy::Debug)]
pub struct GitIgnore {
    /// A sender used to send [`Message`]s to the background task.
    #[debug(skip)]
    message_tx: flume::Sender<Message>,

    /// The exit status of the `git check-ignore` process, if it has exited.
    #[debug(skip)]
    exit_status: Arc<OnceLock<io::Result<process::ExitStatus>>>,

    /// The ID of the `git check-ignore` process.
    #[debug(skip)]
    process_id: u32,
}

/// The type of error that can occur when creating the [`GitIgnore`] filter.
#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq)]
pub enum CreateError {
    /// Shelling out to `git` failed.
    #[display("{_0}")]
    CommandFailed(CommandError),

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

/// The type of error returned when shelling out to `git` while creating a
/// [`GitIgnore`] fails.
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
    inner: io::Error,
    failed_checking_if_in_git_repo: bool,
}

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

#[derive(Debug)]
enum Message {
    /// A request to check if a path is ignored, together with a sender that
    /// the background task can use to send the result back.
    CheckRequest {
        path: AbsPathBuf,
        result_tx: flume::Sender<Result<bool, IgnoreError>>,
    },

    /// `Ok` variants are sent by the stdout task when a new output is read.
    /// The boolean indicates whether the path it read is ignored.
    ///
    /// `Err` variants are sent by the stderr task when a new line is read
    /// that was successfully parsed into a [`GitIgnoreFilterError`].
    ReadIgnoreResult(Result<bool, IgnoreError>),

    /// Sent by the stdout task when it has reached EOF or if an error occured
    /// while reading.
    StdoutClosed,

    /// Sent by the stderr task when it has reached EOF or if an error occured
    /// while reading.
    StderrClosed,

    /// Sent when dropping the last `GitIgnore` instance.
    TerminateProcess,
}

#[derive(Debug, Default)]
struct StdoutParser {
    is_ignored: bool,
    state: StdoutReadState,
}

/// See https://git-scm.com/docs/git-check-ignore#_output for more infos on
/// what each variant represents.
#[derive(Debug, Default)]
enum StdoutReadState {
    #[default]
    Source,
    Linenum,
    Pattern,
    Pathname,
}

impl GitIgnore {
    /// Checks if the given path is ignored by Git.
    pub async fn is_ignored(
        &self,
        path: impl Into<AbsPathBuf>,
    ) -> Result<bool, IgnoreError> {
        if let Some(exit_status) = self.exit_status.get() {
            return Err(IgnoreError::ProcessExited(
                exit_status.as_ref().ok().cloned(),
            ));
        }

        let (result_tx, result_rx) = flume::bounded(1);

        let message = Message::CheckRequest { path: path.into(), result_tx };

        if self.message_tx.send(message).is_err() {
            let exit_status = self.exit_status.get().expect(
                "event loop has stopped running, so the exit status must've \
                 been set",
            );
            return Err(IgnoreError::ProcessExited(
                exit_status.as_ref().ok().cloned(),
            ));
        }

        result_rx.recv_async().await.expect(
            "message has been sent successfully, so we'll get a response",
        )
    }

    /// Creates a new `GitIgnore` filter.
    pub fn new(
        repo_path: &AbsPath,
        bg_spawner: &mut impl BackgroundSpawner,
    ) -> Result<Self, CreateError> {
        let is_in_git_repo = Self::is_in_repo_command()
            .current_dir(repo_path)
            .stdout(process::Stdio::null())
            .stderr(process::Stdio::null())
            .status()
            .map_err(|io_err| match io_err.kind() {
                io::ErrorKind::NotFound => {
                    // The command not being found and the path not existing
                    // both result in the same error (at least on macOS), so
                    // check if the path exists to distinguish between the two.
                    if std::fs::metadata(repo_path).is_ok() {
                        CreateError::GitNotInPath
                    } else {
                        CreateError::InvalidPath
                    }
                },
                _ => CreateError::CommandFailed(CommandError {
                    inner: io_err,
                    failed_checking_if_in_git_repo: true,
                }),
            })?
            .success();

        if !is_in_git_repo {
            return Err(CreateError::PathNotInGitRepository);
        }

        let mut child = Self::check_ignore_command()
            .current_dir(repo_path)
            .stdin(process::Stdio::piped())
            .stdout(process::Stdio::piped())
            .stderr(process::Stdio::piped())
            .spawn()
            .map_err(|io_err| CommandError {
                failed_checking_if_in_git_repo: false,
                inner: io_err,
            })
            .map_err(CreateError::CommandFailed)?;

        let process_id = child.id();
        let stdin = child.stdin.take().expect("stdin handle present");
        let stdout = child.stdout.take().expect("stdout handle present");
        let stderr = child.stderr.take().expect("stderr handle present");

        let exit_status = Arc::new(OnceLock::new());
        let (message_tx, message_rx) = flume::unbounded();

        bg_spawner
            .spawn({
                let exit_status = exit_status.clone();
                async move {
                    Self::event_loop(child, stdin, message_rx, exit_status)
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

        Ok(Self { message_tx, exit_status, process_id })
    }

    /// Returns the ID of the `git check-ignore` process, or an error if the
    /// process has exited (together with its exit status if we could get it).
    pub fn process_id(&self) -> Result<u32, Option<process::ExitStatus>> {
        self.exit_status
            .get()
            .map(|status| status.as_ref().ok().cloned())
            .map_or_else(|| Ok(self.process_id), Err)
    }

    fn check_ignore_command() -> process::Command {
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
        message_rx: flume::Receiver<Message>,
        exit_status: Arc<OnceLock<io::Result<process::ExitStatus>>>,
    ) {
        let mut result_tx_queue = VecDeque::new();
        let mut is_stdout_closed = false;
        let mut is_stderr_closed = false;

        while let Ok(message) = message_rx.recv_async().await {
            match message {
                Message::CheckRequest { path, result_tx } => {
                    result_tx_queue.push_front(result_tx);

                    let write_res = stdin
                        .write_all(path.as_bytes())
                        .and_then(|()| stdin.write_all(b"\0"));

                    // Just give up if we can't write to stdin.
                    if write_res.is_err() {
                        break;
                    }
                },

                Message::ReadIgnoreResult(result) => {
                    // We can always pop from the front of the queue because
                    // 'git check-ignore' outputs paths in the same order they
                    // were sent to stdin.
                    let result_tx = result_tx_queue
                        .pop_back()
                        .expect("the queue should not be empty");

                    // The receiver might've been dropped, and that's ok.
                    let _ = result_tx.send(result);
                },

                // Stop the event loop when both stdout and stderr are closed.
                Message::StdoutClosed if is_stderr_closed => break,
                Message::StderrClosed if is_stdout_closed => break,
                Message::StdoutClosed => is_stdout_closed = true,
                Message::StderrClosed => is_stderr_closed = true,

                Message::TerminateProcess => {
                    // NOTE: sending SIGKILL only marks the child as defunct,
                    // but we need to reap it with 'wait()' to avoid a zombie
                    // process.
                    drop(stdin);
                    let _ = child.kill();
                    let _ = child.wait();
                    return;
                },
            }
        }

        drop(stdin);

        assert!(
            exit_status.set(child.wait()).is_ok(),
            "exit status only set once"
        );

        let maybe_status =
            exit_status.get().expect("just set it").as_ref().ok().copied();

        let result_txs = result_tx_queue.into_iter().chain(
            message_rx.into_iter().filter_map(|msg| {
                if let Message::CheckRequest { result_tx, .. } = msg {
                    Some(result_tx)
                } else {
                    None
                }
            }),
        );

        for result_tx in result_txs {
            let _ =
                result_tx.send(Err(IgnoreError::ProcessExited(maybe_status)));
        }
    }

    fn is_in_repo_command() -> process::Command {
        let mut cmd = process::Command::new("git");
        cmd.arg("rev-parse").arg("--is-inside-work-tree");
        cmd
    }

    /// Returns the number of instances of this `GitIgnore` filter.
    fn num_instances(&self) -> usize {
        let is_event_loop_running = self.exit_status.get().is_none();
        Arc::strong_count(&self.exit_status) - is_event_loop_running as usize
    }

    /// Continuosly reads from the `stdout` of the `git check-ignore` process
    /// until it hits EOF or an error occurs.
    fn read_from_stdout(
        stdout: process::ChildStdout,
        message_tx: flume::Sender<Message>,
    ) {
        let mut reader = io::BufReader::new(stdout);
        let mut parser = StdoutParser::default();

        loop {
            let mut buf = match reader.fill_buf() {
                Ok(buf) if buf.is_empty() => break,
                Ok(buf) => buf,
                Err(_err) => break,
            };

            let buf_len = buf.len();

            while let Some((is_ignored, new_buf)) = parser.feed(buf) {
                buf = new_buf;
                let message = Message::ReadIgnoreResult(Ok(is_ignored));
                let _ = message_tx.send(message);
            }

            reader.consume(buf_len);
        }

        let _ = message_tx.send(Message::StdoutClosed);
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
                Ok(0) | Err(_) => break,
                Ok(_non_zero) => (),
            }

            if let Some(err) = IgnoreError::parse_stderr_line(&line) {
                let message = Message::ReadIgnoreResult(Err(err));
                let _ = message_tx.send(message);
            }
        }

        let _ = message_tx.send(Message::StderrClosed);
    }
}

impl IgnoreError {
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
        let line = line.trim_end();
        Self::parse_path_does_not_exist(line)
            .or_else(|| Self::parse_path_outside_repo(line))
    }
}

impl StdoutParser {
    fn feed<'buf>(
        &mut self,
        mut bytes: &'buf [u8],
    ) -> Option<(bool, &'buf [u8])> {
        while let Some(nul_idx) = bytes.iter().position(|&b| b == 0) {
            bytes = &bytes[nul_idx + 1..];

            match self.state {
                StdoutReadState::Source => {
                    self.is_ignored = nul_idx > 0;
                    self.state = StdoutReadState::Linenum;
                },
                StdoutReadState::Linenum => {
                    self.state = StdoutReadState::Pattern;
                },
                StdoutReadState::Pattern => {
                    self.state = StdoutReadState::Pathname;
                },
                StdoutReadState::Pathname => {
                    self.state = StdoutReadState::Source;
                    let is_ignored = mem::take(&mut self.is_ignored);
                    return Some((is_ignored, bytes));
                },
            }
        }

        None
    }
}

impl Drop for GitIgnore {
    fn drop(&mut self) {
        if self.num_instances() == 1 {
            let _ = self.message_tx.send(Message::TerminateProcess);
        }
    }
}

// We're shelling out to Git, so this can only be a filter on a real
// filesystem.
impl Filter<os::OsFs> for GitIgnore {
    type Error = Either<fs::MetadataNameError, IgnoreError>;

    async fn should_filter(
        &self,
        dir_path: &AbsPath,
        node_meta: &impl fs::Metadata<Fs = os::OsFs>,
    ) -> Result<bool, Self::Error> {
        let node_name = node_meta.name().map_err(Either::Left)?;
        let node_path = dir_path.join(node_name);
        self.is_ignored(node_path).await.map_err(Either::Right)
    }
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

#[cfg(test)]
mod tests {
    use abs_path::path;

    use super::*;

    #[test]
    fn parse_stdout_1() {
        let stdout = b"source\042\0pattern\0pathname\0";
        let mut parser = StdoutParser::default();
        let (is_ignored, rest) = parser.feed(stdout).unwrap();
        assert!(is_ignored);
        assert!(rest.is_empty());
    }

    #[test]
    fn parse_stdout_2() {
        let stdout = b"\0\0\0pathname\0";
        let mut parser = StdoutParser::default();
        let (is_ignored, rest) = parser.feed(stdout).unwrap();
        assert!(!is_ignored);
        assert!(rest.is_empty());
    }

    #[test]
    fn parse_stdout_3() {
        let mut parser = StdoutParser::default();
        assert!(parser.feed(b"source\0").is_none());
        assert!(parser.feed(b"42\0").is_none());
        assert!(parser.feed(b"pattern\0").is_none());
        let (is_ignored, rest) = parser.feed(b"pathname\0").unwrap();
        assert!(is_ignored);
        assert!(rest.is_empty());
    }

    #[test]
    fn parse_stdout_4() {
        let mut parser = StdoutParser::default();
        let first = b"source\042\0pattern\0pathname\0";
        let second = b"\0\0\0pathname\0";
        let stdout = [&first[..], second].concat();

        let (is_ignored, rest) = parser.feed(&stdout).unwrap();
        assert!(is_ignored);
        assert_eq!(rest, second);

        let (is_ignored, rest) = parser.feed(rest).unwrap();
        assert!(!is_ignored);
        assert!(rest.is_empty());
    }

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
