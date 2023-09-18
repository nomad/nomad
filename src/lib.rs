use mad::Mad;
use nvim_oxi as nvim;

#[nvim::module]
fn mad() -> nvim::Result<nvim::Dictionary> {
    Ok(Mad::new()
        // .with_plugin::<completion::Completion>()
        // .with_plugin::<lsp::Lsp>()
        .with_plugin::<seph::Seph>()
        .api())
}
