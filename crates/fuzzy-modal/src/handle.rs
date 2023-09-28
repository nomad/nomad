use common::Sender;

use crate::*;

pub struct FuzzyHandle {
    sender: Sender<(ModalId, Message)>,
    modal_id: ModalId,
}

impl FuzzyHandle {
    /// TODO: docs
    pub fn add_results(&self, results: Vec<FuzzyItem>) {
        self.send(Message::AddResults(results))
    }

    /// TODO: docs
    pub fn close(self) {
        self.send(Message::Close)
    }

    pub(crate) fn new(
        sender: Sender<(ModalId, Message)>,
        modal_id: ModalId,
    ) -> Self {
        Self { sender, modal_id }
    }

    fn send(&self, msg: Message) {
        self.sender.send((self.modal_id, msg))
    }
}
