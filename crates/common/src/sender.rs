use std::sync::mpsc;

use crate::nvim;

/// TODO: docs
pub struct Sender<M> {
    sender: mpsc::Sender<M>,
    handle: nvim::libuv::AsyncHandle,
}

impl<M> Clone for Sender<M> {
    #[inline]
    fn clone(&self) -> Self {
        Self { sender: self.sender.clone(), handle: self.handle.clone() }
    }
}

impl<M> Sender<M> {
    /// TODO: docs
    pub fn new(
        sender: mpsc::Sender<M>,
        handle: nvim::libuv::AsyncHandle,
    ) -> Self {
        Self { sender, handle }
    }

    /// TODO: docs
    pub fn send(&self, msg: M) {
        self.sender.send(msg).unwrap();
        self.handle.send().unwrap();
    }
}
