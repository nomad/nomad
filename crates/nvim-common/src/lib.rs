mod enable;
mod plugin;

pub use enable::Enable;
pub use nvim_oxi as oxi;
pub use plugin::Plugin;
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
        oxi::print!("{msg}");
    }
}
