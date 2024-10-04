use core::future::Future;
use core::pin::Pin;

use crate::{Context, Editor, JoinHandle, Module, Spawner};

/// TODO: docs.
pub struct Nomad<E: Editor> {
    api: E::Api,
    ctx: Context<E>,
    run: Vec<Pin<Box<dyn Future<Output = ()>>>>,
}

impl<E: Editor> Nomad<E> {
    /// TODO: docs.
    pub fn into_api(self) -> E::Api {
        self.api
    }

    /// TODO: docs.
    pub fn new(editor: E) -> Self {
        crate::log::init(&editor.log_dir());
        Self {
            api: E::Api::default(),
            ctx: Context::new(editor),
            run: Vec::default(),
        }
    }

    /// TODO: docs.
    pub fn start_modules(&mut self) {
        for fut in self.run.drain(..) {
            self.ctx.spawner().spawn(fut).detach();
        }
    }

    /// TODO: docs.
    #[track_caller]
    pub fn with_module<M: Module<E>>(mut self) -> Self {
        let (mut module, module_api) = M::init(&self.ctx);
        self.api += module_api;
        self.run.push({
            let ctx = self.ctx.clone();
            Box::pin(async move { module.run(&ctx).await })
        });
        self
    }
}
