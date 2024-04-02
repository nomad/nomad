use core::any::Any;
use core::fmt::{self, Display};
use std::panic::{self, Location, PanicInfo};

use super::error;

/// Initializes the panic hook.
pub(super) fn init() {
    panic::set_hook(Box::new(|info| error!("{}", PanicMsg::from(info))));
}

struct PanicMsg<'a> {
    location: Option<&'a Location<'a>>,
    msg: Option<&'a dyn Display>,
}

impl<'a> From<&'a PanicInfo<'_>> for PanicMsg<'a> {
    fn from(info: &'a PanicInfo<'_>) -> Self {
        let payload = info.payload();

        let msg = downcast_display::<&str>(payload)
            .or_else(|| downcast_display::<String>(payload))
            .or_else(|| downcast_display::<&String>(payload));

        Self { location: info.location(), msg }
    }
}

impl Display for PanicMsg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "panicked")?;

        if let Some(loc) = &self.location {
            write!(f, " at {}:{}:{}", loc.file(), loc.line(), loc.column())?;
        }

        if let Some(msg) = &self.msg {
            write!(f, ": {}", msg)?;
        }

        Ok(())
    }
}

fn downcast_display<T: Any + Display>(
    value: &dyn Any,
) -> Option<&dyn Display> {
    value.downcast_ref::<T>().map(|msg| msg as &dyn Display)
}
