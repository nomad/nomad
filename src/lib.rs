use mad::Mad;
use nvim_oxi as nvim;

#[nvim::module]
fn mad() -> nvim::Result<nvim::Dictionary> {
    Ok(Mad::new()
        .with_tracing_subscriber(tracing_subscriber::subscriber())
        .with_plugin::<colorschemes::Colorschemes>()
        .with_plugin::<fuzzy_modal::FuzzyModal>()
        .with_plugin::<seph::Seph>()
        .init()
        .api())
}
