use abs_path::{AbsPathBuf, path};
use auth::Auth;
use collab::Collab;
use collab::editors::mock::CollabMock;
use collab::start::StartError;
use mock::{EditorExt, Mock};

use crate::editor::ContextExt;

#[test]
fn cannot_start_session_if_not_logged_in() {
    CollabMock::<Mock>::default().block_on(async |ctx| {
        let collab = Collab::from(&Auth::default());
        let err = collab.start(ctx).await.unwrap_err();
        assert_eq!(err, StartError::UserNotLoggedIn);
    });
}

#[test]
fn cannot_start_session_if_no_buffer_is_focused() {
    CollabMock::<Mock>::default().block_on(async |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer1"));
        let err = collab.start(ctx).await.unwrap_err();
        assert_eq!(err, StartError::NoBufferFocused);
    });
}

#[test]
fn cannot_start_session_if_project_root_is_fs_root() {
    let fs = mock::fs! {
        "foo.txt": "",
    };

    let editor =
        CollabMock::new(Mock::new(fs)).with_home_dir(AbsPathBuf::root());

    editor.block_on(async |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer1"));
        let agent_id = ctx.new_agent_id();
        ctx.create_and_focus(path!("/foo.txt"), agent_id).await;
        let err = collab.start(ctx).await.unwrap_err();
        assert_eq!(err, StartError::ProjectRootIsFsRoot);
    });
}
