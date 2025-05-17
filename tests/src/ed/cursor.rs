use core::mem;

use abs_path::path;
use ed::backend::{Backend, Buffer, Cursor, Replacement};
use ed::{Borrowed, ByteOffset, Context, Shared};

pub(crate) fn on_cursor_created_1<Ed: Backend>(
    ctx: &mut Context<Ed, Borrowed>,
) {
    let agent_id = ctx.new_agent_id();

    let num_created = Shared::<usize>::new(0);

    let _handle = ctx.on_cursor_created({
        let num_created = num_created.clone();
        move |_cursor, created_by| {
            assert_eq!(created_by, agent_id);
            num_created.with_mut(|count| *count += 1);
        }
    });

    ctx.block_on(async move |ctx| {
        // Focusing the buffer should create a cursor.
        ctx.create_and_focus(path!("/foo.txt"), agent_id).await.unwrap();
    });

    assert_eq!(num_created.copied(), 1);
}

pub(crate) fn on_cursor_created_2<Ed: Backend>(
    ctx: &mut Context<Ed, Borrowed>,
) {
    let agent_id = ctx.new_agent_id();

    let foo_id = ctx.block_on(async move |ctx| {
        ctx.create_and_focus(path!("/foo.txt"), agent_id).await.unwrap()
    });

    let bar_id = ctx.block_on(async move |ctx| {
        ctx.create_and_focus(path!("/bar.txt"), agent_id).await.unwrap()
    });

    let num_created = Shared::<usize>::new(0);

    let _handle = ctx.on_cursor_created({
        let num_created = num_created.clone();
        move |_cursor, created_by| {
            assert_eq!(created_by, agent_id);
            num_created.with_mut(|count| *count += 1);
        }
    });

    // /bar.txt is currently focused, so focusing it again shouldn't do
    // anything.
    ctx.buffer(bar_id.clone()).unwrap().focus();
    assert_eq!(num_created.copied(), 0);

    // Now focus /foo.txt, which should create a cursor.
    ctx.buffer(foo_id).unwrap().focus();
    assert_eq!(num_created.copied(), 1);

    // Now focus /bar.txt again, which should create a cursor.
    ctx.buffer(bar_id).unwrap().focus();
    assert_eq!(num_created.copied(), 2);
}

pub(crate) fn on_cursor_moved_1<Ed: Backend>(ctx: &mut Context<Ed, Borrowed>) {
    let agent_id = ctx.new_agent_id();

    let num_created = Shared::<usize>::new(0);

    let offsets = Shared::<Vec<ByteOffset>>::default();

    let _handle = ctx.on_cursor_created({
        let num_created = num_created.clone();
        let offsets = offsets.clone();
        move |cursor, created_by| {
            assert_eq!(created_by, agent_id);
            num_created.with_mut(|count| *count += 1);

            let offsets = offsets.clone();
            let handle = cursor.on_moved(move |cursor, moved_by| {
                assert_eq!(moved_by, agent_id);
                offsets.with_mut(|vec| vec.push(cursor.byte_offset()));
            });

            mem::forget(handle);
        }
    });

    let foo_id = ctx.block_on(async move |ctx| {
        ctx.create_and_focus(path!("/foo.txt"), agent_id).await.unwrap()
    });

    assert_eq!(num_created.copied(), 1);

    // We've created a cursor but never moved it, so we shouldn't have pushed
    // any offsets yet.
    assert!(offsets.with(|vec| vec.is_empty()));

    let mut foo_txt = ctx.buffer(foo_id).unwrap();

    foo_txt.edit(
        [Replacement::new(0usize.into()..0usize.into(), "Hello world")],
        agent_id,
    );

    // Editing the buffer should've caused the cursor to move.
    offsets.with(|vec| {
        assert_eq!(vec.len(), 1);
        assert_eq!(*vec.last().unwrap(), "Hello world".len());
    });

    // Moving the cursor shouldn't cause a new one to be created.
    assert_eq!(num_created.copied(), 1);

    assert_eq!(foo_txt.num_cursors(), 1);

    foo_txt.for_each_cursor(|mut cursor| {
        cursor.r#move(5usize.into(), agent_id);

        offsets.with(|vec| {
            assert_eq!(vec.len(), 2);
            assert_eq!(*vec.last().unwrap(), 5usize);
        });
    });
}
