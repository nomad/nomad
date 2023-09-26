use std::cell::RefCell;
use std::collections::HashMap;
use std::convert::Infallible;
use std::rc::Rc;

use common::nvim::{Dictionary, Function, Object};
use common::{
    runtime::{self, Runtime},
    *,
};
use tracing::{Subscriber, *};

use crate::config;

/// TODO: docs
#[derive(Default)]
pub struct Mad {
    api: Api,
    runtime: Rc<RefCell<Runtime>>,
}

impl Mad {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new plugin.
    pub fn with_plugin<P: Plugin>(mut self) -> Self {
        let (plugin, msg_sender) = start::<P>(Rc::clone(&self.runtime));

        {
            // SAFETY: todo.
            let mut plugin = plugin.borrow_mut();
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
        self.runtime.borrow_mut().add_plugin(plugin);
        self
    }

    /// TODO: docs
    pub fn with_tracing_subscriber<S>(self, subscriber: S) -> Self
    where
        S: Subscriber + Send + Sync + 'static,
    {
        self.runtime.borrow_mut().add_tracing_subscriber(subscriber);
        info!("========== starting Nomad ==========");
        self
    }

    /// Returns the dictionary describing the APIs exposed by the plugins that
    /// have been registered.
    pub fn init(self) -> Api {
        let Self { api, runtime } = self;

        runtime::init(runtime);

        std::panic::set_hook(Box::new(|infos| {
            if let Some(location) = infos.location() {
                tracing::error!(
                    "panicked at {}:{}:{}{}",
                    location.file(),
                    location.line(),
                    location.column(),
                    infos
                        .payload()
                        .downcast_ref::<&str>()
                        .map(|msg| format!(": {msg}"))
                        .unwrap_or_default(),
                );
            } else {
                tracing::error!("{}", infos);
            }
        }));

        info!("finished initialization");

        api
    }
}

/// TODO: docs
#[derive(Default)]
pub struct Api {
    plugin_apis: HashMap<&'static str, Dictionary>,
}

impl Api {
    fn insert(&mut self, name: &'static str, api: Dictionary) {
        self.plugin_apis.insert(name, api);
    }

    /// TODO: docs
    pub fn api(self) -> Dictionary {
        self.plugin_apis
            .into_iter()
            .filter(|(_, api)| (!api.is_empty()))
            .map(|(name, api)| (name, Object::from(api)))
            .chain(core::iter::once((
                "config",
                Function::from_fn(config::config).into(),
            )))
            .collect()
    }
}

use std::sync::mpsc;

/// TODO: docs
pub(crate) fn start<P: Plugin>(
    runtime: Rc<RefCell<Runtime>>,
) -> (Rc<RefCell<P>>, Sender<P::Message>) {
    let plugin = Rc::new(RefCell::new(P::default()));

    let (msg_sender, msg_receiver) = mpsc::channel();

    let cloned = Rc::clone(&plugin);

    let ctx = Ctx::<P>::new(runtime);

    let plugin_loop = move || {
        while let Ok(msg) = msg_receiver.try_recv() {
            let plugin = Rc::clone(&cloned);

            let ctx = ctx.clone();

            nvim::schedule(move |_| {
                let mut plugin = plugin.borrow_mut();

                if let Err(err) = plugin.handle_message(msg, &ctx) {
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
