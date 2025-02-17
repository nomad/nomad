#![allow(missing_docs)]

use auth::Auth;
use collab2::Collab;
use collab2::backend::test::CollabTestBackend;
use collab2::start::StartError;
use nvimx2::action::AsyncAction;
use nvimx2::tests::{BackendExt, TestBackend};

#[test]
fn cannot_start_session_if_not_logged_in() {
    CollabTestBackend::<TestBackend>::default().block_on(async move |ctx| {
        let collab = Collab::from(&Auth::default());
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::user_not_logged_in());
    });
}

#[test]
fn cannot_start_session_if_no_buffer_is_focused() {
    CollabTestBackend::<TestBackend>::default().block_on(async move |ctx| {
        let collab = Collab::from(&Auth::dummy("foo"));
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::no_buffer_focused());
    });
}
