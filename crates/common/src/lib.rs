mod api_builder;
mod command_builder;
mod enable;
mod lateinit;
mod plugin;
mod sender;

pub use api_builder::ApiBuilder;
pub use command_builder::CommandBuilder;
pub use enable::Enable;
pub use lateinit::LateInit;
pub use nvim_oxi as nvim;
pub use plugin::Plugin;
pub use sender::Sender;
pub use utils::*;

mod utils {
    use std::fmt::Display;

    use super::*;

    /// TODO: docs
    pub fn display_error<E: Display>(err: E, plugin: Option<&str>) {
        let mut msg = String::from("[mad");
        if let Some(plugin) = plugin {
            msg.push(':');
            msg.push_str(plugin);
        }
        msg.push_str("] ");
        msg.push_str(&err.to_string());
        nvim::print!("{msg}");
    }
}
