use core::cmp::Ordering;
use core::marker::PhantomData;

use nvim_oxi::Object as NvimObject;
use serde::de::DeserializeOwned;

use super::Neovim;
use crate::{Context, Emitter, Event, Module, Shared};

pub(super) type OnConfigChange = Box<dyn Fn(NvimObject)>;

/// TODO: docs.
pub struct ConfigEvent<T> {
    pub(super) module_name: &'static str,
    pub(super) buf: Shared<Option<OnConfigChange>>,
    pub(super) ty: PhantomData<T>,
}

impl<T: Module<Neovim>> ConfigEvent<T> {
    pub(super) fn new(buf: Shared<Option<OnConfigChange>>) -> Self {
        Self { module_name: T::NAME.as_str(), buf, ty: PhantomData }
    }
}

impl<T> PartialEq for ConfigEvent<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T> Eq for ConfigEvent<T> {}

impl<T> PartialOrd for ConfigEvent<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for ConfigEvent<T> {
    #[inline]
    fn cmp(&self, other: &Self) -> Ordering {
        self.module_name.cmp(other.module_name)
    }
}

impl<T: Module<Neovim>> Event<Neovim> for ConfigEvent<T> {
    type Payload = T::Config;
    type SubscribeCtx = ();

    fn subscribe(
        &mut self,
        emitter: Emitter<Self::Payload>,
        ctx: &Context<Neovim>,
    ) {
        let on_config_change =
            Box::new(move |obj| match obj_to_config::<T::Config>(obj) {
                Ok(config) => emitter.send(config),
                Err(err) => {
                    todo!();
                },
            });

        self.buf.with_mut(|buf| {
            *buf = Some(on_config_change);
        });
    }
}

fn obj_to_config<T: DeserializeOwned>(obj: NvimObject) -> Result<T, ()> {
    todo!();
}
