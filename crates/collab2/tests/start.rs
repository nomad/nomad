#![allow(missing_docs)]

use auth::Auth;
use collab_server::SessionId;
use collab2::Collab;
use collab2::backend::test::CollabTestBackend;
use collab2::start::StartError;
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
        let collab = Collab::from(&Auth::dummy("foo"));
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
        .home_dir(AbsPathBuf::root());

    backend.block_on(async |ctx| {
        let collab = Collab::from(&Auth::dummy("foo"));

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
                "bar.txt": "",
            },
        },
    };

    let backend = CollabTestBackend::new(TestBackend::new(fs))
        .home_dir(AbsPathBuf::root())
        .start_session_with::<core::convert::Infallible>(|_args| todo!());

    backend.block_on(async |ctx| {
        let collab = Collab::from(&Auth::dummy("foo"));

        ctx.focus_buffer_at(&path("/a/foo.txt")).unwrap();
        collab.start().call((), ctx).await.unwrap();
        let project = collab.project(session_id(1)).unwrap();
        assert_eq!(project.root(), "/a");

        ctx.focus_buffer_at(&path("/a/b/bar.txt")).unwrap();
        let err = match collab.start().call((), ctx).await.unwrap_err() {
            StartError::OverlappingProject(err) => err,
            other => panic!("unexpected error: {:?}", other),
        };
        assert_eq!(err.existing_root, "/a");
        assert_eq!(err.new_root, "/a");
    });
}

fn path(path: &str) -> AbsPathBuf {
    path.parse().unwrap()
}

fn session_id(id: u64) -> SessionId {
    SessionId::from_parts(id, 0)
}
