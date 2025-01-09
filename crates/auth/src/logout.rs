use nvimx2::action::AsyncAction;
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Plugin};

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Logout {}

impl Logout {
    /// TODO: docs.
    pub fn new() -> Self {
        Self {}
    }
}

impl<P, B> AsyncAction<P, B> for Logout
where
    P: Plugin<B>,
    B: Backend,
{
    const NAME: Name = "logout";

    type Args = ();

    async fn call(&mut self, _: Self::Args, _: &mut AsyncCtx<'_, P, B>) {}
}

impl<B: Backend> ToCompletionFn<B> for Logout {
    fn to_completion_fn(&self) {}
}
