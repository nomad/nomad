//! Contains types and traits related to reporting the progress of long-running
//! operations to the user.

use std::borrow::Cow;

use abs_path::{AbsPath, NodeName};
use editor::Context;

use crate::{CollabEditor, config, join, start};

/// A trait for types that can report the progress of [`Pipeline`]s to the
/// user.
///
/// Editors that don't support progress reporting can set their
/// [`ProgressReporter`](CollabEditor::ProgressReporter) to `()`, which
/// implements this trait for all [`CollabEditor`]s by simply doing nothing.
pub trait ProgressReporter<Ed: CollabEditor, P: Pipeline> {
    /// Returns a new instance of the reporter.
    fn new(ctx: &mut Context<Ed>) -> Self;

    /// Reports a progress update.
    fn report_progress(&mut self, state: P::State<'_>, ctx: &mut Context<Ed>);

    /// Reports that the pipeline has completed successfully.
    fn report_success(self, output: P::Output<'_>, ctx: &mut Context<Ed>);

    /// Reports that the pipeline has failed with an error.
    fn report_error(self, error: P::Error<'_>, ctx: &mut Context<Ed>);

    /// Reports that the pipeline has been cancelled.
    fn report_cancellation(self, ctx: &mut Context<Ed>);
}

/// Represents a long-running operation whose progress can be reported to the
/// user.
pub trait Pipeline {
    /// The type of value returned on success.
    type Output<'a>;

    /// The type of error that can occur at any point during the operation.
    type Error<'a>;

    /// The type representing the current progress state of the operation.
    type State<'a>;
}

impl<Ed: CollabEditor> Pipeline for join::Join<Ed> {
    type Output<'a> = ();
    type Error<'a> = join::JoinError<Ed>;
    type State<'a> = JoinState<'a>;
}

impl<Ed: CollabEditor> Pipeline for start::Start<Ed> {
    type Output<'a> = ();
    type Error<'a> = start::StartError<Ed>;
    type State<'a> = StartState<'a>;
}

/// An enum representing the different progress states of the
/// [`Join`](join::Join) pipeline.
///
/// The variants form a linear sequence, and each variant is guaranteed to be
/// followed by either another instance of the same variant, or the next
/// variant in the sequence.
pub enum JoinState<'a> {
    /// The client is connecting to the server at the given address.
    ConnectingToServer(config::ServerAddress<'a>),

    /// The client has connected to the server, and is now waiting for it to
    /// respond with a [`Welcome`](collab_server::client::Welcome) message.
    JoiningSession,

    /// We've received the [`Welcome`](collab_server::client::Welcome) message,
    /// and are now waiting to receive the project with the given name from
    /// another peer in the session.
    ReceivingProject(Cow<'a, NodeName>),

    /// We've received the project, and are now writing it to disk under the
    /// directory at the given path.
    WritingProject(Cow<'a, AbsPath>),
}

/// An enum representing the different progress states of the
/// [`Start`](start::Start) pipeline.
///
/// The variants form a linear sequence, and each variant is guaranteed to be
/// followed by either another instance of the same variant, or the next
/// variant in the sequence.
pub enum StartState<'a> {
    /// The client is connecting to the server at the given address.
    ConnectingToServer(config::ServerAddress<'a>),

    /// The client has connected to the server, and is now waiting for it to
    /// respond with a [`Welcome`](collab_server::client::Welcome) message.
    StartingSession,

    /// We've received the [`Welcome`](collab_server::client::Welcome) message,
    /// and are now reading the project rooted at the given path.
    ReadingProject(Cow<'a, AbsPath>),
}

impl JoinState<'_> {
    /// Returns a `'static` version of this [`JoinState`].
    pub fn to_owned(&self) -> JoinState<'static> {
        match self {
            Self::ConnectingToServer(server_addr) => {
                JoinState::ConnectingToServer(server_addr.to_owned())
            },
            Self::JoiningSession => JoinState::JoiningSession,
            Self::ReceivingProject(project_name) => {
                JoinState::ReceivingProject(Cow::Owned(
                    project_name.clone().into_owned(),
                ))
            },
            Self::WritingProject(root_path) => JoinState::WritingProject(
                Cow::Owned(root_path.clone().into_owned()),
            ),
        }
    }
}

impl StartState<'_> {
    /// Returns a `'static` version of this [`StartState`].
    pub fn to_owned(&self) -> StartState<'static> {
        match self {
            Self::ConnectingToServer(server_addr) => {
                StartState::ConnectingToServer(server_addr.to_owned())
            },
            Self::StartingSession => StartState::StartingSession,
            Self::ReadingProject(root_path) => StartState::ReadingProject(
                Cow::Owned(root_path.clone().into_owned()),
            ),
        }
    }
}

impl<Ed: CollabEditor, P: Pipeline> ProgressReporter<Ed, P> for () {
    fn new(_: &mut Context<Ed>) -> Self {}
    fn report_progress(&mut self, _: P::State<'_>, _: &mut Context<Ed>) {}
    fn report_success(self, _: P::Output<'_>, _: &mut Context<Ed>) {}
    fn report_error(self, _: P::Error<'_>, _: &mut Context<Ed>) {}
    fn report_cancellation(self, _: &mut Context<Ed>) {}
}
