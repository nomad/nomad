/// TODO: docs
#[derive(Debug, Copy, Clone, Default)]
pub struct Cells(u32);

impl From<u32> for Cells {
    #[inline]
    fn from(value: u32) -> Self {
        Self(value)
    }
}
