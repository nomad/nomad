use core::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

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
            Some((rx, count)) => {
                count.fetch_add(1, Ordering::Relaxed);
                Subscription::new(rx.reactivate(), count, ctx)
            },
            None => {
                let (emitter, rx) = subscription::channel();
                let sub_ctx = event.subscribe(emitter, &ctx);
                let count = Arc::new(AtomicU32::new(1));
                inner.insert_subscription_state(
                    Arc::clone(&count),
                    event,
                    rx.clone().deactivate().into_any(),
                    sub_ctx,
                );
                Subscription::new(rx, count, ctx)
            },
        })
    }

    #[inline]
    fn with_inner<R, F: FnOnce(&mut ContextInner<E>) -> R>(&self, f: F) -> R {
        todo!();
    }
}

impl<E> Clone for Context<E> {
    #[inline]
    fn clone(&self) -> Self {
        todo!();
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
    #[inline]
    fn get_sub_receiver<T: Event<E>>(
        &self,
        event: &T,
    ) -> Option<(InactiveReceiver<T::Payload>, Arc<AtomicU32>)> {
        let vec = self.subscriptions.get(&TypeId::of::<T>())?;

        let idx = vec
            .binary_search_by(|subscription| {
                // SAFETY: todo.
                //
                // TODO: use `downcast_ref_unchecked` once it's stable.
                let probe = unsafe {
                    subscription
                        .event
                        .as_ref()
                        .downcast_ref::<T>()
                        .unwrap_unchecked()
                };

                probe.cmp(event)
            })
            .ok()?;

        // SAFETY: todo.
        let inactive_rx = unsafe { vec[idx].rx.downcast_ref_unchecked() };
        let count = &vec[idx].active_rx_count;
        Some((inactive_rx.clone(), Arc::clone(count)))
    }

    /// TODO: docs.
    #[inline]
    fn insert_subscription_state<T: Event<E>>(
        &mut self,
        active_rx_count: Arc<AtomicU32>,
        event: T,
        rx: AnyReceiver,
        sub_ctx: T::SubscribeCtx,
    ) {
        let vec = self.subscriptions.entry(TypeId::of::<T>()).or_default();

        let Err(idx) = vec.binary_search_by(|subscription| {
            // SAFETY: todo.
            //
            // TODO: use `downcast_ref_unchecked` once it's stable.
            let probe = unsafe {
                subscription
                    .event
                    .as_ref()
                    .downcast_ref::<T>()
                    .unwrap_unchecked()
            };

            probe.cmp(&event)
        }) else {
            panic!("event already has a subscription");
        };

        let state = SubscriptionState {
            active_rx_count,
            event: Box::new(event),
            rx,
            sub_ctx: Box::new(sub_ctx),
        };

        vec.insert(idx, state);
    }
}

struct SubscriptionState {
    /// .
    active_rx_count: Arc<AtomicU32>,

    /// .
    event: Box<dyn Any>,

    /// A type-erased, inactive receiver for payloads of a given event.
    rx: AnyReceiver,

    /// .
    sub_ctx: Box<dyn Any>,
}
