use std::cmp::Ordering;
use std::ops::Range;

use common::{nvim, WindowConfig, *};
use nvim::api::{opts::*, types::*, Buffer, Window};

use crate::*;

type OnBytesArgs = (
    String,
    Buffer,
    u32,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
    usize,
);

#[derive(Default)]
pub(crate) struct PromptConfig {
    /// A placeholder text to display when the prompt is empty.
    pub placeholder_text: Option<String>,

    /// The size of the result space over which the prompt query is matched.
    /// This remains constant between [`Prompt::open`] calls and is displayed
    /// at the end of the prompt together with the current number of matched
    /// results.
    pub total_results: u64,
}

/// TODO: docs
pub(crate) struct Prompt {
    /// The number of results that match the current prompt. This is updated
    /// as the user types and it's displayed at the end of the prompt together
    /// with the total number of results.
    matched_results: u64,

    /// A sender used to send [`Message::PromptChanged`] messages to the parent
    /// plugin when the prompt changes.
    sender: Sender<Message>,

    /// The current configuration of the prompt, which changes every time the
    /// prompt is opened.
    config: PromptConfig,

    /// The buffer used to display the prompt.
    buffer: Buffer,

    /// The window that houses the buffer. This is only set when the prompt is
    /// open.
    window: Option<Window>,

    /// TODO: docs.
    namespace_id: u32,

    /// TODO: docs.
    placeholder_extmark_id: Option<u32>,

    /// TODO: docs.
    matched_on_total_extmark_id: Option<u32>,
}

impl Prompt {
    /// TODO: docs
    pub fn close(&mut self) {
        if let Some(window) = self.window.take() {
            // This fails if the window is already closed.
            let _ = window.close(true);
        }

        self.update_placeholder("");
    }

    /// TODO: docs
    pub fn open(
        &mut self,
        config: PromptConfig,
        window_config: &WindowConfig,
    ) {
        if let Some(placeholder) = config.placeholder_text.as_ref() {
            self.update_placeholder(placeholder);
        }

        self.update_matched_on_total(
            config.total_results,
            config.total_results,
        );

        let window =
            nvim::api::open_win(&self.buffer, true, &window_config.into())
                .unwrap();

        nvim::api::command("startinsert").unwrap();

        self.matched_results = config.total_results;

        self.window = Some(window);

        self.config = config;
    }

    /// Initializes the prompt.
    ///
    /// TODO: docs.
    pub fn new(sender: Sender<Message>) -> Self {
        let mut buffer = nvim::api::create_buf(false, true).unwrap();

        // Neovim 0.9 has a bug that causes `nvim_buf_get_offset` to return -1
        // on newly created buffers. To work around this, we set its first line
        // to an empty string and immediately delete it, which seems to fix it.
        //
        // See [1] and [2] for more infos.
        //
        // [1]: https://github.com/neovim/neovim/issues/25390
        // [2]: https://github.com/neovim/neovim/issues/24930
        #[cfg(feature = "neovim-0-9")]
        {
            buffer
                .set_lines(0..1, true, std::iter::once(nvim::String::from("")))
                .unwrap();

            buffer
                .set_lines(.., true, std::iter::empty::<nvim::String>())
                .unwrap();
        }

        buffer
            .attach(
                false,
                &BufAttachOpts::builder()
                    .on_bytes(on_bytes(sender.clone()))
                    .build(),
            )
            .unwrap();

        Self {
            matched_results: 0,
            sender,
            config: PromptConfig::default(),
            buffer,
            window: None,
            // Create an anonymous namespace for the prompt.
            namespace_id: nvim::api::create_namespace(""),
            placeholder_extmark_id: None,
            matched_on_total_extmark_id: None,
        }
    }

    /// TODO: docs
    pub fn remove_placeholder(&mut self) {
        if let Some(old_extmark) = self.placeholder_extmark_id {
            self.buffer.del_extmark(self.namespace_id, old_extmark).unwrap();
        }

        self.placeholder_extmark_id = None;
    }

    /// TODO: docs
    pub fn show_placeholder(&mut self) {
        if let Some(placeholder) = self.config.placeholder_text.as_ref() {
            self.update_placeholder(placeholder.clone().as_ref());
        }
    }

    /// TODO: docs
    pub fn update_matched(&mut self, new_matched_results: u64) {
        assert!(new_matched_results <= self.config.total_results);

        self.matched_results = new_matched_results;

        self.update_matched_on_total(
            self.matched_results,
            self.config.total_results,
        );
    }

    /// TODO: docs
    fn update_matched_on_total(&mut self, new_matched: u64, new_total: u64) {
        if let Some(old_extmark) = self.matched_on_total_extmark_id {
            self.buffer.del_extmark(self.namespace_id, old_extmark).unwrap();
        }

        let new_matched_on_total =
            format_matched_on_total(new_matched, new_total);

        let new_extmark = self
            .buffer
            .set_extmark(
                self.namespace_id,
                0,
                0,
                &SetExtmarkOpts::builder()
                    .virt_text([(
                        new_matched_on_total,
                        highlights::PROMPT_MATCHED_ON_TOTAL,
                    )])
                    .virt_text_pos(ExtmarkVirtTextPosition::RightAlign)
                    .build(),
            )
            .unwrap();

        self.matched_on_total_extmark_id = Some(new_extmark);
    }

