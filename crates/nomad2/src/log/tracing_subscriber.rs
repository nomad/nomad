use collab_fs::AbsUtf8Path;
use time::format_description::FormatItem;
use time::macros::format_description;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_subscriber::fmt::time::UtcTime;
use tracing_subscriber::fmt::{self, format};
use tracing_subscriber::{filter, fmt as fmt_builder};

/// The name of the log file.
///
/// We use a daily rolling file appender to prevent the log file from growing
/// indefinitely, so this is really just the prefix of any log file. The actual
/// log files will be named like `nomad.log.2020-01-01`.
const LOG_FILE_NAME: &str = "nomad.log";

pub(super) fn init(log_dir: &AbsUtf8Path) {
    let subscriber = NomadTracingSubscriber::new(log_dir);

    tracing::subscriber::set_global_default(subscriber)
        .expect("failed to set the global default subscriber");
}

type FmtSubscriber = fmt::Subscriber<
    format::DefaultFields,
    format::Format<format::Full, UtcTime<&'static [FormatItem<'static>]>>,
    filter::LevelFilter,
    NonBlocking,
>;

struct NomadTracingSubscriber {
    subscriber: FmtSubscriber,

    /// We need to keep this guard around for the entire lifetime of the
    /// program to ensure that the logs are flushed properly.
    ///
    /// The `Drop` implementation of this guard will flush any remaining logs
    /// to the file in case the program is terminated abruptly, for example by
    /// a panic.
    _guard: WorkerGuard,
}

impl NomadTracingSubscriber {
    fn new(log_dir: &AbsUtf8Path) -> Self {
        let file_appender =
            tracing_appender::rolling::daily(log_dir, LOG_FILE_NAME);

        let (non_blocking, _guard) =
            tracing_appender::non_blocking(file_appender);

        let timer = UtcTime::new(format_description!(
            "[year]-[month]-[day]T[hour]:[minute]:[second]Z"
        ));

        let subscriber = fmt_builder()
            .with_ansi(false)
            .with_max_level(tracing::Level::DEBUG)
            .with_timer(timer)
            .with_writer(non_blocking)
            .finish();

        Self { subscriber, _guard }
    }
}

impl tracing::Subscriber for NomadTracingSubscriber {
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
