//! TODO: docs.

mod buf_add;
mod buf_enter;

pub use buf_add::BufAdd;
pub use buf_enter::BufEnter;

/// TODO: docs.
pub trait Event: Sized {
    /// TODO: docs.
    type Ctx<'a>;

    /// TODO: docs.
    fn register(self, ctx: Self::Ctx<'_>);
}
