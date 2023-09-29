mod config;
mod fuzzy_item;
mod fuzzy_modal;
mod handle;
pub mod highlights;
mod message;
mod modal_config;
mod prompt;
mod results;
mod view;

use config::Config;
pub use fuzzy_item::FuzzyItem;
pub use fuzzy_modal::FuzzyModal;
use fuzzy_modal::Sender;
pub use handle::FuzzyHandle;
use message::Message;
use modal_config::*;
use prompt::{Prompt, PromptConfig, PromptDiff};
use results::*;
use view::{ConfirmResult, View};

/// TODO: docs
type ModalId = u64;

/// TODO: docs
const PASSTHROUGH_ID: ModalId = u64::MAX;

/// TODO: docs
fn passthrough(msg: Message) -> (ModalId, Message) {
    (PASSTHROUGH_ID, msg)
}
