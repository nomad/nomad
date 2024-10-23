use nohash::IntMap as NoHashMap;
use nvim_oxi::api::opts;

use crate::autocmd::ShouldDetach;
use crate::ctx::NeovimCtx;
use crate::diagnostics::{DiagnosticSource, Level};
use crate::maybe_result::MaybeResult;
use crate::neovim::BufferId;
use crate::{Action, ActorId, Module, Replacement};

/// TODO: docs.
#[derive(Clone)]
pub struct BufAttachArgs {
    /// TODO: docs.
    pub actor_id: ActorId,

    /// TODO: docs.
    pub buffer_id: BufferId,

    /// TODO: docs.
    pub replacement: Replacement,
}

#[derive(Default)]
pub(crate) struct BufAttachMap {
    inner: NoHashMap<BufferId, Vec<BufAttachCallback>>,
}

type BufAttachCallback = Box<dyn FnMut(BufAttachArgs) -> ShouldDetach>;

impl BufAttachMap {
    pub(crate) fn attach<A>(
        &mut self,
        buffer_id: BufferId,
        mut action: A,
        ctx: NeovimCtx<'static>,
    ) where
        A: Action,
        A::Args: From<BufAttachArgs>,
        A::Return: Into<ShouldDetach>,
    {
        let callback = move |buf_attach_args: BufAttachArgs| {
            let args = buf_attach_args.into();
            match action.execute(args).into_result() {
                Ok(res) => res.into(),
                Err(err) => {
                    let mut source = DiagnosticSource::new();
                    source
                        .push_segment(<A::Module as Module>::NAME.as_str())
                        .push_segment(A::NAME.as_str());
                    err.into().emit(Level::Error, source);
                    ShouldDetach::Yes
                },
            }
        };

        let mut has_attached_to_buffer = true;

        let callbacks =
            self.inner.entry(buffer_id.clone()).or_insert_with(|| {
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
        let buffer_id = buffer_id.clone();
        move |args: opts::OnBytesArgs| {
            let text_buffer_ctx = ctx
                .as_ref()
                .into_buffer(buffer_id.clone())
                .clone()
                .expect(
                    "`on_bytes` is being called, so the buffer ID must still \
                     be valid",
                )
                .into_text_buffer()
                .expect(
                    "`on_bytes` is being called, so the buffer must be a \
                     text buffer",
                );

            let buf_attach_args = BufAttachArgs {
                actor_id: ctx
                    .with_actor_map(|m| m.take_edited_buffer(&buffer_id)),
                buffer_id: buffer_id.clone(),
                replacement: Replacement::from_on_bytes_args(
                    args,
                    text_buffer_ctx,
                ),
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
                        callbacks.remove(idx);
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
