use core::fmt;

use nvimx2::{Constant, Name};

use crate::generated;

/// TODO: docs.
pub const VERSION: Version = Version {
    commit: generated::COMMIT_HASH,
    date: Date {
        year: generated::COMMIT_YEAR,
        month: generated::COMMIT_MONTH,
        day: generated::COMMIT_DAY,
    },
    is_nightly: generated::IS_NIGHTLY,
    semantic: SemanticVersion {
        major: generated::MAJOR,
        minor: generated::MINOR,
        patch: generated::PATCH,
    },
};

/// TODO: docs.
#[derive(serde::Serialize)]
pub struct Version {
    commit: &'static str,
    date: Date,
    is_nightly: bool,
    semantic: SemanticVersion,
}

#[derive(serde::Serialize)]
struct SemanticVersion {
    major: u8,
    minor: u8,
    patch: u8,
}

#[derive(serde::Serialize)]
struct Date {
    year: u16,
    month: u8,
    day: u8,
}

impl Constant for Version {
    const NAME: Name = "version";
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "mad.nvim {semantic:?}{maybe_nightly} ({commit} {date:?})",
            semantic = self.semantic,
            maybe_nightly = if self.is_nightly { "-nightly" } else { "" },
            commit = self.commit,
            date = self.date,
        )
    }
}

impl fmt::Debug for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{major}.{minor}.{patch}",
            major = self.major,
            minor = self.minor,
            patch = self.patch,
        )
    }
}

impl fmt::Debug for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{year}-{month:02}-{day:02}",
            year = self.year,
            month = self.month,
            day = self.day,
        )
    }
}
