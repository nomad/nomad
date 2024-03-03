use std::panic;

use super::error;

/// Initializes the panic hook.
pub(super) fn init() {
    panic::set_hook(Box::new(|info| {
        let Some(location) = info.location() else {
            error!("{info}");
            return;
        };

        error!(
            "panicked at {file}:{line}:{col}{maybe_msg}",
            file = location.file(),
            line = location.line(),
            col = location.column(),
            maybe_msg = info
                .payload()
                .downcast_ref::<&str>()
                .map(|msg| format!(": {msg}"))
                .unwrap_or_default(),
        );
    }));
}
