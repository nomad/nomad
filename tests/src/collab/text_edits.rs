use abs_path::{AbsPathBuf, path};
use collab::editors::mock::CollabMock;
use collab::{Peer, PeerHandle, PeerId};
use mock::{EditorExt, Mock};

#[test]
fn integrating_text_edit_moves_remote_peer_tooltip() {
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

    // Create a new cursor after the space.
    let (cursor_id, cursor_creation) = foo.create_cursor(6);

    // Insert a comma after the "o".
    let insert_comma = foo.insert(5, ",");

    CollabMock::new(Mock::new(fs)).block_on(async move |ctx| {
        let agent_id = ctx.new_agent_id();

        let mut proj = collab::project::Project {
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

        // First, let the project know about the buffer or integrating the
        // cursor creation won't cause a tooltip to be created.
        proj.synchronize_buffer_created(
            ctx.create_buffer(foo_path, agent_id).await.unwrap(),
            foo_path,
            ctx,
        );

        proj.integrate_cursor_creation(cursor_creation, ctx);
        // The tooltip should be after the space.
        assert_eq!(*proj.peer_tooltips.get(&cursor_id).unwrap(), 6);

        proj.integrate_text_edit(insert_comma, ctx).await.unwrap();
        // After integrating the insertion, the tooltip should stay after
        // the space.
        assert_eq!(*proj.peer_tooltips.get(&cursor_id).unwrap(), 7);
    });
}

#[test]
fn integrating_text_edit_creates_buffer() {
    let fs = mock::fs! {
        "foo.txt": "hello world",
    };

    let mut project_1 =
        collab_project::Project::from_mock(PeerId::new(1), fs.root());

    let mut foo = project_1
        .node_at_path_mut(path!("/foo.txt"))
        .unwrap()
        .unwrap_file()
        .unwrap_text();

    let foo_id = foo.local_id();

    // Create a cursor in the file before forking the project.
    let (cursor_id, _) = foo.create_cursor(11);

    let project_2 = project_1.fork(PeerId::new(2));

    let edit =
        project_1.file_mut(foo_id).unwrap().unwrap_text().insert(11, "!");

    CollabMock::new(Mock::new(fs)).block_on(async move |ctx| {
        let agent_id = ctx.new_agent_id();

        let mut proj = collab::project::Project {
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

        // Make sure there are no open buffers before integrating the text edit.
        assert_eq!(ctx.buffer_ids().collect::<Vec<_>>(), []);

        // Integrating the text edit should cause a new buffer to be created.
        let buffer_id =
            proj.integrate_text_edit(edit, ctx).await.unwrap().unwrap();
        assert_eq!(ctx.buffer_ids().collect::<Vec<_>>(), [buffer_id]);

        // The buffer should display a tooltip at the end of the file.
        assert_eq!(*proj.peer_tooltips.get(&cursor_id).unwrap(), 11);
    });
}
