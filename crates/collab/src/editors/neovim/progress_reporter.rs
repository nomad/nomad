use abs_path::{NodeName, NodeNameBuf};
use compact_str::{ToCompactString, format_compact};
use editor::Context;
use neovim::Neovim;
use neovim::notify::{self, NotifyContextExt, Percentage};

use crate::editors::neovim::notifications;
use crate::progress::{JoinState, Pipeline, ProgressReporter, StartState};
use crate::{config, join, start};

pub struct NeovimProgressReporter {
    inner: notify::ProgressReporter,
    state: ReporterState,
}

trait DisplayablePipeline: Pipeline {
    fn display_output(
        output: Self::Output<'_>,
        state: &mut ReporterState,
        ctx: &mut Context<Neovim>,
    ) -> notify::Chunks;

    fn display_error(
        error: Self::Error<'_>,
        state: &mut ReporterState,
        ctx: &mut Context<Neovim>,
    ) -> notify::Chunks;

    fn display_state(
        state: Self::State<'_>,
        state: &mut ReporterState,
        ctx: &mut Context<Neovim>,
    ) -> Option<notify::Chunks>;
}

#[derive(Default)]
struct ReporterState {
    project_name: Option<NodeNameBuf>,
    server_address: Option<config::ServerAddress<'static>>,
    percentage: Option<Percentage>,
}

impl<P: DisplayablePipeline> ProgressReporter<Neovim, P>
    for NeovimProgressReporter
{
    fn new(ctx: &mut Context<Neovim>) -> Self {
        Self { inner: ctx.new_progress_reporter(), state: Default::default() }
    }

    fn report_success(
        mut self,
        output: P::Output<'_>,
        ctx: &mut Context<Neovim>,
    ) {
        self.inner.report_success(P::display_output(
            output,
            &mut self.state,
            ctx,
        ));
    }

    fn report_error(mut self, error: P::Error<'_>, ctx: &mut Context<Neovim>) {
        self.inner.report_error(P::display_error(error, &mut self.state, ctx));
    }

    fn report_progress(
        &mut self,
        state: P::State<'_>,
        ctx: &mut Context<Neovim>,
    ) {
        if let Some(chunks) = P::display_state(state, &mut self.state, ctx) {
            self.inner.report_progress(chunks, self.state.percentage);
        }
    }

    fn report_cancellation(self, _: &mut Context<Neovim>) {}
}

impl DisplayablePipeline for join::Join<Neovim> {
    fn display_output(
        _: (),
        _: &mut ReporterState,
        _: &mut Context<Neovim>,
    ) -> notify::Chunks {
        "Joined session".into()
    }

    fn display_error(
        error: Self::Error<'_>,
        _: &mut ReporterState,
        _: &mut Context<Neovim>,
    ) -> notify::Chunks {
        error.into()
    }

    fn display_state(
        join_state: Self::State<'_>,
        reporter_state: &mut ReporterState,
        ctx: &mut Context<Neovim>,
    ) -> Option<notify::Chunks> {
        let mut chunks = Default::default();

        match join_state {
            JoinState::ConnectingToServer(server_addr) => {
                reporter_state.server_address = Some(server_addr.to_owned());
                return Self::display_state(
                    JoinState::JoiningSession,
                    reporter_state,
                    ctx,
                );
            },

            JoinState::JoiningSession => {
                let server_addr =
                    reporter_state.server_address.as_ref().expect(
                        "JoiningSession must be preceded by \
                         ConnectingToServer",
                    );

                chunks = connecting_to_server(server_addr);
            },

            JoinState::ReceivedWelcome(project_name) => {
                let project_name = &*project_name;
                chunks = receiving_files(project_name);
                reporter_state.project_name = Some(project_name.to_owned());
            },

            JoinState::ReceivingProject(bytes_received, bytes_total) => {
                let new_percentage =
                    ((bytes_received as f32 / bytes_total as f32) * 100.0)
                        .round() as Percentage;

                match reporter_state.percentage {
                    // Only emit an update when the progress percentage
                    // changes.
                    Some(old) if old == new_percentage => return None,
                    _ => reporter_state.percentage = Some(new_percentage),
                }

                let project_name =
                    reporter_state.project_name.as_deref().expect(
                        "ReceivingProject must be preceded by ReceivedWelcome",
                    );

                chunks = receiving_files(project_name);
            },

            JoinState::WritingProject(root_path) => {
                // Clear the percentage set while receiving the project.
                reporter_state.percentage = None;

                let project_name =
                    reporter_state.project_name.as_ref().expect(
                        "WritingProject must be preceded by ReceivingProject",
                    );

                chunks
                    .push("Writing project ")
                    .push_highlighted(
                        project_name.to_string(),
                        notifications::PROJ_NAME_HL_GROUP,
                    )
                    .push(" to ")
                    .push_chunk(notifications::path_chunk(&root_path, ctx));
            },
        }

        Some(chunks)
    }
}

