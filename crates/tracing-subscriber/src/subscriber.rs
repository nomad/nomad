use std::env;
use std::path::PathBuf;

use time::format_description::FormatItem;
use time::macros::format_description;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::{filter, fmt, fmt::format, fmt::time::UtcTime};

type FmtSubscriber = fmt::Subscriber<
    format::DefaultFields,
    format::Format<format::Full, UtcTime<&'static [FormatItem<'static>]>>,
    filter::LevelFilter,
    NonBlocking,
>;

const LOG_FILE_NAME: &str = "nomad.log";

/// TODO: docs
struct Subscriber {
    subscriber: FmtSubscriber,
    _guard: WorkerGuard,
}

pub fn subscriber() -> impl tracing::Subscriber {
    let file_appender =
        tracing_appender::rolling::never(log_dir(), LOG_FILE_NAME);

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let timer = UtcTime::new(format_description!(
        "[year]-[month]-[day]T[hour]:[minute]:[second]Z"
    ));

    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_thread_names(true)
        .with_timer(timer)
        .with_writer(non_blocking)
        .finish();

    Subscriber { subscriber, _guard }
}

#[cfg(unix)]
fn log_dir() -> PathBuf {
    if let Ok(nomad_log_dir) = env::var("NOMAD_LOG_DIR") {
        PathBuf::from(nomad_log_dir)
    } else if let Ok(xdg_state_home) = env::var("XDG_STATE_HOME") {
        PathBuf::from(xdg_state_home).join("nomad")
    } else if let Some(home_dir) = home::home_dir() {
        home_dir.join(".local").join("state").join("nomad")
    } else {
        panic!("Could not determine log directory");
    }
}

#[cfg(windows)]
fn log_dir() -> PathBuf {
    todo!();
}

mod subscriber_impl {
    use super::*;

    impl tracing::Subscriber for Subscriber {
        fn enabled(&self, metadata: &tracing::Metadata<'_>) -> bool {
            self.subscriber.enabled(metadata)
        }

        fn new_span(
            &self,
            span: &tracing::span::Attributes<'_>,
        ) -> tracing::span::Id {
            self.subscriber.new_span(span)
        }

        fn record(
            &self,
            span: &tracing::span::Id,
            values: &tracing::span::Record<'_>,
        ) {
            self.subscriber.record(span, values)
        }

        fn record_follows_from(
            &self,
            span: &tracing::span::Id,
            follows: &tracing::span::Id,
        ) {
            self.subscriber.record_follows_from(span, follows)
        }

        fn event(&self, event: &tracing::Event<'_>) {
            self.subscriber.event(event)
        }

        fn enter(&self, span: &tracing::span::Id) {
            self.subscriber.enter(span)
        }

        fn exit(&self, span: &tracing::span::Id) {
            self.subscriber.exit(span)
        }

        fn clone_span(&self, id: &tracing::span::Id) -> tracing::span::Id {
            self.subscriber.clone_span(id)
        }

        fn try_close(&self, id: tracing::span::Id) -> bool {
            self.subscriber.try_close(id)
        }
    }
}
