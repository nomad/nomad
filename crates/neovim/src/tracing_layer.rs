/// A [`tracing_subscriber::Layer`] implementation that displays logs in the
/// Neovim message area.
pub struct TracingLayer {}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for TracingLayer {}
