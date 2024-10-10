use core::ops::Range;
use std::collections::HashMap;

use collab_project::file::Text;
use collab_project::PeerId;
use smol_str::SmolStr;

#[derive(Default)]
pub(crate) struct TextBacklog {
    map: HashMap<PeerId, PeerBacklog>,
}

impl TextBacklog {
    pub(crate) fn insert(&mut self, text: Text, s: &str) {
        self.map
            .entry(text.inserted_by())
            .or_default()
            .insert(text.char_range(), s);
    }

    pub(crate) fn remove(&mut self, text: Text) -> SmolStr {
        let Some(inner) = self.map.get_mut(&text.inserted_by()) else {
            panic!("no backlog for peer");
        };
        inner.remove(text.char_range())
    }
}

#[derive(Default)]
struct PeerBacklog {
    map: HashMap<Range<usize>, SmolStr>,
}

impl PeerBacklog {
    fn insert(&mut self, range: Range<usize>, s: &str) {
        todo!()
    }

    fn remove(&mut self, range: Range<usize>) -> SmolStr {
        todo!()
    }
}
