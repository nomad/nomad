use editor::{Context, Shared};
use fs::File;
use neovim::Neovim;
use neovim::tests::NeovimExt;
use real_fs::RealFs;

#[neovim::test]
async fn on_buffer_created_is_not_triggered_for_nameless_buffers(
    ctx: &mut Context<Neovim>,
) {
    let tempfile = RealFs::default().tempfile().await.unwrap();

    let did_trigger = Shared::<bool>::new(false);

    let _handle = ctx.on_buffer_created({
        let did_trigger = did_trigger.clone();
        move |_, _| did_trigger.set(true)
    });

    // The callback should be triggered when creating a named buffer.
    ctx.command(format!("edit {}", tempfile.path()));
    assert!(did_trigger.take());

    // The callback shouldn't be triggered when creating a nameless buffer.
    ctx.command("enew");
    assert!(!did_trigger.take());
}
