#![allow(missing_docs)]

use std::{env, fs};

use abs_path::{AbsPath, AbsPathBuf, node};

/// https://github.com/neovim/neovim/tree/8707ec264462b66ff9243f40365d6d24ed2f7f6d
const NEOVIM_COMMIT: &str = "8707ec264462b66ff9243f40365d6d24ed2f7f6d";

fn main() {
    let out_dir = env::var("OUT_DIR")
        .expect("OUT_DIR is set by cargo")
        .parse::<AbsPathBuf>()
        .expect("OUT_DIR is absolute");

    let mut generated_contents = String::default();

    if env::var("CARGO_FEATURE_COLLAB").is_ok() {
        setup_collab(&out_dir, &mut generated_contents);
    }

    fs::write(out_dir.join(node!("generated.rs")), generated_contents)
        .expect("couldn't write generated.rs");
}

/// Sets up the environment needed to run collab-related benchmarks.
fn setup_collab(out_dir: &AbsPath, generated_file: &mut String) {
    println!("cargo::rustc-check-cfg=cfg(neovim_repo)");

    generated_file.push_str("pub(crate) mod collab {");

    match checkout_neovim_commit(out_dir) {
        Ok(repo_path_declaration) => {
            generated_file.push_str(&repo_path_declaration);
            println!("cargo::rustc-cfg=neovim_repo")
        },
        Err(err) => {
            println!(
                "cargo::warning=\"couldn't check out the neovim repo, \
                 benchmarks that depend on it are disabled: {err}\""
            );
        },
    }

    generated_file.push_str("}\n");
}

/// Clones the Neovim repository into the `OUT_DIR` and checks out a specific
/// commit, or does nothing if it already exists.
fn checkout_neovim_commit(_out_dir: &AbsPath) -> anyhow::Result<String> {
    anyhow::bail!("cloning Neovim {NEOVIM_COMMIT} takes too much time..")
    //     let repo_path = out_dir.join(node!("neovim"));
    //
    //     let repo = if Path::new(repo_path.as_str()).exists() {
    //         Repository::open(&repo_path)?
    //     } else {
    //         Repository::clone("https://github.com/neovim/neovim.git", &repo_path)?
    //     };
    //
    //     let commit_obj = repo.revparse_single(NEOVIM_COMMIT)?;
    //     repo.checkout_tree(&commit_obj, Some(CheckoutBuilder::new().force()))?;
    //     repo.set_head_detached(commit_obj.id())?;
    //
    //     Ok(format!(
    //         r#"
    // #[cfg(neovim_repo)]
    // pub(crate) const NEOVIM_REPO_PATH: &::abs_path::AbsPath =
    //     unsafe {{ ::abs_path::AbsPath::from_str_unchecked("{repo_path}") }};
    // "#
    //     ))
}
