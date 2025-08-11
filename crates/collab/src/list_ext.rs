use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::Stream;
use futures_util::stream::FusedStream;

/// A trait for list-like types that can be indexed with a `usize`.
pub(crate) trait List {
    type Value;

    /// Returns an exclusive reference to the value at the given index.
    fn get_mut(&mut self, idx: usize) -> &mut Self::Value;

    /// Returns whether the sequence is empty.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns the number of elements in the sequence.
    fn len(&self) -> usize;

    /// Removes the value at the given index.
    fn remove(&mut self, idx: usize) -> Self::Value;

    /// Turns a `List` of `Stream`s into a `Stream` of the `List`'s `Value`s.
    fn as_stream(&mut self, seed: u64) -> StreamList<'_, Self>
    where
        Self::Value: Stream + Unpin,
    {
        StreamList { rng: fastrand::Rng::with_seed(seed), list: self }
    }
}

pub(crate) struct StreamList<'a, T: ?Sized> {
    list: &'a mut T,
    rng: fastrand::Rng,
}

impl<K, V, S> List for indexmap::IndexMap<K, V, S> {
    type Value = V;

    fn get_mut(&mut self, idx: usize) -> &mut Self::Value {
        self.get_index_mut(idx).expect("index is valid").1
    }

    fn len(&self) -> usize {
        indexmap::IndexMap::len(self)
    }

    fn remove(&mut self, idx: usize) -> Self::Value {
        self.swap_remove_index(idx).expect("index is valid").1
    }
}

impl<'a, T> Stream for StreamList<'a, T>
where
    T: List<Value: Stream + Unpin> + ?Sized,
{
    type Item = <T::Value as Stream>::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let initial_len = self.list.len();

        if initial_len == 0 {
            return Poll::Ready(None);
        }

        let mut idx = self.rng.usize(0..initial_len);

        // The number of streams to poll before looping the index back to 0.
        let num_loop_after = initial_len - idx;

        let mut num_checked = 0;
        loop {
            match Pin::new(self.list.get_mut(idx)).poll_next(ctx) {
                Poll::Ready(Some(val)) => {
                    return Poll::Ready(Some(val));
                },
                Poll::Ready(None) => {
                    self.list.remove(idx);
                },
                Poll::Pending => idx += 1,
            }
            num_checked += 1;
            if num_checked == initial_len {
                return if self.list.is_empty() {
                    Poll::Ready(None)
                } else {
                    Poll::Pending
                };
            }
            if num_checked == num_loop_after {
                idx = 0;
            }
        }
    }
}

impl<'a, T> FusedStream for StreamList<'a, T>
where
    T: List<Value: Stream + Unpin> + ?Sized,
{
    fn is_terminated(&self) -> bool {
        self.list.is_empty()
    }
}
