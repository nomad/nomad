/// TODO: docs
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct Color {
    hex: &'static str,
}

impl Color {
    /// TODO: docs
    #[inline]
    pub fn as_hex_str(&self) -> &str {
        self.hex
    }

    /// TODO: docs
    #[doc(hidden)]
    pub const fn new(hex: &'static str) -> Self {
        Self { hex }
    }
}
