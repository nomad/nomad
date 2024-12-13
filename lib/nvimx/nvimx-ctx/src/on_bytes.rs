use nohash::IntMap as NoHashMap;
use nvim_oxi::api::opts;
use nvimx_common::Replacement;
use nvimx_diagnostics::{DiagnosticMessage, DiagnosticSource, Level};

use crate::autocmd::ShouldDetach;
use crate::buffer_id::BufferId;
use crate::neovim_ctx::NeovimCtx;
use crate::text_buffer_ctx::TextBufferCtx;
use crate::ActorId;

/// TODO: docs.
pub struct RegisterOnBytesArgs<Callback> {
    /// TODO: docs.
    pub callback: Callback,

    /// TODO: docs.
    pub module_name: Option<&'static str>,

    /// TODO: docs.
    pub callback_name: Option<&'static str>,
}

/// TODO: docs.
#[derive(Clone)]
pub struct OnBytesArgs {
    /// TODO: docs.
    pub actor_id: ActorId,

    /// TODO: docs.
    pub buffer_id: BufferId,

    /// TODO: docs.
    pub replacement: Replacement,
}

#[derive(Default)]
pub(crate) struct OnBytesMap {
    inner: NoHashMap<BufferId, Vec<OnBytesCallback>>,
}

type OnBytesCallback = Box<dyn FnMut(OnBytesArgs) -> ShouldDetach>;

impl OnBytesMap {
    pub(crate) fn register<Callback>(
        &mut self,
        mut args: RegisterOnBytesArgs<Callback>,
        buffer_id: BufferId,
        ctx: NeovimCtx<'static>,
    ) where
        Callback: for<'ctx> FnMut(
                OnBytesArgs,
                TextBufferCtx<'ctx>,
            )
                -> Result<ShouldDetach, DiagnosticMessage>
            + 'static,
    {
        let callback = {
            let ctx = ctx.clone();
            move |buf_attach_args: OnBytesArgs| {
                let text_buffer_ctx = ctx
                    .reborrow()
                    .into_buffer(buf_attach_args.buffer_id)
                    .expect("buffer ID is valid")
                    .into_text_buffer()
                    .expect("buffer is text buffer");
                match (args.callback)(buf_attach_args, text_buffer_ctx) {
                    Ok(res) => res,
                    Err(msg) => {
                        let mut source = DiagnosticSource::new();
                        if let Some(module_name) = args.module_name {
                            source.push_segment(module_name);
                        }
                        source.push_segment("BufAttach");
                        if let Some(callback_name) = args.callback_name {
                            source.push_segment(callback_name);
                        }
                        msg.emit(Level::Error, source);
                        ShouldDetach::Yes
                    },
                }
            }
        };

        let mut has_attached_to_buffer = true;

        let callbacks = self.inner.entry(buffer_id).or_insert_with(|| {
            has_attached_to_buffer = false;
            Vec::new()
        });

        callbacks.push(Box::new(callback));

        if !has_attached_to_buffer {
            attach_to(buffer_id, ctx);
        }
    }
}

fn attach_to(buffer_id: BufferId, ctx: NeovimCtx<'static>) {
    let callback = {
        move |args: opts::OnBytesArgs| {
            let text_buffer_ctx = ctx
                .reborrow()
                .into_buffer(buffer_id)
                .expect(
                    "`on_bytes` is being called, so the buffer ID must still \
                     be valid",
                )
                .into_text_buffer()
                .expect(
                    "`on_bytes` is being called, so the buffer must be a \
                     text buffer",
                );

            let buf_attach_args = OnBytesArgs {
                actor_id: ctx
                    .with_actor_map(|m| m.take_edited_buffer(&buffer_id)),
                buffer_id,
                replacement: text_buffer_ctx
                    .replacement_of_on_bytes_args(args),
            };

            ctx.with_buf_attach_map(|m| {
                let Some(callbacks) = m.inner.get_mut(&buffer_id) else {
                    panic!(
                        "Neovim executed `on_bytes` callback on unregistered \
                         buffer: {buffer_id:?}"
                    );
                };
                let mut idx = 0;
                loop {
                    let Some(callback) = callbacks.get_mut(idx) else {
                        break;
                    };
                    if callback(buf_attach_args.clone()).into() {
                        let _ = callbacks.remove(idx);
                    } else {
                        idx += 1;
                    }
                }
                let should_detach = callbacks.is_empty();
                if should_detach {
                    m.inner.remove(&buffer_id);
                }
                should_detach
            })
        }
    };

    let opts = opts::BufAttachOpts::builder().on_bytes(callback).build();

    buffer_id
        .as_nvim()
        .attach(false, &opts)
        .expect("all the arguments are valid");
}
