use nvimx2::AsyncCtx;
use nvimx2::action::AsyncAction;
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Logout {}

impl Logout {
    /// TODO: docs.
    pub fn new() -> Self {
        Self {}
    }
}

impl<B: Backend> AsyncAction<B> for Logout {
    const NAME: Name = "logout";

    type Args = ();

    async fn call(&mut self, _: Self::Args, _: &mut AsyncCtx<'_, B>) {}
}

impl<B: Backend> ToCompletionFn<B> for Logout {
    fn to_completion_fn(&self) {}
}
