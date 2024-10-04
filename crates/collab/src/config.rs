use collab_fs::AbsUtf8PathBuf;

#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {}

impl Config {
    pub(crate) fn nomad_dir(&self) -> AbsUtf8PathBuf {
        todo!();
    }
}
