use core::convert::Infallible;
use core::pin::Pin;
use core::task::{Context, Poll};

use flume::r#async::RecvStream;
use futures::Stream;
use nvim::api::opts::CreateAutocmdOpts;
use nvim::api::types::AutocmdCallbackArgs;
use pin_project_lite::pin_project;

use crate::editor::BufferId;
use crate::AutocmdId;

pin_project! {
    /// A [`Stream`] that yields the [`BufferId`] of the currently focused
    /// buffer every time it changes.
    pub struct FocusedBuffer {
        autocmd_id: AutocmdId,
        #[pin]
        rx_stream: RecvStream<'static, BufferId>,
    }
}

impl Default for FocusedBuffer {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl FocusedBuffer {
    /// Create a new [`FocusedBuffer`] stream.
    #[inline]
    pub fn new() -> Self {
        let (tx, rx) = flume::unbounded();

        let opts = CreateAutocmdOpts::builder()
            .callback(move |args: AutocmdCallbackArgs| {
                let res = tx.send(BufferId::from(&args.buffer));
                Ok::<_, Infallible>(res.is_err())
            })
            .build();

        let Ok(autocmd_id) = nvim::api::create_autocmd(["BufEnter"], &opts)
        else {
            unreachable!("the opts are valid");
        };

        Self {
            autocmd_id: AutocmdId::new(autocmd_id),
            rx_stream: rx.into_stream(),
        }
    }
}

impl Stream for FocusedBuffer {
    type Item = BufferId;

    #[inline]
    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project().rx_stream.poll_next(cx)
    }
}
