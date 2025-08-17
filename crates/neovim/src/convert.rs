use ed::notify;

use crate::oxi::api::types::LogLevel;

/// Same as [`Into`], but for types defined in other crates (for which we
/// couldn't implement [`Into`] because of the orphan rule).
pub(crate) trait Convert<T> {
    fn convert(self) -> T;
}

impl Convert<LogLevel> for notify::Level {
    #[inline]
    fn convert(self) -> LogLevel {
        match self {
            Self::Off => LogLevel::Off,
            Self::Trace => LogLevel::Trace,
            Self::Debug => LogLevel::Debug,
            Self::Info => LogLevel::Info,
            Self::Warn => LogLevel::Warn,
            Self::Error => LogLevel::Error,
        }
    }
}

impl Convert<notify::Level> for LogLevel {
    #[inline]
    fn convert(self) -> notify::Level {
        match self {
            Self::Off => notify::Level::Off,
            Self::Trace => notify::Level::Trace,
            Self::Debug => notify::Level::Debug,
            Self::Info => notify::Level::Info,
            Self::Warn => notify::Level::Warn,
            Self::Error => notify::Level::Error,
            _ => notify::Level::Off,
        }
    }
}

impl<T> Convert<smallvec::SmallVec<[T; 1]>>
    for crate::oxi::api::types::OneOrMore<T>
{
    #[inline]
    fn convert(self) -> smallvec::SmallVec<[T; 1]> {
        match self {
            Self::One(item) => smallvec::smallvec_inline![item],
            Self::List(items) => items.into(),
        }
    }
}
