use std::sync::LazyLock;

use abs_path::{AbsPathBuf, node};

mod build;
mod print_crate_infos;

/// The path to the `Cargo.toml` of the package containing the entrypoint to
/// the Neovim plugin.
static CARGO_TOML_PATH: LazyLock<AbsPathBuf> = LazyLock::new(|| {
    crate::WORKSPACE_ROOT
        .join(node!("crates"))
        .join(node!("nomad-neovim"))
        .join(node!("Cargo.toml"))
});

/// Metadata contained in the `Cargo.toml` of the package containing the
/// entrypoint to the Neovim plugin.
static CARGO_TOML_META: LazyLock<cargo_metadata::Package> =
    LazyLock::new(|| {
        let manifest_path = &CARGO_TOML_PATH;

        let meta = match cargo_metadata::MetadataCommand::new()
            .manifest_path((**manifest_path).clone())
            .no_deps()
            .exec()
        {
            Ok(meta) => meta,
            Err(err) => {
                panic!(
                    "couldn't run 'cargo metadata' for manifest at \
                     {manifest_path:?}: {err}",
                )
            },
        };

        let Some(package) = meta.packages.iter().find(|package| {
            package.manifest_path.as_str() == manifest_path.as_str()
        }) else {
            panic!(
                "couldn't find the root package for manifest at \
                 {manifest_path:?}"
            )
        };

        package.clone()
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
