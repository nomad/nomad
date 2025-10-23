use abs_path::path;
use collab_project::{PeerId, Project};

#[test]
fn integrate_deletions_in_opposite_order() {
    let fs = mock::fs! {
        "foo.txt": "hello world",
    };

    let mut proj_1 = Project::from_mock(PeerId::new(1), fs.root());

    let mut proj_2 = proj_1.fork(PeerId::new(2));

    let mut foo_txt_1 = proj_1
        .node_at_path_mut(path!("/foo.txt"))
        .unwrap()
        .unwrap_file()
        .unwrap_text();

    let delete_d = foo_txt_1.delete(10..11);
    let delete_l = foo_txt_1.delete(9..10);
    let delete_r = foo_txt_1.delete(8..9);

    proj_2.integrate_text_edit(delete_r).unwrap();
    proj_2.integrate_text_edit(delete_l).unwrap();
    proj_2.integrate_text_edit(delete_d).unwrap();

    let foo_txt_2 = proj_2
        .node_at_path(path!("/foo.txt"))
        .unwrap()
        .unwrap_file()
        .unwrap_text();

    assert_eq!(foo_txt_2.contents(), "hello wo");
}
