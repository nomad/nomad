use abs_path::{AbsPathBuf, path};
use auth::Auth;
use collab::Collab;
use collab::mock::{CollabMock, CollabServer, MockSessionId};
use collab::start::StartError;
use ed::action::AsyncAction;
use futures_lite::future::{self, FutureExt};
use mock::{EditorExt, Mock};

#[test]
fn cannot_start_session_if_not_logged_in() {
    CollabMock::<Mock>::default().block_on(async |ctx| {
        let collab = Collab::from(&Auth::default());
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::UserNotLoggedIn);
    });
}

#[test]
fn cannot_start_session_if_no_buffer_is_focused() {
    CollabMock::<Mock>::default().block_on(async |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer1"));
        let err = collab.start().call((), ctx).await.unwrap_err();
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
        ctx.create_and_focus(path!("/foo.txt"), agent_id).await.unwrap();
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::ProjectRootIsFsRoot);
    });
}

#[test]
#[ignore = "not yet implemented in pando"]
fn cannot_start_session_if_root_overlaps_existing_project() {
    let fs = mock::fs! {
        "a": {
            ".git": {},
            "foo.txt": "",
            "b": {
                ".git": {},
                "bar.txt": "",
            },
        },
    };

    let server = CollabServer::default();

    let editor = CollabMock::new(Mock::new(fs))
        .with_home_dir(AbsPathBuf::root())
        .with_server(&server);

    let run_test = editor.run(async |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer1"));
        let agent_id = ctx.new_agent_id();

        // Start session at "/a/b".
        ctx.create_and_focus(path!("/a/b/bar.txt"), agent_id).await.unwrap();
        collab.start().call((), ctx).await.unwrap();
        let project = collab.project(MockSessionId(1)).unwrap();
        assert_eq!(project.root(), "/a/b");

        // Can't start new session at "/a", it overlaps "/a/b".
        ctx.create_and_focus(path!("/a/foo.txt"), agent_id).await.unwrap();
        let err = match collab.start().call((), ctx).await.unwrap_err() {
            StartError::OverlappingProject(err) => err,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(err.existing_root, "/a/b");
        assert_eq!(err.new_root, "/a");
    });

    future::block_on(run_test.or(server.run()));
}
