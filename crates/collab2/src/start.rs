use nvimx2::action::AsyncAction;
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared};

use crate::Collab;
use crate::config::Config;

/// The [`Action`] used to start a new collaborative editing session.
#[derive(Clone)]
pub struct Start {
    _config: Shared<Config>,
}

impl<B: Backend> AsyncAction<B> for Start {
    const NAME: Name = "start";

    type Args = ();

    async fn call(&mut self, _: Self::Args, _: &mut AsyncCtx<'_, B>) {
        todo!()
    }
}

impl<B: Backend> ToCompletionFn<B> for Start {
    fn to_completion_fn(&self) {}
}

impl From<&Collab> for Start {
    fn from(collab: &Collab) -> Self {
        Self { _config: collab.config.clone() }
    }
}
