use abs_path::{AbsPathBuf, path};
use auth::Auth;
use collab::Collab;
use collab::mock::{CollabMock, CollabServer, SessionId};
use collab::start::StartError;
use ed::action::AsyncAction;
use futures_lite::future::{self, FutureExt};
use mock::{BackendExt, Mock};

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

    let backend =
        CollabMock::new(Mock::new(fs)).with_home_dir(AbsPathBuf::root());

    backend.block_on(async |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer1"));
        ctx.focus_buffer_at(path!("/foo.txt")).unwrap();
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::ProjectRootIsFsRoot);
    });
}

#[test]
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

    let backend = CollabMock::new(Mock::new(fs))
        .with_home_dir(AbsPathBuf::root())
        .with_server(&server);

    let run_test = backend.run(async |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer1"));

        // Start session at "/a/b".
        ctx.focus_buffer_at(path!("/a/b/bar.txt")).unwrap();
        collab.start().call((), ctx).await.unwrap();
        let project = collab.project(SessionId(1)).unwrap();
        assert_eq!(project.root(), "/a/b");

        // Can't start new session at "/a", it overlaps "/a/b".
        ctx.focus_buffer_at(path!("/a/foo.txt")).unwrap();
        let err = match collab.start().call((), ctx).await.unwrap_err() {
            StartError::OverlappingProject(err) => err,
            other => panic!("unexpected error: {other:?}"),
        };
        assert_eq!(err.existing_root, "/a/b");
        assert_eq!(err.new_root, "/a");
    });

    future::block_on(run_test.or(server.run()));
}
