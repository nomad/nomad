use core::time::Duration;

use editor::{AgentId, Buffer, Context, Edit, Replacement, Shared};
use futures_lite::FutureExt as _;
use futures_util::stream::StreamExt;
use neovim::Neovim;
use neovim::buffer::BufferExt;
use neovim::oxi::api::{self, opts};
use neovim::tests::NeovimExt;

use crate::editor::buffer::EditExt;
use crate::utils::FutureExt as _;

#[neovim::test]
async fn trailing_newline_is_reinserted_after_deleting_it(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "Hello\n");
        let _ = buf.schedule_deletion(5..6, agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::deletion(5..6)]);

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(5, "\n")]);
}

#[neovim::test]
async fn trailing_newline_is_inserted_after_inserting_after_it(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "Hello\n");
        let _ = buf.schedule_insertion(6, "World", agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(6, "World")]);

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(11, "\n")]);
}

#[neovim::test]
async fn inserting_nothing_after_trailing_newline_does_nothing(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "Hello\n");
        let _ = buf.schedule_insertion(6, "", agent_id);
    });

    if let Some(edit) = edit_stream
        .select_next_some()
        .timeout(Duration::from_millis(500))
        .await
    {
        panic!("expected no edits, got {edit:?}");
    }

    let opts = opts::OptionOpts::builder().buf(buffer_id.into()).build();
    assert!(api::get_option_value::<bool>("eol", &opts).unwrap());
    assert!(api::get_option_value::<bool>("fixeol", &opts).unwrap());
}

#[neovim::test]
async fn trailing_newline_is_reinserted_after_replacement_deletes_it(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "Hello\n");
        let _ =
            buf.schedule_replacement(Replacement::new(2..6, "y"), agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::new(2..6, "y")]);

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(3, "\n")]);
}

#[neovim::test]
async fn unsetting_eol_is_like_deleting_trailing_newline(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    // Eol is only relevant in non-empty buffers.
    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    let opts = opts::OptionOpts::builder().buf(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(&*edit.replacements, &[Replacement::deletion(5..6)]);
}

#[neovim::test]
async fn setting_eol_is_like_inserting_trailing_newline(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    // Eol is only relevant in non-empty buffers.
    ctx.feedkeys("iHello");

    let opts = opts::OptionOpts::builder().buf(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    api::set_option_value("eol", true, &opts).unwrap();

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(5, "\n")]);
}

#[neovim::test]
async fn inserting_via_api_in_empty_buf_with_eol_causes_newline_insertion(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        let _ = buf.schedule_insertion(0, "foo", agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(
        &*edit.replacements,
        &[Replacement::insertion(0, "foo"), Replacement::insertion(3, "\n"),]
    );
}

#[neovim::test]
async fn single_insertion_ending_in_newline_in_empty_buf_with_eol_doesnt_cause_extra_newline_insertion(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        let _ = buf.schedule_insertion(0, "foo\n", agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(0, "foo\n")]);

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "foo\n");
    });
}

#[neovim::test]
async fn multiple_insertions_ending_in_newline_in_empty_buf_with_eol_doesnt_cause_extra_newline_insertion(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        let _ = buf.schedule_edit(
            [
                Replacement::insertion(0, "foo"),
                Replacement::insertion(3, "\n"),
            ],
            agent_id,
        );
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(0, "foo\n")]);

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "foo\n");
    });
}

#[neovim::test]
async fn inserting_by_typing_in_empty_buf_with_eol_causes_newline_insertion(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.feedkeys("iH");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(
        &*edit.replacements,
        &[Replacement::insertion(0, "H"), Replacement::insertion(1, "\n")]
    );
}

#[neovim::test]
async fn deleting_up_to_newline_via_api_in_buf_with_eol_causes_newline_deletion(
    ctx: &mut Context<Neovim>,
) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "Hello\n");
        let _ = buf.schedule_deletion(0..5, agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(
        &*edit.replacements,
        &[Replacement::deletion(0..5), Replacement::deletion(0..1)]
    );
}

