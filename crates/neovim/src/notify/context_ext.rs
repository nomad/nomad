use editor::Editor;
use editor::context::{BorrowState, Context};
use executor::Executor;
use nvim_oxi::api::types::LogLevel;

use crate::Neovim;
use crate::notify::{self, Chunks, ProgressReporter};

/// An extension trait for `Context<Neovim>` providing methods to emit
/// notifications via the `vim.notify()` API.
pub trait NotifyContextExt {
    /// Emits a notification with the given message and level.
    fn new_progress_reporter(&mut self) -> ProgressReporter;

    /// Emits a notification with the given message and level.
    fn notify(
        &mut self,
        notification_message: impl Into<Chunks>,
        notification_level: LogLevel,
    );

    /// Emits a notification at the `ERROR` level with the given message.
    fn notify_error(&mut self, notification_message: impl Into<Chunks>) {
        self.notify(notification_message, LogLevel::Error);
    }

    /// Emits a notification at the `INFO` level with the given message.
    fn notify_info(&mut self, notification_message: impl Into<Chunks>) {
        self.notify(notification_message, LogLevel::Info);
    }

    /// Emits a notification at the `WARN` level with the given message.
    fn notify_warn(&mut self, notification_message: impl Into<Chunks>) {
        self.notify(notification_message, LogLevel::Warn);
    }
}

impl<Bs: BorrowState> NotifyContextExt for Context<Neovim, Bs> {
    #[inline]
    fn new_progress_reporter(&mut self) -> ProgressReporter {
        ProgressReporter::new(self)
    }

    #[inline]
    fn notify(
        &mut self,
        notification_message: impl Into<Chunks>,
        notification_level: LogLevel,
    ) {
        if notify::NvimNotify::is_installed() {
            let namespace_id = self.with_editor(|nvim| nvim.namespace_id());
            notify::NvimNotify::notify(
                self.namespace(),
                notification_message.into(),
                notification_level,
                namespace_id,
            )
        } else {
            let namespace = self.namespace().clone();
            self.with_editor(|nvim| {
                notify::NvimEcho::notify(
                    &namespace,
                    notification_message.into(),
                    notification_level,
                    nvim.executor().local_spawner(),
                )
            })
        }
    }
}
