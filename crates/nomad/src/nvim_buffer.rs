use nvim::api::{self, opts};
use smol_str::SmolStr;

use crate::shared::Shared;

type OnEdit = Box<dyn FnMut(&NvimEdit) + 'static>;

type ByteOffset = usize;

/// A handle to a Neovim buffer.
#[cfg_attr(not(feature = "tests"), doc(hidden))]
#[derive(Clone)]
pub struct NvimBuffer {
    /// The buffer handle.
    inner: api::Buffer,

    /// The list of callbacks to be called every time the buffer is edited.
    on_edit_callbacks: Shared<Vec<OnEdit>>,
}

impl core::fmt::Debug for NvimBuffer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("NvimBuffer").field(&self.inner).finish()
    }
}

impl NvimBuffer {
    /// Registers a callback to be called every time the buffer is edited.
    pub fn on_edit<F: FnMut(&NvimEdit) + 'static>(&self, callback: F) {
        self.on_edit_callbacks
            .with_mut(|callbacks| callbacks.push(Box::new(callback)));
    }

    #[inline]
    fn new(buffer: api::Buffer) -> Result<Self, NvimBufferDoesntExist> {
        let on_edit_callbacks = Shared::<Vec<OnEdit>>::default();

        let cbs = on_edit_callbacks.clone();

        let opts = opts::BufAttachOpts::builder()
            .on_bytes(move |args| {
                let edit = NvimEdit::from(args);
                cbs.with_mut(|cbs| cbs.iter_mut().for_each(|cb| cb(&edit)));
                Ok(false)
            })
            .build();

        buffer
            .attach(false, &opts)
            // All the arguments passed to `attach()` are valid, so if it fails
            // it must be because the buffer doesn't exist.
            .map_err(|_| NvimBufferDoesntExist)?;

        Ok(Self { inner: buffer, on_edit_callbacks })
    }
}

#[cfg_attr(not(feature = "tests"), doc(hidden))]
#[derive(Debug, Clone)]
pub struct NvimEdit {
    start: ByteOffset,
    end: ByteOffset,
    replacement: SmolStr,
}

impl From<opts::OnBytesArgs> for NvimEdit {
    #[inline]
    fn from(
        (
            _bytes,
            buf,
            _changedtick,
            start_row,
            start_col,
            start_offset,
            _old_end_row,
            _old_end_col,
            old_end_len,
            new_end_row,
            new_end_col,
            _new_end_len,
        ): opts::OnBytesArgs,
    ) -> Self {
        todo!();
        // let replacement_start = Point { row: start_row, col: start_col };
        //
        // let replacement_end = Point {
        //     row: start_row + new_end_row,
        //     col: start_col * (new_end_row == 0) as usize + new_end_col,
        // };
        //
        // let replacement = if replacement_start == replacement_end {
        //     String::new()
        // } else {
        //     nvim_buf_get_text(&buf, replacement_start..replacement_end)
        //         .expect("buffer must exist")
        // };
        //
        // Self {
        //     start: start_offset,
        //     end: start_offset + old_end_len,
        //     replacement,
        // }
    }
}

/// An error returned whenever a..
pub struct NvimBufferDoesntExist;
