#![allow(missing_docs)]

use core::iter;
use std::ffi::OsStr;
use std::path::{self, Path, PathBuf};
use std::sync::LazyLock;
use std::{env, fs};

static CARGO_MANIFEST_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    Path::new(&env::var("CARGO_MANIFEST_DIR").expect("is set")).to_owned()
});

static OUT_DIR: LazyLock<PathBuf> = LazyLock::new(|| {
    Path::new(&env::var("OUT_DIR").expect("is set")).to_owned()
});

static NEOVIM_ENTRYPOINT_MANIFEST: LazyLock<PathBuf> = LazyLock::new(|| {
    WORKSPACE_ROOT
        .clone()
        .join("crates")
        .join("nomad-neovim")
        .join("Cargo.toml")
});

static WORKSPACE_ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
    CARGO_MANIFEST_DIR.parent().expect("not root").to_owned()
});

fn main() {
    let json = serde_json::to_string(&neovim_entrypoint_package())
        .expect("failed to serialize metadata");

    fs::write(OUT_DIR.join("neovim_package_metadata.json"), json)
        .expect("failed to write metadata");

    println!(
        "cargo:rerun-if-changed={}",
        relative_to_manifest_dir(&WORKSPACE_ROOT.join("Cargo.toml")).display(),
    );
    println!(
        "cargo:rerun-if-changed={}",
        relative_to_manifest_dir(&NEOVIM_ENTRYPOINT_MANIFEST).display(),
    );
}

fn neovim_entrypoint_package() -> cargo_metadata::Package {
    let manifest_path = &**NEOVIM_ENTRYPOINT_MANIFEST;

    let meta = match cargo_metadata::MetadataCommand::new()
        .manifest_path(manifest_path.to_owned())
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

    let Some(package) = meta
        .packages
        .iter()
        .find(|package| package.manifest_path == manifest_path)
    else {
        panic!(
            "couldn't find the root package for manifest at {manifest_path:?}"
        )
    };

    package.clone()
}

/// Turns the given absolute path to a file or directory within the workspace
/// into the corresponding relative path to the $CARGO_MANIFEST_DIR.
fn relative_to_manifest_dir(workspace_path: &Path) -> PathBuf {
    let num_levels_from_root_to_manifest_dir = CARGO_MANIFEST_DIR
        .strip_prefix(&**WORKSPACE_ROOT)
        .expect("$CARGO_MANIFEST_DIR is in workspace")
        .components()
        .count();

    let relative_to_root = workspace_path
        .strip_prefix(&**WORKSPACE_ROOT)
        .expect("path not in workspace");

    iter::repeat_n(OsStr::new(".."), num_levels_from_root_to_manifest_dir)
        .chain(relative_to_root.components().map(path::Component::as_os_str))
        .collect()
}
