#![allow(missing_docs)]

use core::ops;

use auth_types::AuthInfos;
use editor::{Borrowed, Context, Editor, EditorAdapter};

use crate::{AuthEditor, login, logout};

pub struct AuthMock<Ed> {
    inner: Ed,
}

impl<Ed> AuthMock<Ed> {
    pub fn new(inner: Ed) -> Self {
        Self { inner }
    }
}

impl<Ed: Editor> AuthEditor for AuthMock<Ed> {
    type LoginError = core::convert::Infallible;

    #[allow(clippy::manual_async_fn)]
    fn credential_builder(
        _: &mut Context<Self, Borrowed>,
    ) -> impl Future<Output = Box<keyring::CredentialBuilder>> + Send + 'static
    {
        async move { todo!() }
    }

    async fn login(
        _: &mut Context<Self>,
    ) -> Result<AuthInfos, Self::LoginError> {
        todo!()
    }

    fn on_login_error(_: login::LoginError<Self>, _: &mut Context<Self>) {
        unimplemented!()
    }

    fn on_logout_error(_: logout::LogoutError, _: &mut Context<Self>) {
        unimplemented!()
    }
}

impl<Ed> ops::Deref for AuthMock<Ed> {
    type Target = Ed;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<Ed> ops::DerefMut for AuthMock<Ed> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<Ed: Editor> EditorAdapter for AuthMock<Ed> {}
