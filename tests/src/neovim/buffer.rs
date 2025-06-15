use core::time::Duration;

use ed::{Buffer, Context, Edit, Replacement};
use futures_util::future::FutureExt;
use futures_util::select_biased;
use futures_util::stream::StreamExt;
use neovim::Neovim;
use neovim::buffer::BufferId;
use neovim::oxi::api::{self, opts};
use neovim::tests::ContextExt;

use crate::ed::buffer::EditExt;

#[neovim::test]
async fn deleting_trailing_newline_is_like_unsetting_eol(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    ctx.feedkeys("iHello");

    let buffer_id = BufferId::of_focused();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(0..buf.byte_len()), "Hello\n");
        buf.edit([Replacement::removal(0..6)], agent_id);
    });

    let edit = edit_stream.next().await.unwrap();

    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::removal(0..6)]);

    let opts = opts::OptionOpts::builder().buffer(buffer_id.into()).build();
    assert!(!api::get_option_value::<bool>("eol", &opts).unwrap());
    assert!(!api::get_option_value::<bool>("fixeol", &opts).unwrap());
}

#[neovim::test]
async fn inserting_after_trailing_newline_unsets_eol(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    ctx.feedkeys("iHello");

    let buffer_id = BufferId::of_focused();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(0..buf.byte_len()), "Hello\n");
        buf.edit([Replacement::insertion(6, "World")], agent_id);
    });

    let edit = edit_stream.next().await.unwrap();

    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(6, "World")]);

    let opts = opts::OptionOpts::builder().buffer(buffer_id.into()).build();
    assert!(!api::get_option_value::<bool>("eol", &opts).unwrap());
    assert!(!api::get_option_value::<bool>("fixeol", &opts).unwrap());
}

#[neovim::test]
async fn inserting_nothing_after_trailing_newline_does_nothing(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    ctx.feedkeys("iHello");

    let buffer_id = BufferId::of_focused();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(0..buf.byte_len()), "Hello\n");
        buf.edit([Replacement::insertion(6, "")], agent_id);
    });

    let sleep = async_io::Timer::after(Duration::from_millis(500));
    select_biased! {
        edit = edit_stream.select_next_some() => {
            panic!("expected no edits, got {edit:?}");
        },
        _now = FutureExt::fuse(sleep) => {},
    }

    let opts = opts::OptionOpts::builder().buffer(buffer_id.into()).build();
    assert!(api::get_option_value::<bool>("eol", &opts).unwrap());
    assert!(api::get_option_value::<bool>("fixeol", &opts).unwrap());
}

#[neovim::test]
async fn replacement_including_trailing_newline_unsets_eol(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    ctx.feedkeys("iHello");

    let buffer_id = BufferId::of_focused();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(0..buf.byte_len()), "Hello\n");
        buf.edit([Replacement::new(2..6, "y")], agent_id);
    });

    let edit = edit_stream.next().await.unwrap();

    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::new(2..6, "y")]);

    let opts = opts::OptionOpts::builder().buffer(buffer_id.into()).build();
    assert!(!api::get_option_value::<bool>("eol", &opts).unwrap());
    assert!(!api::get_option_value::<bool>("fixeol", &opts).unwrap());
}

#[neovim::test]
async fn unsetting_eol_is_like_deleting_trailing_newline(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = BufferId::of_focused();

    // Eol is only relevant in non-empty buffers.
    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    let opts = opts::OptionOpts::builder().buffer(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    let edit = edit_stream.next().await.unwrap();

    assert!(edit.made_by.is_unknown());
    assert_eq!(&*edit.replacements, &[Replacement::removal(5..6)]);
}

#[neovim::test]
async fn setting_eol_is_like_inserting_trailing_newline(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = BufferId::of_focused();

    // Eol is only relevant in non-empty buffers.
    ctx.feedkeys("iHello");

    let opts = opts::OptionOpts::builder().buffer(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    api::set_option_value("eol", true, &opts).unwrap();

    let edit = edit_stream.next().await.unwrap();

    assert!(edit.made_by.is_unknown());
    assert_eq!(&*edit.replacements, &[Replacement::insertion(5, "\n")]);
}

#[neovim::test]
async fn inserting_in_empty_buf_with_eol_causes_newline_insertion(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = BufferId::of_focused();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        buf.edit([Replacement::insertion(0, "foo")], agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(0, "foo")]);

    let edit = edit_stream.next().await.unwrap();
    assert!(edit.made_by.is_unknown());
    assert_eq!(&*edit.replacements, &[Replacement::insertion(3, "\n")]);
}

#[neovim::test]
async fn deleting_all_in_buf_with_eol_causes_newline_deletion(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    ctx.feedkeys("iHello");

    let buffer_id = BufferId::of_focused();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(0..buf.byte_len()), "Hello\n");
        buf.edit([Replacement::removal(0..5)], agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::removal(0..5)]);

    let edit = edit_stream.next().await.unwrap();
    assert!(edit.made_by.is_unknown());
    assert_eq!(&*edit.replacements, &[Replacement::removal(0..1)]);
}

mod ed_buffer {
    //! Contains the editor-agnostic buffer tests.

    use super::*;
    use crate::ed::buffer;

    #[neovim::test]
    async fn fuzz_edits_10e1(ctx: &mut Context<Neovim>) {
        buffer::fuzz_edits(10, ctx).await;
    }

    #[neovim::test]
    async fn fuzz_edits_10e2(ctx: &mut Context<Neovim>) {
        buffer::fuzz_edits(100, ctx).await;
    }

    #[neovim::test]
    async fn fuzz_edits_10e3(ctx: &mut Context<Neovim>) {
        buffer::fuzz_edits(1_000, ctx).await;
    }

    #[neovim::test]
    async fn fuzz_edits_10e4(ctx: &mut Context<Neovim>) {
        buffer::fuzz_edits(10_000, ctx).await;
    }
}
