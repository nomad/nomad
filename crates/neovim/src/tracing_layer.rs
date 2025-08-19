use std::io::{self, Write};

use compact_str::CompactString;
use ed::executor::{Executor, LocalSpawner};
use ed::shared::MultiThreaded;
use ed::{Editor, Shared};
use tracing_subscriber::fmt;
use tracing_subscriber::registry::LookupSpan;

use crate::Neovim;

/// A [`tracing_subscriber::Layer`] implementation that displays logs in the
/// Neovim message area.
pub struct TracingLayer<S> {
    inner: fmt::Layer<
        S,
        fmt::format::DefaultFields,
        fmt::format::Format<fmt::format::Full, ()>,
        MessageAreaWriter,
    >,
}

/// A [`Write`]r that writes messages to the Neovim message area.
#[derive(Clone)]
struct MessageAreaWriter {
    /// The buffer where the messages are written.
    buffer: Shared<CompactString, MultiThreaded>,

    /// The sender used to send messages to the main thread when `Self` is
    /// flushed.
    message_tx: flume::Sender<CompactString>,
}

/// A [`Write`]r-wrapper that flushes the inner writer when it's dropped.
struct FlushOnDrop<W: io::Write> {
    inner: W,
}

impl<S> TracingLayer<S> {
    pub(crate) fn new(nvim: &mut Neovim) -> Self {
        let inner = fmt::Layer::default()
            .with_ansi(false)
            .without_time()
            .with_writer(MessageAreaWriter::new(nvim));

        Self { inner }
    }
}

impl MessageAreaWriter {
    fn new(nvim: &mut Neovim) -> Self {
        let (message_tx, message_rx) = flume::unbounded::<CompactString>();

        // We do this because print! can only be called from the main thread.
        nvim.executor()
            .local_spawner()
            .spawn(async move {
                while let Ok(message) = message_rx.recv_async().await {
                    nvim_oxi::print!(
                        "{}",
                        // Each call to print! is already displayed on a new
                        // line, so strip any trailing newlines.
                        message
                            .strip_suffix("\r\n")
                            .or_else(|| message.strip_suffix('\n'))
                            .unwrap_or(&message)
                    );
                }
            })
            .detach();

        Self { buffer: Default::default(), message_tx }
    }
}

impl fmt::MakeWriter<'_> for MessageAreaWriter {
    type Writer = FlushOnDrop<Self>;

    fn make_writer(&self) -> Self::Writer {
        // Tracing's layer constructs a new writer for each event, so we wrap
        // the inner writer with one that flushes it when it's dropped.
        FlushOnDrop { inner: self.clone() }
    }
}

impl io::Write for MessageAreaWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buffer.with_mut(|message_buffer| {
            message_buffer.push_str(&String::from_utf8_lossy(buf))
        });
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let message = self.buffer.take();
        if !message.is_empty() {
            self.message_tx.send(message).expect("the task is still running");
        }
        Ok(())
    }
}

impl<W: io::Write> io::Write for FlushOnDrop<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl<W: io::Write> Drop for FlushOnDrop<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for TracingLayer<S>
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_register_dispatch(&self, subscriber: &tracing::Dispatch) {
        self.inner.on_register_dispatch(subscriber);
    }

    fn on_layer(&mut self, subscriber: &mut S) {
        self.inner.on_layer(subscriber);
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

    fn boxed(
        self,
    ) -> Box<dyn tracing_subscriber::Layer<S> + Send + Sync + 'static>
    where
        Self: Sized,
        Self: tracing_subscriber::Layer<S> + Send + Sync + 'static,
        S: tracing::Subscriber,
    {
        self.inner.boxed()
    }
}
