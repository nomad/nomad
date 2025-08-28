use abs_path::path;
use editor::{Buffer, Context, Shared};
use fs::File;
use neovim::tests::NeovimExt;
use neovim::{Neovim, oxi};
use real_fs::RealFs;

#[neovim::test]
async fn on_buffer_created_doesnt_fire_for_nameless_buffers(
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

    // The event shouldn't fire when creating a nameless buffer.
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
fn on_buffer_created_fires_when_nameless_buffer_is_renamed(
    ctx: &mut Context<Neovim>,
) {
    let num_times_fired = Shared::<u8>::new(0);

    let _handle = ctx.on_buffer_created({
        let num_times_fired = num_times_fired.clone();
        move |_, _| num_times_fired.with_mut(|n| *n += 1)
    });

    // The event shouldn't fire when creating a nameless buffer.
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
    // absolute file path, so giving the buffer an empty name should
    // be the same as removing it.
    oxi::api::Buffer::from(buffer_id).set_name("").unwrap();

    assert_eq!(num_times_fired.take(), 1);
}

#[neovim::test]
fn on_cursor_created_doesnt_fire_when_editing_current_buffer(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

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
    let buffer_id = ctx.create_and_focus_scratch_buffer();

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
