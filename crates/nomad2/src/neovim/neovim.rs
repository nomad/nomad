use collab_fs::{AbsUtf8PathBuf, OsFs};

use super::{Api, ModuleApi, NeovimSpawner};
use crate::Editor;

/// TODO: docs.
#[derive(Default)]
pub struct Neovim {}

impl Editor for Neovim {
    type Fs = OsFs;
    type Api = Api;
    type ModuleApi = ModuleApi;
    type Spawner = NeovimSpawner;

    fn fs(&self) -> Self::Fs {
        OsFs::new()
    }

    fn spawner(&self) -> Self::Spawner {
        NeovimSpawner
    }

    fn log_dir(&self) -> AbsUtf8PathBuf {
        #[cfg(target_family = "unix")]
        {
            let mut dir = data_local_dir();
            dir.push("nvim");
            dir.push("nomad");
            dir.push("logs");
            dir
        }
        #[cfg(not(target_family = "unix"))]
        {
            todo!("")
        }
    }
}

#[cfg(target_family = "unix")]
fn data_local_dir() -> AbsUtf8PathBuf {
    match home::home_dir() {
        Some(home) if !home.as_os_str().is_empty() => {
            AbsUtf8PathBuf::from_path_buf(home.join(".local").join("share"))
                .expect("home is absolute")
        },
        _ => panic!("failed to get the home directory"),
    }
}
