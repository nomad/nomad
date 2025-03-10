#![allow(missing_docs)]

use auth::Auth;
use collab2::Collab;
use collab2::backend::test::{CollabTestBackend, CollabTestServer, SessionId};
use collab2::start::StartError;
use futures_lite::future;
use nvimx2::action::AsyncAction;
use nvimx2::fs::AbsPathBuf;
use nvimx2::tests::{self, BackendExt, TestBackend};

#[test]
fn cannot_start_session_if_not_logged_in() {
    CollabTestBackend::<TestBackend>::default().block_on(async |ctx| {
        let collab = Collab::from(&Auth::default());
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::UserNotLoggedIn);
    });
}

#[test]
fn cannot_start_session_if_no_buffer_is_focused() {
    CollabTestBackend::<TestBackend>::default().block_on(async |ctx| {
        let collab = Collab::from(&Auth::dummy("peer-1"));
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::NoBufferFocused);
    });
}

#[test]
fn cannot_start_session_if_project_root_is_fs_root() {
    let fs = tests::fs! {
        "foo.txt": "",
    };

    let backend = CollabTestBackend::new(TestBackend::new(fs))
        .with_home_dir(AbsPathBuf::root());

    backend.block_on(async |ctx| {
        let collab = Collab::from(&Auth::dummy("peer-1"));
        ctx.focus_buffer_at(&path("/foo.txt")).unwrap();
        let err = collab.start().call((), ctx).await.unwrap_err();
        assert_eq!(err, StartError::ProjectRootIsFsRoot);
    });
}

#[test]
fn cannot_start_session_if_root_overlaps_existing_project() {
    let fs = tests::fs! {
        "a": {
            ".git": {},
            "foo.txt": "",
            "b": {
                ".git": {},
                "bar.txt": "",
            },
        },
    };

    let server = CollabTestServer::default();

    let backend = CollabTestBackend::new(TestBackend::new(fs))
        .with_home_dir(AbsPathBuf::root())
        .with_server(&server);

    let run_test = backend.run(async |ctx| {
        let collab = Collab::from(&Auth::dummy("peer-1"));

        // Start session at "/a/b".
        ctx.focus_buffer_at(&path("/a/b/bar.txt")).unwrap();
        collab.start().call((), ctx).await.unwrap();
        let project = collab.project(SessionId(1)).unwrap();
        assert_eq!(project.root(), "/a/b");

        // Can't start new session at "/a", it overlaps "/a/b".
        ctx.focus_buffer_at(&path("/a/foo.txt")).unwrap();
        let err = match collab.start().call((), ctx).await.unwrap_err() {
            StartError::OverlappingProject(err) => err,
            other => panic!("unexpected error: {:?}", other),
        };
        assert_eq!(err.existing_root, "/a/b");
        assert_eq!(err.new_root, "/a");
    });

    future::block_on(future::zip(run_test, server.run()));
}

fn path(path: &str) -> AbsPathBuf {
    path.parse().unwrap()
}
