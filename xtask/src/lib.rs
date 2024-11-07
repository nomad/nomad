#![allow(missing_docs)]

mod r#build;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(about)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Build the Nomad plugin.
    #[command(visible_alias = "b")]
    Build(build::BuildArgs),
}

/// The entrypoint of the `xtask` binary.
pub fn run() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Build(args) => build::build(args),
    }
}
