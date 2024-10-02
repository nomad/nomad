use core::cmp::Ordering;
use core::marker::PhantomData;

use nvim_oxi::serde::Deserializer as NvimDeserializer;
use nvim_oxi::{Function as NvimFunction, Object as NvimObject};
use serde::de::{Deserialize, DeserializeOwned};

use super::Neovim;
use crate::{Context, Emitter, Event, Module, Shared, Subscription};

/// TODO: docs.
#[inline]
pub fn function<T: Function>(
    ctx: &Context<Neovim>,
) -> (FunctionHandle, Subscription<FunctionEvent<T>, Neovim>) {
    let buf = Shared::new(None);
    let event = FunctionEvent {
        module_name: T::Module::NAME.as_str(),
        function_name: T::NAME,
        function_buf: buf.clone(),
        ty: PhantomData,
    };
    let sub = ctx.subscribe(event);
    let handle = FunctionHandle {
        name: T::NAME,
        inner: buf.with_mut(Option::take).expect("just set when subscribing"),
    };
    (handle, sub)
}

/// TODO: docs.
pub trait Function: 'static {
    /// TODO: docs.
    const NAME: &'static str;

    /// TODO: docs.
    type Args: DeserializeOwned;

    /// TODO: docs.
    type Module: Module<Neovim>;
}

/// TODO: docs.
pub struct FunctionHandle {
    pub(super) name: &'static str,
    pub(super) inner: NvimFunction<NvimObject, ()>,
}

/// TODO: docs.
pub struct FunctionEvent<T> {
    module_name: &'static str,
    function_name: &'static str,
    function_buf: Shared<Option<NvimFunction<NvimObject, ()>>>,
    ty: PhantomData<T>,
}

impl<T: Function> Event<Neovim> for FunctionEvent<T> {
    type Payload = T::Args;
    type SubscribeCtx = ();

    #[inline]
    fn subscribe(&mut self, emitter: Emitter<T::Args>, _: &Context<Neovim>) {
        let nvim_fun = NvimFunction::<NvimObject, ()>::from_fn(move |obj| {
            match T::Args::deserialize(NvimDeserializer::new(obj)) {
                Ok(payload) => emitter.send(payload),
                Err(_err) => {
                    todo!();
                },
            };
        });

        self.function_buf.with_mut(|buf| {
            *buf = Some(nvim_fun);
        });
    }
}

impl<T> PartialEq for FunctionEvent<T> {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<T> Eq for FunctionEvent<T> {}

impl<T> PartialOrd for FunctionEvent<T> {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for FunctionEvent<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.module_name.cmp(other.module_name) {
            Ordering::Equal => self.function_name.cmp(other.function_name),
            ord => ord,
        }
    }
}
