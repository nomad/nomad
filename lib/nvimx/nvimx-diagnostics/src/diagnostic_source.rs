use core::fmt;

use smol_str::SmolStr;

/// TODO: docs.
#[derive(Default)]
pub struct DiagnosticSource {
    segments: Vec<SmolStr>,
}

impl DiagnosticSource {
    /// TODO: docs.
    pub fn new() -> Self {
        Self { segments: Vec::new() }
    }

    /// TODO: docs.
    pub fn push_segment(&mut self, segment: &str) -> &mut Self {
        self.segments.push(SmolStr::new(segment));
        self
    }
}

impl fmt::Display for DiagnosticSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        write!(f, "[{}", "mad")?;

        if !self.segments.is_empty() {
            f.write_char('.')?;
        }

        for (idx, segment) in self.segments.iter().enumerate() {
            f.write_str(segment)?;
            let is_last = idx + 1 == self.segments.len();
            if !is_last {
                f.write_char('.')?;
            }
        }

        f.write_char(']')
    }
}
