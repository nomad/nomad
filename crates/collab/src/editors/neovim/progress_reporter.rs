use core::fmt;
use core::time::Duration;

use abs_path::NodeNameBuf;
use editor::notify::{self, Emitter};
use editor::{Context, Editor};
use flume::TrySendError;
use futures_util::{FutureExt, StreamExt, select_biased};
use neovim::Neovim;

use crate::progress::{JoinState, ProgressReporter, StartState};
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
    message_tx: flume::Sender<Message>,
    state: ReporterState,
}

#[derive(Default)]
struct ReporterState {
    project_name: Option<NodeNameBuf>,
    server_address: Option<config::ServerAddress<'static>>,
}

struct Message {
    level: notify::Level,
    text: String,
    is_last: bool,
}

impl ProgressReporter<Neovim> for NeovimProgressReporter {
    fn new(ctx: &mut Context<Neovim>) -> Self {
        let (message_tx, message_rx) = flume::bounded::<Message>(4);

        ctx.spawn_local(async move |ctx| {
            let namespace = ctx.namespace().clone();
            let mut spin = async_io::Timer::interval(SPINNER_UPDATE_INTERVAL);
            let mut messages = message_rx.into_stream();

            let Some(mut message) = messages.next().await else { return };
            let mut spinner_frame_idx = 0;
            let mut prev_id = None;

            loop {
                let prefix: &dyn fmt::Display = if !message.is_last {
                    &format_args!("{} ", SPINNER_FRAMES[spinner_frame_idx])
                } else if message.level == notify::Level::Error {
                    &""
                } else {
                    &"✔ "
                };

                prev_id = ctx.with_editor(|nvim| {
                    Some(nvim.emitter().emit(notify::Notification {
                        level: message.level,
                        message: notify::Message::from_display(format_args!(
                            "{prefix}{}",
                            message.text,
                        )),
                        namespace: &namespace,
                        updates_prev: prev_id,
                    }))
                });

                if message.is_last {
                    break;
                }

                select_biased! {
                    _ = spin.next().fuse() => {
                        spinner_frame_idx += 1;
                        spinner_frame_idx %= SPINNER_FRAMES.len();
                    },
                    next_message = messages.select_next_some() => {
                        message = next_message
                    },
                }
            }
        })
        .detach();

        Self { message_tx, state: Default::default() }
    }

    fn report_join_progress(
        &mut self,
        state: JoinState<'_, Neovim>,
        _: &mut Context<Neovim>,
    ) {
        let level = match &state {
            JoinState::Done(Err(_)) => notify::Level::Error,
            _ => notify::Level::Info,
        };

        let message = Message {
            level,
            text: join_progress_message(&state, &mut self.state),
            is_last: matches!(state, JoinState::Done(_)),
        };

        if let Err(err) = self.message_tx.try_send(message) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
    }

    fn report_start_progress(
        &mut self,
        state: StartState<'_, Neovim>,
        _: &mut Context<Neovim>,
    ) {
        // If the user did not confirm starting a new session, don't show any
        // message.
        if let StartState::Done(Err(start::StartError::UserDidNotConfirm)) =
            &state
        {
            return;
        }

        let level = match &state {
            StartState::Done(Err(_)) => notify::Level::Error,
            _ => notify::Level::Info,
        };

        let message = Message {
            level,
            text: start_progress_message(&state, &mut self.state),
            is_last: matches!(state, StartState::Done(_)),
        };

        if let Err(err) = self.message_tx.try_send(message) {
            match err {
                TrySendError::Disconnected(_) => unreachable!(),
                TrySendError::Full(_) => {},
            }
        }
    }
}

fn join_progress_message(
    join_state: &JoinState<'_, Neovim>,
    reporter_state: &mut ReporterState,
) -> String {
    match &join_state {
        JoinState::ConnectingToServer(server_addr) => {
            reporter_state.server_address = Some(server_addr.to_owned());
            join_progress_message(&JoinState::JoiningSession, reporter_state)
        },

        JoinState::JoiningSession => {
            let server_addr = reporter_state.server_address.as_ref().expect(
                "StartingSession must be preceded by ConnectingToServer",
            );
            format!("Connecting to server at {server_addr}")
        },

        JoinState::ReceivingProject(project_name) => {
            reporter_state.project_name = Some((**project_name).to_owned());
            format!("Receiving files for {project_name}")
        },

        JoinState::WritingProject(root_path) => {
            let project_name = reporter_state
                .project_name
                .as_ref()
                .expect("WritingProject must be preceded by ReceivingProject");
            format!("Writing {project_name} to {root_path}")
        },

        JoinState::Done(Ok(())) => "Joined session".to_owned(),

        JoinState::Done(Err(err)) => match err {
            join::JoinError::UserNotLoggedIn => user_not_logged_in_message(),
            other => other.to_string(),
        },
    }
}

fn start_progress_message(
    state: &StartState<'_, Neovim>,
    reporter_state: &mut ReporterState,
) -> String {
    match state {
        StartState::ConnectingToServer(server_addr) => {
            reporter_state.server_address = Some(server_addr.to_owned());
            start_progress_message(
                &StartState::StartingSession,
                reporter_state,
            )
        },

        StartState::StartingSession => {
            let server_addr = reporter_state.server_address.as_ref().expect(
                "StartingSession must be preceded by ConnectingToServer",
            );
            format!("Connecting to server at {server_addr}")
        },

        StartState::ReadingProject(root_path) => {
            format!("Reading project at {root_path}")
        },

        StartState::Done(Ok(())) => "Started session".to_owned(),

        StartState::Done(Err(err)) => match err {
            start::StartError::UserNotLoggedIn => user_not_logged_in_message(),
            other => other.to_string(),
        },
    }
}

fn user_not_logged_in_message() -> String {
    format!(
        "You must be logged in to start collaborating. You can log in by \
         executing ':Mad {}'",
        <auth::login::Login as editor::module::AsyncAction::<Neovim>>::NAME
    )
}
