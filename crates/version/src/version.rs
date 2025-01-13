use core::fmt;

use nvimx2::module::Constant;
use nvimx2::notify::Name;

use crate::generated;

/// TODO: docs.
pub const VERSION: Version = Version {
    commit: generated::COMMIT_SHORT_HASH,
    date: Date {
        year: generated::COMMIT_YEAR,
        month: generated::COMMIT_MONTH,
        day: generated::COMMIT_DAY,
    },
    semantic: SemanticVersion {
        major: generated::MAJOR,
        minor: generated::MINOR,
        patch: generated::PATCH,
        pre: generated::PRE,
    },
};

/// TODO: docs.
#[derive(serde::Serialize)]
pub struct Version {
    /// The short hash of the current commit.
    commit: &'static str,
    /// The commit date in the UTC+0 timezone.
    date: Date,
    semantic: SemanticVersion,
}

#[derive(serde::Serialize)]
struct Date {
    year: u16,
    month: u8,
    day: u8,
}

#[derive(serde::Serialize)]
struct SemanticVersion {
    major: u8,
    minor: u8,
    patch: u8,
    /// A pre-release label, like `dev`.
    pre: Option<&'static str>,
}

impl Constant for Version {
    const NAME: Name = "version";
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "mad.nvim {semantic} ({commit} {date})",
            semantic = self.semantic,
            commit = self.commit,
            date = self.date,
        )
    }
}

impl fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{major}.{minor}.{patch}",
            major = self.major,
            minor = self.minor,
            patch = self.patch,
        )?;
        if let Some(pre) = self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}

impl fmt::Display for Date {
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
