use core::borrow::Borrow;

pub(crate) struct OrderedMap<K, V> {
    inner: Vec<(K, V)>,
}

impl<K: Ord, V> OrderedMap<K, V> {
    #[inline]
    pub(crate) fn contains_key(&self, key: K) -> bool {
        self.get_idx(&key).is_ok()
    }

    #[inline]
    pub(crate) fn get_index_mut(
        &mut self,
        idx: usize,
    ) -> Option<(&K, &mut V)> {
        self.inner.get_mut(idx).map(|(k, v)| (&*k, v))
    }

    #[inline]
    pub(crate) fn get_key_value_mut<Q>(
        &mut self,
        key: &Q,
    ) -> Option<(&K, &mut V)>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let idx = self.get_idx(key).ok()?;
        let (key, value) = &mut self.inner[idx];
        Some((key, value))
    }

    #[inline]
    pub(crate) fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut V>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        let idx = self.get_idx(key).ok()?;
        Some(&mut self.inner[idx].1)
    }

    #[inline]
    pub(crate) fn insert(&mut self, key: K, value: V) -> &mut V {
        let idx = self.get_idx(&key).unwrap_or_else(|x| x);
        self.inner.insert(idx, (key, value));
        &mut self.inner[idx].1
    }

    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    #[inline]
    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    #[inline]
    pub(crate) fn keys(&self) -> impl ExactSizeIterator<Item = &K> + '_ {
        self.inner.iter().map(|(k, _)| k)
    }

    #[inline]
    pub(crate) fn remove_index(&mut self, idx: usize) -> Option<(K, V)> {
        (idx < self.len()).then(|| self.inner.remove(idx))
    }

    #[inline]
    fn get_idx<Q>(&self, key: &Q) -> Result<usize, usize>
    where
        K: Borrow<Q>,
        Q: ?Sized + Ord,
    {
        self.inner
            .binary_search_by(|(probe, _)| Borrow::<Q>::borrow(probe).cmp(key))
    }
}

impl<K, V> Default for OrderedMap<K, V> {
    #[inline]
    fn default() -> Self {
        Self { inner: Vec::new() }
    }
}
