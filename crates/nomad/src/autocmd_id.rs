use std::cell::Cell;
use std::rc::Rc;

/// TODO: docs
pub(crate) struct AutocmdId {
    id: u32,
    num_clones: Rc<Cell<u32>>,
}

impl core::fmt::Debug for AutocmdId {
    #[inline]
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        f.debug_tuple("AutocmdId").field(&self.id).finish()
    }
}

impl Clone for AutocmdId {
    #[inline]
    fn clone(&self) -> Self {
        self.num_clones.set(self.num_clones.get() + 1);
        Self { id: self.id, num_clones: Rc::clone(&self.num_clones) }
    }
}

impl AutocmdId {
    #[inline]
    pub(crate) fn new(id: u32) -> Self {
        Self { id, num_clones: Rc::new(Cell::new(1)) }
    }
}

impl Drop for AutocmdId {
    #[inline]
    fn drop(&mut self) {
        let num_clones = self.num_clones.get() - 1;
        self.num_clones.set(num_clones);
        if num_clones == 0 {
            let _ = nvim::api::del_autocmd(self.id);
        }
    }
}
