mod config;
mod mad;
mod runtime;

use common::oxi;
use mad::Mad;

#[oxi::module]
fn mad() -> oxi::Result<oxi::Dictionary> {
    Ok(Mad::new()
        // .with_plugin(completion::Completion)
        // .with_plugin(lsp::Lsp)
        .with_plugin(seph::Seph)
        .api())
}
