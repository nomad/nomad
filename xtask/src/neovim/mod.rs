use std::sync::LazyLock;

mod build;
mod print_crate_infos;

/// Metadata contained in the `Cargo.toml` of the package containing the
/// entrypoint to the Neovim plugin.
static ENTRYPOINT_METADATA: LazyLock<cargo_metadata::Package> =
    LazyLock::new(|| {
        let json = include_str!(concat!(
            env!("OUT_DIR"),
            "/neovim_package_metadata.json"
        ));
        serde_json::from_str(json).expect("failed to parse package metadata")
    });

#[derive(clap::Subcommand)]
pub(crate) enum Command {
    /// Build the Neovim plugin.
    #[command(visible_alias = "b")]
    Build(build::BuildArgs),

    /// Prints a JSON-formatted string including the name and version of the
    /// package containing the entrypoint to the Neovim plugin.
    PrintCrateInfos,
}

pub(crate) fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Build(args) => build::build(args)?,
        Command::PrintCrateInfos => print_crate_infos::run(),
    }

    Ok(())
}
