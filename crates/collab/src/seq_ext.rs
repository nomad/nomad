//! Extension traits for map-like types.

use core::pin::Pin;
use core::task::{Context, Poll};

use futures_util::Stream;
use futures_util::stream::FusedStream;

pub(crate) trait IndexableSeq {
    type Value;

    /// Returns a shared reference to the value at the given index.
    fn get(&self, idx: usize) -> &Self::Value;

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
}

pub(crate) trait StreamableSeq:
    IndexableSeq<Value: Stream + Unpin>
{
    /// TODO: docs.
    fn as_stream(&mut self, seed: u64) -> StreamSeq<Self> {
        StreamSeq { rng: fastrand::Rng::with_seed(seed), seq: self }
    }
}

pub(crate) struct StreamSeq<'a, M: ?Sized> {
    rng: fastrand::Rng,
    seq: &'a mut M,
}

impl<K, V, S> IndexableSeq for indexmap::IndexMap<K, V, S> {
    type Value = V;

    fn get(&self, idx: usize) -> &Self::Value {
        self.get_index(idx).expect("index is valid").1
    }

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

impl<T: IndexableSeq<Value: Stream + Unpin>> StreamableSeq for T {}

impl<'a, M: StreamableSeq + ?Sized> Stream for StreamSeq<'a, M> {
    type Item = <M::Value as Stream>::Item;

    fn poll_next(
        mut self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let initial_len = self.seq.len();

        if initial_len == 0 {
            return Poll::Ready(None);
        }

        let mut idx = self.rng.usize(0..initial_len);

        // The number of streams to poll before looping the index back to 0.
        let num_loop_after = initial_len - idx;

        let mut num_checked = 0;
        loop {
            match Pin::new(self.seq.get_mut(idx)).poll_next(ctx) {
                Poll::Ready(Some(val)) => {
                    return Poll::Ready(Some(val));
                },
                Poll::Ready(None) => {
                    self.seq.remove(idx);
                },
                Poll::Pending => idx += 1,
            }
            num_checked += 1;
            if num_checked == initial_len {
                return Poll::Ready(None);
            }
            if num_checked == num_loop_after {
                idx = 0;
            }
        }
    }
}

impl<'a, M: StreamableSeq + ?Sized> FusedStream for StreamSeq<'a, M> {
    fn is_terminated(&self) -> bool {
        self.seq.is_empty()
    }
}