    /// TODO: docs
    fn update_placeholder(&mut self, new_placeholder: &str) {
        self.remove_placeholder();

        let new_extmark = self
            .buffer
            .set_extmark(
                self.namespace_id,
                0,
                0,
                &SetExtmarkOpts::builder()
                    .virt_text([(
                        new_placeholder,
                        highlights::PROMPT_PLACEHOLDER,
                    )])
                    .virt_text_pos(ExtmarkVirtTextPosition::Overlay)
                    .build(),
            )
            .unwrap();

        self.placeholder_extmark_id = Some(new_extmark);
    }

    /// TODO: docs
    pub fn update_total(&mut self, new_total_results: u64) {
        assert!(new_total_results >= self.matched_results);

        self.config.total_results = new_total_results;

        self.update_matched_on_total(
            self.matched_results,
            self.config.total_results,
        );
    }
}

/// TODO: docs
fn format_matched_on_total(
    matched: u64,
    total: u64,
) -> impl Into<nvim::String> {
    let formatted = format!("{}/{}", matched, total);
    nvim::String::from(formatted.as_str())
}

/// TODO: docs
#[derive(Debug)]
pub enum PromptDiff {
    /// TODO: docs
    Insertion(usize, String),

    /// TODO: docs
    Deletion(Range<usize>),

    /// TODO: docs
    Replacement(String),
}

/// TODO: docs
fn on_bytes(
    sender: Sender<Message>,
) -> impl Fn(OnBytesArgs) -> Result<bool, nvim::api::Error> {
    move |(
        // The string "bytes".
        _bytes,
        // The prompt's buffer.
        buffer,
        // The buffer's changedtick.
        _changedtick,
        // The row where the change started. Always 0 because we immediately
        // close the prompt if the user tries to insert a newline.
        _start_row,
        // The column where the change started. Equal to `byte_offset` because
        // the prompt has a single line.
        _start_col,
        // The byte offset where the change started.
        start_offset,
        // The row containing the last changed byte before the change. Always
        // 0 because the prompt has a single line.
        _old_end_row,
        // The column containing the last changed byte before the change. Equal
        // to `old_end_len` because the prompt always has a single line.
        _old_end_col,
        // The length of the changed region before the change.
        old_end_len,
        // The row containing the last changed byte after the change. Always 0.
        _new_end_row,
        // The column containing the last changed byte after the change. Equal
        // to `new_end_len` because the prompt always has a single line.
        _new_end_col,
        // The length of the changed region after the change.
        new_end_len,
    ): (
        String,
        Buffer,
        u32,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
        usize,
    )| {
        let old_end_offset = start_offset + old_end_len;

        let new_end_offset = start_offset + new_end_len;

        handle_on_bytes(
            buffer,
            start_offset,
            old_end_offset,
            new_end_offset,
            &sender,
        );

        Ok(false)
    }
}

/// TODO: docs
fn handle_on_bytes(
    buffer: Buffer,
    start_offset: usize,
    old_end_offset: usize,
    new_end_offset: usize,
    sender: &Sender<Message>,
) {
    if buffer.line_count().unwrap() > 1 {
        sender.send(Message::Close);
        return;
    }

    let new_len = buffer.get_offset(1).unwrap() - 1;

    let is_empty = new_len == 0;

    let was_empty = !is_empty && (new_end_offset - start_offset) == new_len;

    if is_empty {
        sender.send(Message::ShowPlaceholder);
    } else if was_empty {
        sender.send(Message::HidePlaceholder);
    }

    let diff = match old_end_offset.cmp(&new_end_offset) {
        // The text that was in the `start_offset..old_end_offset` range
        // has been deleted.
        Ordering::Greater if start_offset == new_end_offset => {
            PromptDiff::Deletion(start_offset..old_end_offset)
        },

        // The text that's in the `start_offset..new_end_offset` has just been
        // inserted.
        Ordering::Less if start_offset == old_end_offset => {
            let insertion = buffer
                .get_text(
                    0..1,
                    start_offset,
                    new_end_offset,
                    &GetTextOpts::default(),
                )
                .unwrap()
                .next()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            PromptDiff::Insertion(start_offset, insertion)
        },

        // Anything that's not clearly an insertion or a deletion is just
        // considered a replacement of the whole prompt.
        _ => {
            let prompt = buffer
                .get_lines(0..1, true)
                .unwrap()
                .next()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            PromptDiff::Replacement(prompt)
        },
    };

    // nvim::print!("diff: {diff:?}");

    sender.send(Message::PromptChanged(diff));
}
