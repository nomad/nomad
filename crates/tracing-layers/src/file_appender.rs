use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use abs_path::{AbsPath, AbsPathBuf, NodeName, NodeNameBuf};
use ed::{BorrowState, Context, Editor};
use fs::os::OsFs;
use tracing::error;
use tracing_appender::non_blocking::{NonBlocking, WorkerGuard};
use tracing_appender::rolling::{InitError, RollingFileAppender, Rotation};
use tracing_subscriber::Layer;
use tracing_subscriber::fmt::{self, format, time};
use tracing_subscriber::registry::LookupSpan;

/// A [`Layer`] implementation that appends logs to a file, creating a new file
/// every day.
#[derive(cauchy::Clone)]
pub struct FileAppender<S> {
    inner: Arc<OnceLock<FileAppenderInner<S>>>,
    creating_inner_has_failed: Arc<AtomicBool>,
}

struct FileAppenderInner<S> {
    inner: fmt::Layer<
        S,
        format::DefaultFields,
        format::Format<format::Full, time::ChronoUtc>,
        NonBlocking,
    >,

    /// We need to keep this guard around for the entire lifetime of the
    /// program to ensure that the logs are flushed properly.
    ///
    /// The `Drop` implementation of this guard will flush any remaining logs
    /// to the file in case the program is terminated abruptly, for example by
    /// a panic.
    _guard: WorkerGuard,
}

impl<S: 'static> FileAppender<S> {
    /// Creates a new `FileAppender` that creates daily log files under the
    /// given directory with the given file name prefix.
    pub fn new<Ed>(
        log_dir: AbsPathBuf,
        filename_prefix: NodeNameBuf,
        ctx: &mut Context<Ed, impl BorrowState>,
    ) -> Self
    where
        // Creating the inner file appender will access the file system, so
        // bound this to editors with a real file system.
        Ed: Editor<Fs = OsFs>,
    {
        let this = Self {
            inner: Arc::new(OnceLock::new()),
            creating_inner_has_failed: Arc::new(AtomicBool::new(false)),
        };

        let namespace = ctx.namespace().clone();

        // Creating the inner file appender does a bunch of blocking I/O, so we
        // do it in the background.
        ctx.spawn_background({
            let this = this.clone();
            async move {
                match FileAppenderInner::new(&log_dir, &filename_prefix) {
                    Ok(file_appender) => {
                        assert!(this.inner.set(file_appender).is_ok());
                    },
                    Err(err) => {
                        this.creating_inner_has_failed
                            .store(true, Ordering::Relaxed);
                        error!(
                            title = %namespace.dot_separated(),
                            "failed to create tracing file appender: {err}",
                        );
                    },
                };
            }
        })
        .detach();

        this
    }
}

impl<S> FileAppenderInner<S> {
    fn new(
        log_dir: &AbsPath,
        filename_prefix: &NodeName,
    ) -> Result<Self, InitError> {
        let file_appender = RollingFileAppender::builder()
            .rotation(Rotation::DAILY)
            .filename_prefix(filename_prefix.to_string())
            .build(log_dir)?;

        let (non_blocking, _guard) =
            tracing_appender::non_blocking(file_appender);

        let inner = fmt::Layer::default()
            .with_ansi(false)
            // Formats timestamps as "2001-07-08T00:34:60Z".
            .with_timer(time::ChronoUtc::new("%FT%TZ".to_owned()))
            .with_writer(non_blocking);

        Ok(Self { inner, _guard })
    }
}

impl<S> Layer<S> for FileAppender<S>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_register_dispatch(&self, subscriber: &tracing::Dispatch) {
        if let Some(inner) = self.inner.get() {
            inner.on_register_dispatch(subscriber);
        }
    }

    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        if let Some(inner) = self.inner.get() {
            inner.enabled(metadata, ctx)
        } else {
            !self.creating_inner_has_failed.load(Ordering::Relaxed)
        }
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_new_span(attrs, id, ctx);
        }
    }

    fn on_record(
        &self,
        span: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_record(span, values, ctx);
        }
    }

    fn on_follows_from(
        &self,
        span: &tracing::span::Id,
        follows: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_follows_from(span, follows, ctx);
        }
    }

    fn event_enabled(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        if let Some(inner) = self.inner.get() {
            inner.event_enabled(event, ctx)
        } else {
            !self.creating_inner_has_failed.load(Ordering::Relaxed)
        }
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_event(event, ctx);
        }
    }

    fn on_enter(
        &self,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_enter(id, ctx);
        }
    }

    fn on_exit(
        &self,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_exit(id, ctx);
        }
    }

    fn on_close(
        &self,
        id: tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_close(id, ctx);
        }
    }

    fn on_id_change(
        &self,
        old: &tracing::span::Id,
        new: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if let Some(inner) = self.inner.get() {
            inner.on_id_change(old, new, ctx);
        }
    }
}

impl<S: tracing::Subscriber> Layer<S> for FileAppenderInner<S>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_register_dispatch(&self, subscriber: &tracing::Dispatch) {
        self.inner.on_register_dispatch(subscriber);
    }

    fn register_callsite(
        &self,
        metadata: &'static tracing::Metadata<'static>,
    ) -> tracing::subscriber::Interest {
        self.inner.register_callsite(metadata)
    }

    fn enabled(
        &self,
        metadata: &tracing::Metadata<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        self.inner.enabled(metadata, ctx)
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_new_span(attrs, id, ctx);
    }

    fn on_record(
        &self,
        span: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_record(span, values, ctx);
    }

    fn on_follows_from(
        &self,
        span: &tracing::span::Id,
        follows: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_follows_from(span, follows, ctx);
    }

    fn event_enabled(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        self.inner.event_enabled(event, ctx)
    }

    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_event(event, ctx);
    }

    fn on_enter(
        &self,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_enter(id, ctx);
    }

    fn on_exit(
        &self,
        id: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_exit(id, ctx);
    }

    fn on_close(
        &self,
        id: tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_close(id, ctx);
    }

    fn on_id_change(
        &self,
        old: &tracing::span::Id,
        new: &tracing::span::Id,
        ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        self.inner.on_id_change(old, new, ctx);
    }
}
