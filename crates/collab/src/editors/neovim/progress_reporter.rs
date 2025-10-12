use core::fmt;
use core::time::Duration;

use abs_path::NodeNameBuf;
use editor::notify::{self, Emitter};
use editor::{Context, Editor};
use flume::TrySendError;
use futures_util::{FutureExt, StreamExt, select_biased};
use neovim::Neovim;

use crate::progress::{JoinState, Pipeline, ProgressReporter, StartState};
use crate::{config, join, start};

/// Frames for the spinner animation.
const SPINNER_FRAMES: &[&str] = &["⣾", "⣽", "⣻", "⢿", "⡿", "⣟", "⣯", "⣷"];

/// How many revolutions per minute the spinner should complete.
const SPINNER_RPM: u8 = 75;

/// How often the spinner should be updated to achieve the desired RPM.
const SPINNER_UPDATE_INTERVAL: Duration = Duration::from_millis({
    (60_000.0 / ((SPINNER_RPM as u16 * SPINNER_FRAMES.len() as u16) as f32))
        .round() as u64
});

pub struct NeovimProgressReporter {
    updates_tx: flume::Sender<Update>,
    state: ReporterState,
}

trait DisplayablePipeline: Pipeline {
    fn display_output(
        output: Self::Output<'_>,
        state: &mut ReporterState,
    ) -> String;

    fn display_error(
        error: Self::Error<'_>,
        state: &mut ReporterState,
    ) -> String;

    fn display_state(
        state: Self::State<'_>,
        state: &mut ReporterState,
    ) -> String;
}

#[derive(Default)]
struct ReporterState {
    project_name: Option<NodeNameBuf>,
    server_address: Option<config::ServerAddress<'static>>,
}

struct Update {
    level: notify::Level,
    text: String,
    is_last: bool,
}

impl NeovimProgressReporter {
    fn send_update(&self, update: Update) {
        if let Err(err) = self.updates_tx.try_send(update) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
    }
}

impl<P: DisplayablePipeline> ProgressReporter<Neovim, P>
    for NeovimProgressReporter
{
    fn new(ctx: &mut Context<Neovim>) -> Self {
        let (updates_tx, updates_rx) = flume::bounded::<Update>(4);

        ctx.spawn_local(async move |ctx| {
            let namespace = ctx.namespace().clone();
            let mut spin = async_io::Timer::interval(SPINNER_UPDATE_INTERVAL);
            let mut updates = updates_rx.into_stream();

            let Some(mut update) = updates.next().await else { return };
            let mut spinner_frame_idx = 0;
            let mut prev_id = None;

            loop {
                let prefix: &dyn fmt::Display = if !update.is_last {
                    &format_args!("{} ", SPINNER_FRAMES[spinner_frame_idx])
                } else if update.level == notify::Level::Error {
                    &""
                } else {
                    &"✔ "
                };

                prev_id = ctx.with_editor(|nvim| {
                    Some(nvim.emitter().emit(notify::Notification {
                        level: update.level,
                        message: notify::Message::from_display(format_args!(
                            "{prefix}{}",
                            update.text,
                        )),
                        namespace: &namespace,
                        updates_prev: prev_id,
                    }))
                });

                if update.is_last {
                    break;
                }

                select_biased! {
                    _ = spin.next().fuse() => {
                        spinner_frame_idx += 1;
                        spinner_frame_idx %= SPINNER_FRAMES.len();
                    },
                    next_update = updates.next() => {
                        match next_update {
                            Some(next_update) => update = next_update,
                            // The pipeline has been cancelled.
                            None => break,
                        }
                    },
                }
            }
        })
        .detach();

        Self { updates_tx, state: Default::default() }
    }

    fn report_success(
        mut self,
        output: P::Output<'_>,
        _: &mut Context<Neovim>,
    ) {
        let text = P::display_output(output, &mut self.state);
        self.send_update(Update {
            level: notify::Level::Info,
            text,
            is_last: true,
        });
    }

    fn report_error(mut self, error: P::Error<'_>, _: &mut Context<Neovim>) {
        let text = P::display_error(error, &mut self.state);
        self.send_update(Update {
            level: notify::Level::Error,
            text,
            is_last: true,
        });
    }

    fn report_progress(
        &mut self,
        state: P::State<'_>,
        _: &mut Context<Neovim>,
    ) {
        let text = P::display_state(state, &mut self.state);
        self.send_update(Update {
            level: notify::Level::Info,
            text,
            is_last: false,
        });
    }

    fn report_cancellation(self, _: &mut Context<Neovim>) {}
}

impl DisplayablePipeline for join::Join<Neovim> {
    fn display_output(_: (), _: &mut ReporterState) -> String {
        "Joined session".to_owned()
    }

    fn display_error(error: Self::Error<'_>, _: &mut ReporterState) -> String {
        match error {
            join::JoinError::UserNotLoggedIn => user_not_logged_in_message(),
            other => other.to_string(),
        }
    }

    fn display_state(
        join_state: Self::State<'_>,
        reporter_state: &mut ReporterState,
    ) -> String {
        match &join_state {
            JoinState::ConnectingToServer(server_addr) => {
                reporter_state.server_address = Some(server_addr.to_owned());
                Self::display_state(JoinState::JoiningSession, reporter_state)
            },

            JoinState::JoiningSession => {
                let server_addr =
                    reporter_state.server_address.as_ref().expect(
                        "StartingSession must be preceded by \
                         ConnectingToServer",
                    );
                format!("Connecting to server at {server_addr}")
            },

            JoinState::ReceivingProject(project_name) => {
                reporter_state.project_name =
                    Some((**project_name).to_owned());
                format!("Receiving files for {project_name}")
            },

            JoinState::WritingProject(root_path) => {
                let project_name =
                    reporter_state.project_name.as_ref().expect(
                        "WritingProject must be preceded by ReceivingProject",
                    );
                format!("Writing {project_name} to {root_path}")
            },
        }
    }
}

impl DisplayablePipeline for start::Start<Neovim> {
    fn display_output(_: (), _: &mut ReporterState) -> String {
        "Started session".to_owned()
    }

    fn display_error(error: Self::Error<'_>, _: &mut ReporterState) -> String {
        match error {
            start::StartError::UserNotLoggedIn => user_not_logged_in_message(),
            other => other.to_string(),
        }
    }

    fn display_state(
        state: Self::State<'_>,
        reporter_state: &mut ReporterState,
    ) -> String {
        match state {
            StartState::ConnectingToServer(server_addr) => {
                reporter_state.server_address = Some(server_addr.to_owned());
                Self::display_state(
                    StartState::StartingSession,
                    reporter_state,
                )
            },

            StartState::StartingSession => {
                let server_addr =
                    reporter_state.server_address.as_ref().expect(
                        "StartingSession must be preceded by \
                         ConnectingToServer",
                    );
                format!("Connecting to server at {server_addr}")
            },

            StartState::ReadingProject(root_path) => {
                format!("Reading project at {root_path}")
            },
        }
    }
}

fn user_not_logged_in_message() -> String {
    format!(
        "You must be logged in to start collaborating. You can log in by \
         executing ':Mad {}'",
        <auth::login::Login as editor::module::AsyncAction::<Neovim>>::NAME
    )
}
