use abs_path::{AbsPathBuf, path};
use collab::editors::mock::CollabMock;
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

        let mut proj = collab::project::Project::<CollabMock<Mock>> {
            agent_id,
            id_maps: Default::default(),
            local_peer: Peer {
                id: project_2.peer_id(),
                handle: PeerHandle::GitHub("peer2".parse().unwrap()),
            },
            inner: project_2,
            peer_selections: Default::default(),
            peer_tooltips: Default::default(),
            remote_peers: [Peer {
                id: project_1.peer_id(),
                handle: PeerHandle::GitHub("peer1".parse().unwrap()),
            }]
            .into_iter()
            .collect(),
            root_path: AbsPathBuf::root(),
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
