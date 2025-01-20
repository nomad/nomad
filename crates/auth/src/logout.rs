use nvimx2::action::AsyncAction;
use nvimx2::backend::Backend;
use nvimx2::command::ToCompletionFn;
use nvimx2::notify::Name;
use nvimx2::{AsyncCtx, Shared};

use crate::{Auth, AuthInfos};

/// TODO: docs.
#[derive(Clone, Default)]
pub struct Logout {
    infos: Shared<Option<AuthInfos>>,
}

impl<B: Backend> AsyncAction<B> for Logout {
    const NAME: Name = "logout";

    type Args = ();

    async fn call(&mut self, _: Self::Args, _: &mut AsyncCtx<'_, B>) {
        self.infos.set(None);
    }
}

impl<B: Backend> ToCompletionFn<B> for Logout {
    fn to_completion_fn(&self) {}
}

impl From<&Auth> for Logout {
    fn from(auth: &Auth) -> Self {
        Self { infos: auth.infos().clone() }
    }
}