impl DisplayablePipeline for start::Start<Neovim> {
    fn display_output(
        _: (),
        _: &mut ReporterState,
        _: &mut Context<Neovim>,
    ) -> notify::Chunks {
        "Started session".into()
    }

    fn display_error(
        error: Self::Error<'_>,
        _: &mut ReporterState,
        _: &mut Context<Neovim>,
    ) -> notify::Chunks {
        error.into()
    }

    fn display_state(
        start_state: Self::State<'_>,
        reporter_state: &mut ReporterState,
        ctx: &mut Context<Neovim>,
    ) -> Option<notify::Chunks> {
        let mut chunks = Default::default();

        match start_state {
            StartState::ConnectingToServer(server_addr) => {
                reporter_state.server_address = Some(server_addr.to_owned());
                return Self::display_state(
                    StartState::StartingSession,
                    reporter_state,
                    ctx,
                );
            },

            StartState::StartingSession => {
                let server_addr =
                    reporter_state.server_address.as_ref().expect(
                        "StartingSession must be preceded by \
                         ConnectingToServer",
                    );
                chunks = connecting_to_server(server_addr);
            },

            StartState::ReadingProject(root_path) => {
                chunks
                    .push("Reading project at ")
                    .push_chunk(notifications::path_chunk(&root_path, ctx));
            },
        }

        Some(chunks)
    }
}

fn connecting_to_server(
    server_addr: &config::ServerAddress,
) -> notify::Chunks {
    let mut chunks = notify::Chunks::default();
    chunks
        .push("Connecting to server at ")
        .push_highlighted(server_addr.host.to_compact_string(), "Identifier")
        .push_highlighted(format_compact!(":{}", server_addr.port), "Comment");
    chunks
}

fn receiving_files(project_name: &NodeName) -> notify::Chunks {
    let mut chunks = notify::Chunks::default();
    chunks.push("Receiving files for project ").push_highlighted(
        project_name.as_str(),
        notifications::PROJ_NAME_HL_GROUP,
    );
    chunks
}

impl From<join::JoinError<Neovim>> for notify::Chunks {
    fn from(error: join::JoinError<Neovim>) -> Self {
        match error {
            join::JoinError::UserNotLoggedIn => {
                start::StartError::UserNotLoggedIn.into()
            },
            other => other.to_string().into(),
        }
    }
}

impl From<start::StartError<Neovim>> for notify::Chunks {
    fn from(error: start::StartError<Neovim>) -> Self {
        match error {
            start::StartError::UserNotLoggedIn => {
                let mut chunks = Self::default();

                let login =
                    <auth::login::Login as editor::module::AsyncAction<
                        Neovim,
                    >>::NAME;

                chunks
                    .push(
                        "You must be logged in to start collaborating. You \
                         can log in by executing:",
                    )
                    .push_newline()
                    .push_highlighted(":", "Comment")
                    .push_highlighted(format_compact!("Mad {login}"), "Title");

                chunks
            },
            other => other.to_string().into(),
        }
    }
}
