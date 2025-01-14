use nvimx2::AsyncCtx;
use nvimx2::action::AsyncAction;
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Login {}

impl Login {
    /// TODO: docs.
    pub fn new() -> Self {
        Self {}
    }
}

impl<B: Backend> AsyncAction<B> for Login {
    const NAME: Name = "login";

    type Args = ();

    async fn call(&mut self, _: Self::Args, _: &mut AsyncCtx<'_, B>) {}
}

impl<B: Backend> ToCompletionFn<B> for Login {
    fn to_completion_fn(&self) {}
}
