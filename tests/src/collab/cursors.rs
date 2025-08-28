use abs_path::{AbsPathBuf, path};
use collab::event::BufferEvent;
use collab::mock::CollabMock;
use collab::{Peer, PeerId};
use futures_lite::future;
use mock::{EditorExt, Mock};

#[test]
fn remote_peer_tooltip_is_moved_after_integrating_edit() {
    let fs = mock::fs! {
        "foo.txt": "hello world",
    };

    let mut project_1 =
        collab_project::Project::from_mock(PeerId::new(1), fs.root());

    let project_2 = project_1.fork(PeerId::new(2));

    let mut foo = project_1
        .node_at_path_mut(path!("/foo.txt"))
        .unwrap()
        .unwrap_file()
        .unwrap_text();

    // Create a new cursor after the "o".
    let (cursor_id, cursor_creation) = foo.create_cursor(5);

    // Insert a comma at the cursor position.
    let insert_comma = foo.insert(5, ",");

    let fut = CollabMock::new(Mock::new(fs)).run(async move |ctx| {
        let agent_id = ctx.new_agent_id();

        let mut proj = collab::project::Project {
            agent_id,
            id_maps: Default::default(),
            local_peer: Peer {
                id: project_2.peer_id(),
                github_handle: "peer2".parse().unwrap(),
            },
            inner: project_2,
            peer_selections: Default::default(),
            peer_tooltips: Default::default(),
            remote_peers: [Peer {
                id: project_1.peer_id(),
                github_handle: "peer1".parse().unwrap(),
            }]
            .into_iter()
            .collect(),
            root_path: AbsPathBuf::root(),
        };

        let foo_path = path!("/foo.txt");

        // First, let the project know about the buffer or the following events
        // will be ignored.
        proj.synchronize_buffer(BufferEvent::Created(
            ctx.create_buffer(foo_path, agent_id).await.unwrap(),
            foo_path.to_owned(),
        ));

        proj.integrate_cursor_creation(cursor_creation, ctx).await;
        // The tooltip should be after the "o".
        assert_eq!(*proj.peer_tooltips.get(&cursor_id).unwrap(), 5);

        proj.integrate_text_edit(insert_comma, ctx).await;
        // After integrating the insertion, the tooltip should be moved after
        // the comma.
        assert_eq!(*proj.peer_tooltips.get(&cursor_id).unwrap(), 6);
    });

    future::block_on(fut);
}
