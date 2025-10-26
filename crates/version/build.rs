#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use core::ops::Deref;
use std::env;
use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;

use chrono::Datelike;
use git2::Repository;

const COMMIT_HASH_ENV: &str = "COMMIT_HASH";
const COMMIT_UNIX_TIMESTAMP_ENV: &str = "COMMIT_UNIX_TIMESTAMP";
const RELEASE_TAG_ENV: &str = "RELEASE_TAG";

fn main() {
    let mut file = GeneratedFile::default();
    add_commit(&mut file);
    add_tag(&mut file);
    file.create();
}

fn add_commit(file: &mut GeneratedFile) {
    let repo = LazyRepo::default();

    let commit_hash = env::var(COMMIT_HASH_ENV)
        .unwrap_or_else(|_| repo.head_commit().id().to_string());

    let commit_unix_timestamp = env::var(COMMIT_UNIX_TIMESTAMP_ENV)
        .map(|env_var| env_var.parse::<i64>().unwrap())
        .unwrap_or_else(|_| repo.head_commit().time().seconds());

    let commit_date =
        chrono::DateTime::from_timestamp(commit_unix_timestamp, 0)
            .expect("invalid timestamp");

    file.contents.push_str(&format!(
        r#"
pub(crate) const COMMIT: crate::version::Commit = crate::version::Commit {{
    hash: "{commit_hash}",
    date: crate::version::Date {{
        year: {},
        month: {},
        day: {},
    }},
}};
"#,
        commit_date.year(),
        commit_date.month(),
        commit_date.day(),
    ));

    if let Some(repo) = repo.inner() {
        // Trigger a rebuild when new commits are made.
        let head_path = repo.path().join("HEAD");
        println!("cargo:rerun-if-changed={}", head_path.display());

        let head_contents = std::fs::read_to_string(&head_path).unwrap();
        if let Some((_, relative_ref_path)) = head_contents.split_once("ref: ")
        {
            let ref_path = repo.path().join(relative_ref_path.trim());
            println!("cargo:rerun-if-changed={}", ref_path.display());
        }
    }
}

fn add_tag(file: &mut GeneratedFile) {
    let tag_path = "crate::version::ReleaseTag";

    let tag = match env::var(RELEASE_TAG_ENV) {
        Ok(tag) if tag == "nightly" => format!("Some({tag_path}::Nightly)"),

        Ok(tag) => {
            let try_block = || {
                let mut parts = tag.split('.');
                let year = parts.next()?;
                let month = parts.next()?;
                let patch = parts.next()?;
                Some(format!(
                    "Some({tag_path}::Stable {{ year: {year}, month: \
                     {month}, patch: {patch}}})"
                ))
            };
            try_block().unwrap_or_else(|| {
                panic!(
                    "expected ${RELEASE_TAG_ENV} to be formatted as \
                     YYYY.MM.PATCH",
                )
            })
        },

        Err(env::VarError::NotPresent) => "None".to_owned(),

        Err(env::VarError::NotUnicode(_)) => {
            panic!("${RELEASE_TAG_ENV} is not valid unicode")
        },
    };

    file.contents.push_str(&format!(
        "pub(crate) const TAG: Option<{tag_path}> = {tag};",
    ));

    println!("cargo:rerun-if-env-changed={RELEASE_TAG_ENV}");
}

#[derive(Default)]
struct GeneratedFile {
    contents: String,
}

impl GeneratedFile {
    const NAME: &'static str = "generated.rs";

    fn create(self) {
        use std::io::Write;
        out_file(Self::NAME).write_all(self.contents.as_bytes()).unwrap();
    }
}

#[derive(Default)]
struct LazyRepo {
    inner: OnceLock<Repository>,
}

impl LazyRepo {
    fn head_commit(&self) -> git2::Commit<'_> {
        self.head()
            .expect("couldn't get HEAD")
            .peel_to_commit()
            .expect("couldn't get HEAD commit")
    }

    fn inner(&self) -> Option<&Repository> {
        self.inner.get()
    }
}

impl Deref for LazyRepo {
    type Target = Repository;

    fn deref(&self) -> &Self::Target {
        self.inner.get_or_init(|| {
            let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
            Repository::discover(manifest_dir).expect("couldn't find repo")
        })
    }
}

/// Opens the file with the given name in the `OUT_DIR`, or creates a new one
/// if it doesn't exist.
fn out_file(file_name: &str) -> File {
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR is set in build script");
    let out_path = Path::new(&out_dir).join(file_name);
    File::create(&out_path).unwrap_or_else(|err| {
        panic!("couldn't create file at {out_path:?}: {err}")
    })
}
