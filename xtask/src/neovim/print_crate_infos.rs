use cargo_metadata::semver;

use crate::neovim::ENTRYPOINT_METADATA;

pub(super) fn run() {
    let meta = &ENTRYPOINT_METADATA;
    let infos = CrateInfos::from(&**meta);
    let json = serde_json::to_string(&infos).expect("never fails");
    println!("{json}");
}

#[derive(serde::Serialize)]
struct CrateInfos<'meta> {
    name: &'meta str,
    version: &'meta semver::Version,
}

impl<'meta> From<&'meta cargo_metadata::Package> for CrateInfos<'meta> {
    fn from(meta: &'meta cargo_metadata::Package) -> Self {
        Self { name: meta.name.as_str(), version: &meta.version }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn run() {
        super::run();
    }
}
