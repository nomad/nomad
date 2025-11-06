use abs_path::path;
use editor::{AgentId, Buffer, Context, Shared};
use fs::File;
use neovim::tests::NeovimExt;
use neovim::{Neovim, oxi};
use real_fs::RealFs;

#[neovim::test]
async fn on_buffer_created_doesnt_fire_for_unnamed_buffers(
    ctx: &mut Context<Neovim>,
) {
    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_buffer_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    // The event should fire when creating a file-backed buffer.
    let tempfile = RealFs::default().tempfile().await.unwrap();
    ctx.command(format!("edit {}", tempfile.path()));
    assert_eq!(num_times_fired.take(), 1);

    // The event shouldn't fire when creating an unnamed buffer.
    ctx.command("enew");
    assert_eq!(num_times_fired.take(), 0);
}

#[neovim::test]
fn on_buffer_created_fires_when_creating_buffer_not_backed_by_a_file(
    ctx: &mut Context<Neovim>,
) {
    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_buffer_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    // The event should fire when creating a named buffer, even if it's not
    // backed by a file.
    ctx.command(format!("edit {}", path!("/foo/bar.txt")));
    assert_eq!(num_times_fired.take(), 1);
}

#[neovim::test]
fn on_buffer_created_fires_when_unnamed_buffer_is_renamed(
    ctx: &mut Context<Neovim>,
) {
    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_buffer_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    // The event shouldn't fire when creating an unnamed buffer.
    ctx.command("enew");
    assert_eq!(num_times_fired.take(), 0);

    // The event should fire if the buffer is given a name.
    ctx.command("file foo.txt");
    assert_eq!(num_times_fired.take(), 1);
}

#[neovim::test]
fn on_buffer_created_doesnt_fire_when_named_buffer_is_renamed(
    ctx: &mut Context<Neovim>,
) {
    ctx.command("edit foo.txt");

    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_buffer_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    // The event shouldn't fire when a named buffer is renamed.
    ctx.command("file bar.txt");
    assert_eq!(num_times_fired.take(), 0);
}

#[neovim::test]
async fn on_buffer_created_fires_when_creating_buffer_via_the_editor_api(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let created_by = Shared::<AgentId>::new(AgentId::UNKNOWN);
    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_buffer_created({
        let created_by = created_by.clone();
        let num_times_fired = num_times_fired.clone();
        move |_, agent_id| {
            created_by.set(agent_id);
            num_times_fired.with_mut(|n| *n += 1);
        }
    });

    let tempfile = RealFs::default().tempfile().await.unwrap();
    ctx.create_buffer(tempfile.path(), agent_id).await.unwrap();

    assert_eq!(created_by.take(), agent_id);
    assert_eq!(num_times_fired.take(), 1);
}

#[neovim::test]
async fn on_buffer_created_doesnt_fire_when_file_is_modified(
    ctx: &mut Context<Neovim>,
) {
    let tempfile = RealFs::default().tempfile().await.unwrap();

    ctx.command(format!("edit {}", tempfile.path()));

    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_buffer_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    std::fs::write(tempfile.path(), "new contents").unwrap();

    ctx.command("checktime");

    assert_eq!(num_times_fired.take(), 0);
}

#[neovim::test]
#[ignore = "fails in CI"]
fn on_buffer_removed_fires_when_named_buffer_is_renamed_to_empty_name(
    ctx: &mut Context<Neovim>,
) {
    ctx.command("edit foo.txt");

    let num_times_fired = Shared::<u8>::new(0);

    let (buffer_id, _handle) = ctx.with_borrowed(|ctx| {
        let mut buffer = ctx.current_buffer().unwrap();

        let handle = buffer.on_removed({
            let num_times_fired = num_times_fired.clone();
            move |_, _| num_times_fired.with_mut(|n| *n += 1)
        });

        (buffer.id(), handle)
    });

    // In our model of an editor, a buffer is always associated with an
    // absolute file path, so giving the buffer an empty name should be the
    // same as removing it.
    oxi::api::Buffer::from(buffer_id).set_name("").unwrap();

    assert_eq!(num_times_fired.take(), 1);
}

#[neovim::test]
fn on_cursor_created_doesnt_fire_when_editing_current_buffer(
    ctx: &mut Context<Neovim>,
) {
    ctx.create_and_focus_scratch_buffer();

    let buffer_path = ctx.with_borrowed(|ctx| {
        ctx.current_buffer().unwrap().path().into_owned()
    });

    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_cursor_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    // ":edit <path>" triggers BufEnter even if the given path is the one of
    // the current buffer, so make sure we guard against that.
    ctx.command(format!("edit {buffer_path}"));
    assert_eq!(num_times_fired.take(), 0);
}

#[neovim::test]
fn on_cursor_created_doesnt_fire_when_splitting_current_buffer(
    ctx: &mut Context<Neovim>,
) {
    ctx.create_and_focus_scratch_buffer();

    let buffer_path = ctx.with_borrowed(|ctx| {
        ctx.current_buffer().unwrap().path().into_owned()
    });

    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_cursor_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    // ":split <path>" triggers BufEnter even if the given path is the one of
    // the current buffer, so make sure we guard against that.
    ctx.command(format!("split {buffer_path}"));
    assert_eq!(num_times_fired.take(), 0);
}

#[neovim::test]
fn on_cursor_created_fires_when_editing_buffer_from_unnamed_buffer(
    ctx: &mut Context<Neovim>,
) {
    ctx.command("enew");

    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_cursor_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    ctx.command(format!("edit {}", path!("/foo/bar.txt")));

    assert_eq!(num_times_fired.take(), 1);
}

#[neovim::test]
async fn on_cursor_created_doesnt_fire_when_creating_buffer_via_the_editor_api(
    ctx: &mut Context<Neovim>,
) {
    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_cursor_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| {
            num_times_fired.with_mut(|n| *n += 1);
        }
    });

    let tempfile = RealFs::default().tempfile().await.unwrap();
    ctx.create_buffer(tempfile.path(), AgentId::UNKNOWN).await.unwrap();
    assert_eq!(num_times_fired.take(), 0);
}

#[neovim::test]
fn on_cursor_moved_fires_when_window_is_split(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.with_borrowed(|ctx| {
        ctx.cursor(buffer_id).unwrap().on_moved({
            let num_times_fired = num_times_fired.clone();
            move |_, _| num_times_fired.with_mut(|n| *n += 1)
        })
    });

    // Each window keeps its own cursor state, so splitting the current window
    // should cause the event to fire.
    ctx.command("split");
    assert_eq!(num_times_fired.take(), 1);
}
