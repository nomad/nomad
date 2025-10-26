use core::fmt;

use editor::module::Constant;

use crate::generated;

/// TODO: docs.
pub const VERSION: Version =
    Version { commit: generated::COMMIT, tag: generated::TAG };

/// TODO: docs.
#[derive(serde::Serialize)]
pub struct Version {
    commit: Commit,
    tag: Option<ReleaseTag>,
}

/// TODO: docs.
#[derive(serde::Serialize)]
pub(crate) struct Commit {
    pub(crate) hash: &'static str,
    /// The commit date in the UTC+0 timezone.
    pub(crate) date: Date,
}

#[derive(serde::Serialize)]
pub(crate) struct Date {
    pub(crate) year: u16,
    pub(crate) month: u8,
    pub(crate) day: u8,
}

#[derive(Copy, Clone, serde::Serialize)]
#[serde(into = "String")]
#[allow(dead_code, reason = "only created when $RELEASE_TAG is set")]
pub(crate) enum ReleaseTag {
    Nightly,
    Stable { year: u16, month: u8, patch: u16 },
}

impl Commit {
    fn short_hash(&self) -> &'static str {
        &self.hash[..7]
    }
}

impl Constant for Version {
    const NAME: &str = "version";
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self { commit, tag } = self;

        f.write_str("Nomad")?;

        if let Some(tag) = tag {
            match tag {
                ReleaseTag::Nightly => f.write_str(" Nightly")?,
                stable @ ReleaseTag::Stable { .. } => write!(f, " {stable}")?,
            }
        }

        write!(f, " ({} {})", commit.short_hash(), commit.date)
    }
}

impl fmt::Display for ReleaseTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Nightly => f.write_str("nightly"),
            Self::Stable { year, month, patch } => {
                write!(f, "{year}.{month:02}.{patch}")
            },
        }
    }
}

impl From<ReleaseTag> for String {
    fn from(value: ReleaseTag) -> Self {
        value.to_string()
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}-{}", self.year, self.month, self.day)
    }
}
