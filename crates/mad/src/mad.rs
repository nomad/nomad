use std::collections::HashMap;
use std::convert::Infallible;
use std::rc::Rc;

use common::nvim::{Dictionary, Function, Object};
use common::*;

use crate::config;
use crate::runtime::{self, MadRuntime};

/// TODO: docs
pub struct Mad {
    /// TODO: docs
    api: HashMap<&'static str, Dictionary>,

    /// TODO: docs
    runtime: MadRuntime,
}

impl Mad {
    /// Returns the dictionary describing the APIs exposed by the plugins that
    /// have been registered.
    pub fn api(self) -> Dictionary {
        let api = self.create_api();
        runtime::init(self.runtime);
        api
    }

    /// TODO: docs
    fn create_api(&self) -> Dictionary {
        self.api
            .iter()
            .filter(|(_, api)| (!api.is_empty()))
            .map(|(name, api)| (*name, Object::from(api.clone())))
            .chain(core::iter::once((
                "config",
                Function::from_fn(config::config).into(),
            )))
            .collect()
    }

    pub fn new() -> Self {
        Self { api: HashMap::new(), runtime: MadRuntime::new() }
    }

    /// Registers a new plugin.
    pub fn with_plugin<P: Plugin>(mut self) -> Self {
        let (plugin, msg_sender) = start::<P>();

        {
            // SAFETY: todo.
            let plugin = unsafe { rc_to_mut(&plugin) };
            if let Err(err) = plugin.init(&msg_sender) {
                display_error(err, Some(P::NAME));
            }
        }

        P::init_commands(&mut CommandBuilder::new(&msg_sender));

        let api = {
            let mut builder = ApiBuilder::new(&msg_sender);
            P::init_api(&mut builder);
            builder.api()
        };

        self.api.insert(P::NAME, api);
        self.runtime.add_plugin(plugin);
        self
    }
}

use std::sync::mpsc;

/// TODO: docs
pub(crate) fn start<P: Plugin>() -> (Rc<P>, Sender<P::Message>) {
    let plugin = Rc::new(P::default());

    let (msg_sender, msg_receiver) = mpsc::channel();

    let cloned = Rc::clone(&plugin);

    let plugin_loop = move || {
        while let Ok(msg) = msg_receiver.try_recv() {
            let plugin = Rc::clone(&cloned);

            nvim::schedule(move |_| {
                // SAFETY: todo.
                let plugin = unsafe { rc_to_mut(&plugin) };

                if let Err(err) = plugin.handle_message(msg) {
                    display_error(err, Some(P::NAME));
                }

                Ok(())
            })
        }

        Ok::<_, Infallible>(())
    };

    let handle = nvim::libuv::AsyncHandle::new(plugin_loop).unwrap();

    (plugin, Sender::new(msg_sender, handle))
}

#[allow(clippy::mut_from_ref)]
pub(crate) unsafe fn rc_to_mut<T: ?Sized>(rc: &Rc<T>) -> &mut T {
    // TODO: use `Rc::get_mut_unchecked` once it's stable.
    &mut *(Rc::as_ptr(rc) as *mut T)
}
