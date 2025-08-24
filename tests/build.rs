#![allow(missing_docs)]

use core::cmp::Ordering;
use core::str::FromStr;
use std::env;
use std::process::Command;

fn main() {
    setup_git();

    if env::var("CARGO_FEATURE_NEOVIM").is_ok()
        // Avoid building the Neovim tests if we're running coverage.
        && env::var("CARGO_CFG_COVERAGE").is_err()
    {
        setup_neovim();
    }
}

/// Enables `cfg(git_in_PATH)` if git is in $PATH and its version is at least
/// 2.32.
///
/// We require 2.32 because that was the first release which supported the
/// `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` environment variables that we
/// need to have a reproducible git environment.
///
/// See [this][1] for more infos.
///
/// [1]: https://github.com/git/git/blob/master/Documentation/RelNotes/2.32.0.adoc#updates-since-v231
fn setup_git() {
    println!("cargo::rustc-check-cfg=cfg(git_in_PATH)");

    let maybe_git_version = Command::new("git")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| output.status.success().then_some(output.stdout))
        .and_then(|stdout| String::from_utf8(stdout).ok())
        .and_then(|stdout| stdout.parse::<GitVersion>().ok());

    if let Some(git_version) = maybe_git_version
        && git_version >= GitVersion(2, 32, 0)
    {
        println!("cargo:rustc-cfg=git_in_PATH");
    }
}

fn setup_neovim() {
    // On macOS we need to set these linker flags or nvim-oxi won't build.
    //
    // See https://github.com/rust-lang/rust/issues/62874 for more infos.
    #[cfg(target_os = "macos")]
    {
        println!("cargo:rustc-cdylib-link-arg=-undefined");
        println!("cargo:rustc-cdylib-link-arg=dynamic_lookup");
    }

    neovim::oxi::tests::build().expect("couldn't build neovim tests");
}

#[derive(PartialEq, Eq)]
struct GitVersion(u8, u8, u8);

impl PartialOrd for GitVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GitVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        let Self(this_major, this_minor, this_patch) = self;
        let Self(other_major, other_minor, other_patch) = other;
        this_major
            .cmp(other_major)
            .then(this_minor.cmp(other_minor))
            .then(this_patch.cmp(other_patch))
    }
}

impl FromStr for GitVersion {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn inner(s: &str) -> Option<GitVersion> {
            let (_, semver) = s.split_once("git version ")?;
            let mut versions = semver.trim().split('.');
            let major = versions.next().and_then(|s| s.parse().ok())?;
            let minor = versions.next().and_then(|s| s.parse().ok())?;
            let patch = versions.next().and_then(|s| s.parse().ok())?;
            Some(GitVersion(major, minor, patch))
        }
        inner(s).ok_or(())
    }
}