#[neovim::test]
async fn deleting_up_to_newline_by_typing_in_buf_with_eol_causes_newline_deletion(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.feedkeys("diw");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(
        &*edit.replacements,
        &[Replacement::deletion(0..5), Replacement::deletion(0..1)]
    );
}

#[neovim::test]
async fn delete_all_via_api_in_buf_with_eol(ctx: &mut Context<Neovim>) {
    let agent_id = ctx.new_agent_id();

    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "Hello\n");
        let _ = buf.schedule_deletion(0..6, agent_id);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, agent_id);
    assert_eq!(&*edit.replacements, &[Replacement::deletion(0..6)]);
}

#[neovim::test]
async fn dd_in_last_line_with_no_eol_deletes_trailing_newline(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let opts = opts::OptionOpts::builder().buf(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    ctx.feedkeys("iHello<CR>");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    // The buffer contains "Hello\n" and the cursor is on the 2nd (empty) row.
    //
    // When executing 'dd', the coordinates of the start position given to the
    // `on_bytes` callback are:
    //
    // start_row: 1
    // start_col: 0
    // start_byte: 5
    //
    // which are semantically inconsistent because a byte offset of 5 means
    // that the edit starts between the 'o' and the '\n', while a
    // (start_row, start_col) of (1, 0) means that it starts after the '\n'.
    //
    // Interestingly, deleting the line by pressing backspace in the same
    // situation results in:
    //
    // start_row: 0
    // start_col: 5
    // start_byte: 5
    //
    // which is what I would expect 'dd' to result in as well.
    //
    // Boring semantics aside, let's just make sure we handle this correctly on
    // our side.
    ctx.feedkeys("dd");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(&*edit.replacements, &[Replacement::deletion(5..6)]);
}

/// Makes sure we have a workaround for
/// https://github.com/neovim/neovim/issues/35557.
#[neovim::test]
async fn dd_in_buffer_with_single_line_deletes_whole_buffer(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<Esc>");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.feedkeys("dd");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(&*edit.replacements, &[Replacement::deletion(0..6)]);
}

/// Same as [`dd_in_buffer_with_single_line_deletes_whole_buffer`] but with
/// 'eol' and 'fixeol' unset, so that the buffer doesn't have a trailing
/// newline.
#[neovim::test]
#[ignore = "We don't have a workaround for this, it should be fixed upstream"]
async fn dd_in_buffer_with_single_line_and_no_eol_deletes_whole_buffer(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let opts = opts::OptionOpts::builder().buf(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    ctx.feedkeys("iHello<Esc>");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.feedkeys("dd");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(&*edit.replacements, &[Replacement::deletion(0..5)]);
}

#[neovim::test]
#[allow(non_snake_case)]
async fn dG_from_first_row_deletes_whole_buffer(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello<CR>World<Esc>gg");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.feedkeys("dG");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(&*edit.replacements, &[Replacement::deletion(0..12)]);
}

#[neovim::test]
async fn insert_newline_via_api_in_empty_buf_with_eol(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        let _ = buf.schedule_insertion(0, "\n", AgentId::UNKNOWN);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(
        &*edit.replacements,
        &[Replacement::insertion(0, "\n"), Replacement::insertion(1, "\n")]
    );

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "\n\n");
    });
}

#[neovim::test]
async fn insert_newline_by_typing_in_empty_buf_with_eol(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.feedkeys("i<CR>");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(
        &*edit.replacements,
        &[Replacement::insertion(0, "\n"), Replacement::insertion(1, "\n")]
    );

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "\n\n");
    });
}

#[neovim::test]
async fn insert_newline_in_empty_buf_with_no_eol(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let opts = opts::OptionOpts::builder().buf(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.with_borrowed(|ctx| {
        let mut buf = ctx.buffer(buffer_id).unwrap();
        let _ = buf.schedule_insertion(0, "\n", AgentId::UNKNOWN);
    });

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(&*edit.replacements, &[Replacement::insertion(0, "\n")]);

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "\n");
    });
}

#[neovim::test]
fn grapheme_offsets_empty(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut offsets = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .grapheme_offsets()
            .collect::<Vec<_>>()
            .into_iter()
    });

    assert_eq!(offsets.next(), None);
}

