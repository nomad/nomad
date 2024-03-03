use alloc::rc::Rc;
use core::cell::RefCell;

use neovim::Ctx;

use crate::log;
use crate::prelude::nvim::Dictionary;
use crate::runtime;
use crate::{EnableConfig, Module, ObjectSafeModule};

/// TODO: docs
pub struct Nomad {
    /// TODO: docs
    api: Dictionary,

    /// TODO: docs
    ctx: Rc<RefCell<Ctx>>,
    // /// TODO: docs
    // modules: Vec<Box<dyn ObjectSafeModule>>,
}

impl Default for Nomad {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}

impl Nomad {
    /// TODO: docs
    #[inline]
    pub fn api(self) -> Dictionary {
        let Self { api, .. } = self;
        // load(ctx, modules);
        api
    }

    /// TODO: docs
    #[inline]
    pub fn new() -> Self {
        log::init();

        log::info!("======== Starting Nomad ========");

        Self::new_default()
    }

    /// TODO: docs
    #[inline]
    fn new_default() -> Self {
        Self { api: Dictionary::default(), ctx: Rc::default() }
    }

    /// TODO: docs
    #[inline]
    pub fn with_module<M: Module>(mut self) -> Self {
        let ctx = self.ctx.borrow();

        let init_ctx = ctx.as_init();

        // TODO: docs
        let (config, _set_config) =
            init_ctx.new_input(EnableConfig::<M>::default());

        let module = M::init(config, init_ctx);

        drop(ctx);

        let module_api = ObjectSafeModule::api(&module, &self.ctx);

        self.api.insert(M::NAME.as_str(), module_api);

        for _command in module.commands() {}

        let ctx = self.ctx.clone();

        runtime::spawn(
            #[allow(clippy::await_holding_refcell_ref)]
            async move {
                let ctx = &mut *ctx.borrow_mut();
                let set_ctx = ctx.as_set();
                let _ = module.load(set_ctx).await;
            },
        )
        .detach();

        self
    }
}

// /// TODO: docs
// fn load(ctx: Rc<RefCell<Ctx>>, modules: Vec<Box<dyn ObjectSafeModule>>) {
//     nvim::schedule(move |()| {
//         let ctx = &mut *ctx.borrow_mut();
//         let set_ctx = ctx.as_set();
//         for module in modules {
//             module.load(set_ctx);
//         }
//         Ok(())
//     });
// }
