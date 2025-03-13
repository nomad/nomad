/// TODO: docs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NotificationId(u64);

impl NotificationId {
    /// TODO: docs.
    pub const fn into_u64(self) -> u64 {
        self.0
    }

    /// TODO: docs.
    pub const fn new(id: u64) -> Self {
        Self(id)
    }
}