#[neovim::test]
fn grapheme_offsets_ascii(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut offsets = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .grapheme_offsets()
            .collect::<Vec<_>>()
            .into_iter()
    });

    assert_eq!(offsets.next().unwrap(), 1);
    assert_eq!(offsets.next().unwrap(), 2);
    assert_eq!(offsets.next().unwrap(), 3);
    assert_eq!(offsets.next().unwrap(), 4);
    assert_eq!(offsets.next().unwrap(), 5);
    assert_eq!(offsets.next().unwrap(), 6);
    assert_eq!(offsets.next(), None);
}

#[neovim::test]
fn grapheme_offsets_multiline(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("ifoo<CR>bar");

    let mut offsets = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .grapheme_offsets()
            .collect::<Vec<_>>()
            .into_iter()
    });

    assert_eq!(offsets.next().unwrap(), 1);
    assert_eq!(offsets.next().unwrap(), 2);
    assert_eq!(offsets.next().unwrap(), 3);
    assert_eq!(offsets.next().unwrap(), 4);
    assert_eq!(offsets.next().unwrap(), 5);
    assert_eq!(offsets.next().unwrap(), 6);
    assert_eq!(offsets.next().unwrap(), 7);
    assert_eq!(offsets.next().unwrap(), 8);
    assert_eq!(offsets.next(), None);
}

#[neovim::test]
fn grapheme_offsets_multibyte_chars(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("i🦆🦀🐎🦖🐤");

    let mut offsets = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .grapheme_offsets()
            .collect::<Vec<_>>()
            .into_iter()
    });

    assert_eq!(offsets.next().unwrap(), 4);
    assert_eq!(offsets.next().unwrap(), 8);
    assert_eq!(offsets.next().unwrap(), 12);
    assert_eq!(offsets.next().unwrap(), 16);
    assert_eq!(offsets.next().unwrap(), 20);
    assert_eq!(offsets.next().unwrap(), 21);
    assert_eq!(offsets.next(), None);
}

#[neovim::test]
fn grapheme_offsets_multichar_graphemes(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    // Polar bear is 13 bytes, scientist is 11.
    ctx.feedkeys("i🐻‍❄️🧑‍🔬");

    let mut offsets = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .grapheme_offsets()
            .collect::<Vec<_>>()
            .into_iter()
    });

    assert_eq!(offsets.next().unwrap(), 13);
    assert_eq!(offsets.next().unwrap(), 24);
    assert_eq!(offsets.next().unwrap(), 25);
    assert_eq!(offsets.next(), None);
}

#[neovim::test]
fn grapheme_offsets_start_from_middle_of_grapheme(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    // Polar bear is 13 bytes, scientist is 11.
    ctx.feedkeys("i🐻‍❄️🧑‍🔬");

    let mut offsets = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            // Start between the ZWJ and the snowflake emoji.
            .grapheme_offsets_from(7)
            .collect::<Vec<_>>()
            .into_iter()
    });

    assert_eq!(offsets.next().unwrap(), 13);
    assert_eq!(offsets.next().unwrap(), 24);
    assert_eq!(offsets.next().unwrap(), 25);
    assert_eq!(offsets.next(), None);
}

#[neovim::test]
fn highlight_ranges_empty(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let mut ranges = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .highlight_ranges()
            .collect::<Vec<_>>()
            .into_iter()
    });

    assert_eq!(ranges.next(), None);
}

#[neovim::test]
#[ignore = "nvim_buf_get_extmarks() doesn't include ephemeral extmarks"]
fn highlight_range_simple(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let _hl_range_handle = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id).unwrap().highlight_range(0..5, "Normal")
    });

    ctx.redraw();

    let mut ranges = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .highlight_ranges()
            .collect::<Vec<_>>()
            .into_iter()
    });

    let (byte_range, hl_groups) = ranges.next().unwrap();
    assert_eq!(byte_range, 0..5);
    assert_eq!(&*hl_groups, &["Normal"]);

    assert_eq!(ranges.next(), None);
}

