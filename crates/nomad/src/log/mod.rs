//! TODO: docs

mod panic_hook;
mod tracing_subscriber;

/// Intializes the logging system.
pub(crate) fn init(log_dir: &collab_fs::AbsUtf8Path) {
    panic_hook::init();
    tracing_subscriber::init(log_dir);
}
