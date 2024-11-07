use anyhow::anyhow;
use fs::os_fs::OsFs;
use fs::{AbsPath, AbsPathBuf, FsNodeName};
use futures_executor::block_on;
use root_finder::markers;

pub(crate) fn build(_release: bool) -> anyhow::Result<()> {
    let sh = xshell::Shell::new()?;
    let project_root = find_project_root(&sh)?;
    let package_name = parse_package_name(&project_root)?;
    let nvim_version = detect_nvim_version(&sh)?;
    build_plugin(&project_root, &package_name, nvim_version, &sh)?;
    fix_library_name(&project_root, &package_name, &sh)?;
    Ok(())
}

fn find_project_root(sh: &xshell::Shell) -> anyhow::Result<AbsPathBuf> {
    let current_dir = sh.current_dir();
    let current_dir = <&AbsPath>::try_from(&*current_dir)?;
    let root_finder = root_finder::Finder::new(OsFs);
    block_on(root_finder.find_root(current_dir, markers::Git))?
        .ok_or_else(|| anyhow!("Could not find the project root"))
}

fn parse_package_name(project_root: &AbsPath) -> anyhow::Result<String> {
    let cargo_dot_toml = {
        let mut root = project_root.to_owned();
        #[allow(clippy::unwrap_used)]
        root.push(<&FsNodeName>::try_from("Cargo.toml").unwrap());
        root
    };
    let metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(cargo_dot_toml.clone())
        .exec()?;
    metadata.root_package().map(|p| p.name.to_owned()).ok_or_else(|| {
        anyhow!(
            "Could not find the root package for manifest at \
             {cargo_dot_toml:?}"
        )
    })
}

fn detect_nvim_version(sh: &xshell::Shell) -> anyhow::Result<NeovimVersion> {
    todo!();
}

fn build_plugin(
    project_root: &AbsPathBuf,
    package_name: &str,
    nvim_version: NeovimVersion,
    sh: &xshell::Shell,
) -> anyhow::Result<()> {
    todo!();
}

fn fix_library_name(
    project_root: &AbsPathBuf,
    package_name: &str,
    sh: &xshell::Shell,
) -> anyhow::Result<()> {
    todo!();
}

/// The possible Neovim versions the Nomad plugin can be built for.
enum NeovimVersion {
    /// The latest stable version.
    ZeroDotTen,

    /// The latest nightly version.
    Nightly,
}
