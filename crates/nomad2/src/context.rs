use core::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::event::AnyEvent;
use crate::subscription::{self, AnyReceiver, InactiveReceiver};
use crate::{Editor, Event, Subscription};

/// TODO: docs.
pub struct Context<E> {
    inner: Arc<Mutex<ContextInner<E>>>,
}

impl<E: Editor> Context<E> {
    /// TODO: docs.
    #[inline]
    pub fn subscribe<T>(&self, event: T) -> Subscription<T, E>
    where
        T: Event<E>,
    {
        let ctx = self.clone();
        self.with_inner(move |inner| match inner.get_sub_receiver(&event) {
            Some((rx, event)) => {
                Subscription::new(event.clone(), rx.clone().reactivate(), ctx)
            },
            None => {
                let (emitter, rx) = subscription::channel();
                let sub_ctx = event.subscribe(emitter, &ctx);
                let event = AnyEvent::new(event);
                let state = SubscriptionState {
                    event: event.clone(),
                    rx: rx.clone().deactivate().into_any(),
                    sub_ctx: Box::new(sub_ctx),
                };
                inner.insert_subscription_state::<T>(state);
                Subscription::new(event, rx, ctx)
            },
        })
    }

    /// TODO: docs.
    #[inline]
    pub fn with_editor<F: FnOnce(&mut E) -> R, R>(&self, f: F) -> R {
        self.with_inner(|inner| f(&mut inner.editor))
    }

    /// TODO: docs.
    pub(crate) fn remove_subscription<T: Event<E>>(
        &self,
        event: &T,
    ) -> Option<SubscriptionState> {
        self.with_inner(|inner| {
            let vec = inner.subscriptions.get_mut(&TypeId::of::<T>())?;
            let idx = vec
                .binary_search_by(|sub| {
                    sub.event.downcast_ref::<T, E>().cmp(event)
                })
                .ok()?;
            Some(vec.remove(idx))
        })
    }

    #[inline]
    fn with_inner<R, F: FnOnce(&mut ContextInner<E>) -> R>(&self, f: F) -> R {
        let mut inner = self.inner.lock().expect("thread panicked");
        f(&mut *inner)
    }
}

impl<E> Clone for Context<E> {
    #[inline]
    fn clone(&self) -> Self {
        Self { inner: Arc::clone(&self.inner) }
    }
}

struct ContextInner<E> {
    editor: E,

    /// Map from the `TypeId` of a given event to a list of active
    /// subscriptions, sorted according to the event's `Ord` impl.
    subscriptions: HashMap<TypeId, Vec<SubscriptionState>>,
}

impl<E: Editor> ContextInner<E> {
    /// Returns the receiver for the givent event, or `None` if there aren't
    /// any active [`Subscription`]s for it.
    #[allow(clippy::type_complexity)]
    #[inline]
    fn get_sub_receiver<T: Event<E>>(
        &self,
        event: &T,
    ) -> Option<(&InactiveReceiver<T::Payload>, &AnyEvent)> {
        let vec = self.subscriptions.get(&TypeId::of::<T>())?;

        let idx = vec
            .binary_search_by(|sub| {
                sub.event.downcast_ref::<T, E>().cmp(event)
            })
            .ok()?;

        // SAFETY: todo.
        let inactive_rx = unsafe { vec[idx].rx.downcast_ref_unchecked() };
        let event = &vec[idx].event;
        Some((inactive_rx, event))
    }

    /// TODO: docs.
    #[allow(clippy::too_many_arguments)]
    #[inline]
    fn insert_subscription_state<T: Event<E>>(
        &mut self,
        state: SubscriptionState,
    ) {
        let vec = self.subscriptions.entry(TypeId::of::<T>()).or_default();
        let event = state.event.downcast_ref::<T, E>();

        let Err(idx) = vec.binary_search_by(|sub| {
            sub.event.downcast_ref::<T, E>().cmp(event)
        }) else {
            panic!("event already has a subscription");
        };

        vec.insert(idx, state);
    }
}

pub(crate) struct SubscriptionState {
    /// .
    pub(crate) event: AnyEvent,

    /// A type-erased, inactive receiver for payloads of a given event.
    pub(crate) rx: AnyReceiver,

    /// .
    pub(crate) sub_ctx: Box<dyn Any>,
}
