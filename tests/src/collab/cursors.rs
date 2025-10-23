use abs_path::{AbsPathBuf, path};
use collab::editors::mock::CollabMock;
use collab::peers::RemotePeers;
use collab::{Peer, PeerHandle, PeerId};
use mock::{EditorExt, Mock};

#[test]
fn remote_peer_tooltip_is_present_when_opening_buffer() {
    let fs = mock::fs! {
        "foo.txt": "hello world",
    };

    let mut project_1 =
        collab_project::Project::from_mock(PeerId::new(1), fs.root());

    let (cursor_id, _) = project_1
        .node_at_path_mut(path!("/foo.txt"))
        .unwrap()
        .unwrap_file()
        .unwrap_text()
        .create_cursor(5);

    let project_2 = project_1.fork(PeerId::new(2));

    CollabMock::new(Mock::new(fs)).block_on(async move |ctx| {
        let agent_id = ctx.new_agent_id();

        let remote_peer = Peer {
            id: project_1.peer_id(),
            handle: PeerHandle::GitHub("peer1".parse().unwrap()),
        };

        let mut proj = collab::project::Project::<CollabMock<Mock>> {
            agent_id,
            id_maps: Default::default(),
            local_peer: Peer {
                id: project_2.peer_id(),
                handle: PeerHandle::GitHub("peer2".parse().unwrap()),
            },
            peer_selections: Default::default(),
            peer_tooltips: Default::default(),
            remote_peers: RemotePeers::new([remote_peer], &project_2),
            root_path: AbsPathBuf::root(),
            inner: project_2,
        };

        let foo_path = path!("/foo.txt");

        // First, let the project synchronize the buffer creation.
        proj.synchronize_buffer_created(
            ctx.create_buffer(foo_path, agent_id).await.unwrap(),
            foo_path,
            ctx,
        );

        // Peer 1 created a cursor at offset 5, so peer 2 should display a
        // tooltip at that offset.
        assert_eq!(*proj.peer_tooltips.get(&cursor_id).unwrap(), 5);
    });
}

#[test]
fn main_cursor_is_removed_when_cursor_deletion_is_received() {
    let fs = mock::fs! {
        "foo.txt": "hello world",
    };

    let mut project_1 =
        collab_project::Project::from_mock(PeerId::new(1), fs.root());

    let (cursor_id, _) = project_1
        .node_at_path_mut(path!("/foo.txt"))
        .unwrap()
        .unwrap_file()
        .unwrap_text()
        .create_cursor(5);

    let project_2 = project_1.fork(PeerId::new(2));

    CollabMock::new(Mock::new(fs)).block_on(async move |ctx| {
        let agent_id = ctx.new_agent_id();

        let remote_peer = Peer {
            id: project_1.peer_id(),
            handle: PeerHandle::GitHub("peer1".parse().unwrap()),
        };

        let mut proj = collab::project::Project::<CollabMock<Mock>> {
            agent_id,
            id_maps: Default::default(),
            local_peer: Peer {
                id: project_2.peer_id(),
                handle: PeerHandle::GitHub("peer2".parse().unwrap()),
            },
            peer_selections: Default::default(),
            peer_tooltips: Default::default(),
            remote_peers: RemotePeers::new([remote_peer], &project_2),
            root_path: AbsPathBuf::root(),
            inner: project_2,
        };

        let cursor_deletion =
            project_1.cursor_mut(cursor_id).unwrap().unwrap().delete();

        proj.integrate_cursor_deletion(cursor_deletion, ctx);

        // The cursor we removed was peer1's main, so it should be None now.
        let peer1_main_cursor = proj
            .remote_peers
            .get(project_1.peer_id())
            .unwrap()
            .main_cursor_id();

        assert_eq!(peer1_main_cursor, None);
    });
}
