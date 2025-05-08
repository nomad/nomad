use abs_path::{AbsPathBuf, path};
use auth::Auth;
use collab::Collab;
use collab::mock::{CollabMock, CollabServer, SessionId};
use ed::action::AsyncAction;
use ed::fs::Fs;
use futures_lite::future::{self, FutureExt};
use mock::{BackendExt, Mock};

#[test]
fn replicate_simple_project() {
    let fs1 = mock::fs! {
        "foo": {
            "world.txt": "Hello, world!",
            "mars.txt": "Hello, mars!",
        },
    };

    let server = CollabServer::default();

    let peer1 = CollabMock::new(Mock::new(fs1.clone()))
        .with_home_dir(AbsPathBuf::root())
        .with_server(&server);

    let peer2 = CollabMock::<Mock>::default()
        .with_default_dir_for_remote_projects(path!("/remote"))
        .with_server(&server);

    let (started_tx, started_rx) = flume::bounded(1);

    let run_peer1 = peer1.run_all(async move |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer1"));
        ctx.focus_buffer_at(path!("/foo/mars.txt")).unwrap();
        collab.start().call((), ctx).await.unwrap();
        started_tx.send(SessionId(1)).unwrap();
    });

    let run_peer2 = peer2.run(async move |ctx| {
        let collab = Collab::from(&Auth::logged_in("peer2"));
        let session_id = started_rx.recv_async().await.unwrap();
        collab.join().call(session_id, ctx).await.unwrap();
        let fs2 = ctx.fs();
        assert_eq!(
            fs1.node_at_path(path!("/foo")).await.unwrap().unwrap(),
            fs2.node_at_path(path!("/remote/foo")).await.unwrap().unwrap(),
        );
    });

    future::block_on(run_peer1.or(run_peer2).or(server.run()));
}
