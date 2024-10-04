use collab_fs::{AbsUtf8PathBuf, OsFs};
use nohash::IntMap as NoHashMap;

use super::{Api, Buffer, BufferId, ModuleApi, NeovimSpawner};
use crate::{ActorId, Editor, Shared};

/// TODO: docs.
#[derive(Default)]
pub struct Neovim {
    /// TODO: docs.
    actor_ids: NoHashMap<BufferId, Shared<Option<ActorId>>>,
}

impl Editor for Neovim {
    type Api = Api;
    type Buffer<'ed> = Buffer;
    type Fs = OsFs;
    type ModuleApi = ModuleApi;
    type Spawner = NeovimSpawner;

    fn current_buffer(&mut self) -> Option<Self::Buffer<'_>> {
        self.get_buffer(BufferId::new(nvim_oxi::api::Buffer::current()))
    }

    fn get_buffer(&mut self, buffer_id: BufferId) -> Option<Self::Buffer<'_>> {
        if !buffer_id.is_of_text_buffer() {
            return None;
        }
        let actor_id = self.actor_ids.entry(buffer_id.clone()).or_default();
        Some(Buffer::new(buffer_id, actor_id.clone()))
    }

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
            unimplemented!()
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
