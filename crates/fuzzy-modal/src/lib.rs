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
pub use handle::FuzzyHandle;
use message::Message;
use modal_config::{FuzzyBuilder, FuzzyConfig};
use prompt::{Prompt, PromptConfig, PromptDiff};
use results::Results;
use view::View;

type ModalId = u64;
