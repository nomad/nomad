#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use core::ops::Deref;
use std::env;
use std::fmt::Write;
use std::fs::File;
use std::path::Path;
use std::sync::OnceLock;

use chrono::Datelike;
use git2::Repository;

fn main() {
    let mut file = GeneratedFile::default();
    add_commit_infos(&mut file);
    add_version_infos(&mut file);
    file.create();
}

fn add_commit_infos(file: &mut GeneratedFile) {
    let repo = LazyRepo::default();

    let commit_hash = env::var("COMMIT_HASH")
        .unwrap_or_else(|_| repo.head_commit().id().to_string());

    let commit_unix_timestamp = env::var("COMMIT_UNIX_TIMESTAMP")
        .map(|env_var| env_var.parse::<i64>().unwrap())
        .unwrap_or_else(|_| repo.head_commit().time().seconds());

    let commit_date =
        chrono::DateTime::from_timestamp(commit_unix_timestamp, 0)
            .expect("invalid timestamp");

    file.add_const("COMMIT_SHORT_HASH", &commit_hash[..7])
        .add_const("COMMIT_YEAR", commit_date.year() as u16)
        .add_const("COMMIT_MONTH", commit_date.month() as u8)
        .add_const("COMMIT_DAY", commit_date.day() as u8);

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

fn add_version_infos(file: &mut GeneratedFile) {
    let major = env::var("CARGO_PKG_VERSION_MAJOR").unwrap();
    let minor = env::var("CARGO_PKG_VERSION_MINOR").unwrap();
    let patch = env::var("CARGO_PKG_VERSION_PATCH").unwrap();
    let pre = env::var("CARGO_PKG_VERSION_PRE").unwrap();

    file.add_const("MAJOR", major.parse::<u8>().unwrap())
        .add_const("MINOR", minor.parse::<u8>().unwrap())
        .add_const("PATCH", patch.parse::<u8>().unwrap())
        .add_const("PRE", (!pre.is_empty()).then_some(&*pre));
}

#[derive(Default)]
struct GeneratedFile {
    contents: String,
}

impl GeneratedFile {
    const NAME: &'static str = "generated.rs";

    fn add_const<T>(&mut self, name: &str, value: T) -> &mut Self
    where
        T: DisplayType,
    {
        write!(&mut self.contents, "pub(crate) const {name}: ").unwrap();
        T::display_type(&mut self.contents);
        self.contents.push_str(" = ");
        T::display_value(&value, &mut self.contents);
        self.contents.push_str(";\n");
        self
    }

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

trait DisplayType {
    fn display_type(buf: &mut String);
    fn display_value(&self, buf: &mut String);
}

impl DisplayType for u8 {
    fn display_type(buf: &mut String) {
        buf.push_str("u8");
    }

    fn display_value(&self, buf: &mut String) {
        write!(buf, "{self}").unwrap()
    }
}

impl DisplayType for u16 {
    fn display_type(buf: &mut String) {
        buf.push_str("u16");
    }

    fn display_value(&self, buf: &mut String) {
        write!(buf, "{self}").unwrap()
    }
}

impl DisplayType for &str {
    fn display_type(buf: &mut String) {
        buf.push_str("&str");
    }

    fn display_value(&self, buf: &mut String) {
        write!(buf, "\"{self}\"").unwrap()
    }
}

impl<T: DisplayType> DisplayType for Option<T> {
    fn display_type(buf: &mut String) {
        buf.push_str("Option<");
        T::display_type(buf);
        buf.push('>');
    }

    fn display_value(&self, buf: &mut String) {
        match self {
            Some(value) => {
                write!(buf, "Some(").unwrap();
                value.display_value(buf);
                write!(buf, ")").unwrap();
            },
            None => {
                buf.push_str("None");
            },
        }
    }
}