#[neovim::test]
#[ignore = "nvim_buf_get_extmarks() doesn't include ephemeral extmarks"]
fn highlight_range_including_eol(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let _hl_range_handle = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id).unwrap().highlight_range(0..6, "Normal")
    });

    ctx.redraw();

    let mut ranges = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id)
            .unwrap()
            .highlight_ranges()
            .collect::<Vec<_>>()
            .into_iter()
    });

    let (byte_range, hl_groups) = ranges.next().unwrap();
    assert_eq!(byte_range, 0..6);
    assert_eq!(&*hl_groups, &["Normal"]);

    assert_eq!(ranges.next(), None);
}

#[neovim::test]
#[ignore = "nvim_buf_get_extmarks() doesn't include ephemeral extmarks"]
fn highlight_range_is_removed_when_handle_is_dropped(
    ctx: &mut Context<Neovim>,
) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let hl_range_handle = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id).unwrap().highlight_range(0..5, "Normal")
    });

    ctx.redraw();

    ctx.with_borrowed(|ctx| {
        let num_ranges =
            ctx.buffer(buffer_id).unwrap().highlight_ranges().count();

        assert_eq!(num_ranges, 1);
    });

    drop(hl_range_handle);

    ctx.redraw();

    ctx.with_borrowed(|ctx| {
        let num_ranges =
            ctx.buffer(buffer_id).unwrap().highlight_ranges().count();

        assert_eq!(num_ranges, 0);
    });
}

#[neovim::test]
fn num_bytes_in_line_after_trailine_newline(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.get_text(), "Hello\n");
        assert_eq!(buf.num_bytes_in_line_after(1), 0);
    });
}

#[neovim::test]
fn empty_buffer_with_fixeol_is_empty(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.byte_len(), 0);
        assert_eq!(buf.get_text(), "");
    });
}

#[neovim::test]
fn empty_buffer_with_no_fixeol_is_empty(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let opts = opts::OptionOpts::builder().buf(buffer_id.into()).build();
    api::set_option_value("eol", false, &opts).unwrap();
    api::set_option_value("fixeol", false, &opts).unwrap();

    ctx.with_borrowed(|ctx| {
        let buf = ctx.buffer(buffer_id).unwrap();
        assert_eq!(buf.byte_len(), 0);
        assert_eq!(buf.get_text(), "");
    });
}

#[neovim::test]
async fn save_buffer_via_api(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let saved_by = Shared::<Option<AgentId>>::new(None);

    let _event_handle = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id).unwrap().on_saved({
            let saved_by = saved_by.clone();
            move |_, agent_id| saved_by.set(Some(agent_id))
        })
    });

    let agent_id = ctx.new_agent_id();

    ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id).unwrap().schedule_save(agent_id).boxed_local()
    })
    .await
    .unwrap();

    assert_eq!(saved_by.copied(), Some(agent_id));
}

#[neovim::test]
async fn save_buffer_via_write(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    let saved_by = Shared::<Option<AgentId>>::new(None);

    let _event_handle = ctx.with_borrowed(|ctx| {
        ctx.buffer(buffer_id).unwrap().on_saved({
            let saved_by = saved_by.clone();
            move |_, agent_id| saved_by.set(Some(agent_id))
        })
    });

    ctx.command("write");

    assert_eq!(saved_by.copied(), Some(AgentId::UNKNOWN));
}

#[neovim::test]
#[ignore = "https://github.com/neovim/neovim/issues/36370"]
async fn search_and_replace(ctx: &mut Context<Neovim>) {
    let buffer_id = ctx.create_and_focus_scratch_buffer();

    ctx.feedkeys("iHello");

    let mut edit_stream = Edit::new_stream(buffer_id, ctx);

    ctx.command("s/llo/y");

    let edit = edit_stream.next().await.unwrap();
    assert_eq!(edit.made_by, AgentId::UNKNOWN);
    assert_eq!(&*edit.replacements, &[Replacement::new(2..5, "y")]);
}

mod ed_buffer {
    //! Contains the editor-agnostic buffer tests.

    use super::*;
    use crate::editor::buffer;

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
