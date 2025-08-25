use collab_types::annotation::AnnotationDeletion;
use collab_types::binary::BinaryEdit;
use collab_types::text::{
    CursorCreation,
    CursorMove,
    CursorRemoval,
    SelectionCreation,
    SelectionMove,
    SelectionRemoval,
    TextEdit,
};
use collab_types::{PeerId, puff};
use puff::directory::{GlobalDirectoryId, LocalDirectoryId};
use puff::file::{GlobalFileId, LocalFileId};
use smallvec::SmallVec;

use crate::abs_path::AbsPath;
use crate::{ProjectBuilder, binary, fs, text};

/// TODO: docs.
#[derive(Clone)]
pub struct Project {
    backlogs: Backlogs,
    contexts: Contexts,
    fs: fs::Fs,
}

/// An error returned when trying to acquire a mutable reference to some
/// resource (like cursors or selections) that is not owned by the local peer.
pub struct LocalPeerIsNotOwnerError;

/// TODO: docs.
#[derive(Debug, derive_more::Display, cauchy::From, cauchy::Error)]
#[cfg(feature = "serde")]
#[display("{inner}")]
pub struct DecodeError {
    inner: bincode::error::DecodeError,
}

/// TODO: docs.
pub(crate) struct State<'proj> {
    contexts: &'proj Contexts,
    peer_id: PeerId,
}

/// TODO: docs.
pub(crate) struct StateMut<'proj> {
    backlogs: &'proj mut Backlogs,
    contexts: &'proj mut Contexts,
    peer_id: PeerId,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Backlogs {
    binary: binary::BinaryEditBacklog,
    text: text::TextEditBacklog,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
struct Contexts {
    binary: binary::BinaryCtx,
    text: text::TextCtx,
}

impl Project {
    /// TODO: docs.
    #[inline]
    pub fn builder(peer_id: PeerId) -> ProjectBuilder {
        ProjectBuilder {
            inner: fs::FsBuilder::new(peer_id.into()),
            binary_ctx: binary::BinaryCtx::default(),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn cursor(
        &self,
        cursor_id: text::CursorId,
    ) -> Option<text::CursorRef<'_>> {
        text::CursorRef::from_id(cursor_id, self)
    }

    /// TODO: docs.
    #[inline]
    pub fn cursor_mut(
        &mut self,
        cursor_id: text::CursorId,
    ) -> Result<Option<text::CursorMut<'_>>, LocalPeerIsNotOwnerError> {
        if cursor_id.owner() == self.peer_id() {
            Ok(text::CursorMut::from_id(cursor_id, self))
        } else {
            Err(LocalPeerIsNotOwnerError)
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn cursors(&self) -> text::Cursors<'_> {
        text::Cursors::new(self)
    }

    /// TODO: docs.
    #[cfg(feature = "serde")]
    pub fn decode(
        encoded_buf: &[u8],
        local_id: PeerId,
    ) -> Result<Self, DecodeError> {
        let (proj, num_read) = bincode::serde::seed_decode_from_slice(
            Self::deserialize(local_id),
            encoded_buf,
            Self::bincode_config(),
        )?;
        assert_eq!(num_read, encoded_buf.len());
        Ok(proj)
    }

    /// TODO: docs.
    #[inline]
    pub fn directory(
        &self,
        directory_id: LocalDirectoryId,
    ) -> Option<fs::Directory<'_>> {
        match self.fs.directory(directory_id) {
            puff::directory::DirectoryState::Visible(directory) => {
                Some(fs::Directory::new(directory, self.state()))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn directory_mut(
        &mut self,
        directory_id: LocalDirectoryId,
    ) -> Option<fs::DirectoryMut<'_>> {
        let (state, fs) = self.state_mut();

        match fs.directory_mut(directory_id) {
            puff::directory::DirectoryMutState::Visible(directory) => {
                Some(fs::DirectoryMut::new(directory, state))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[cfg(feature = "serde")]
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode_into(&mut buf);
        buf
    }

    /// TODO: docs.
    #[cfg(feature = "serde")]
    pub fn encode_into(&self, buf: &mut impl std::io::Write) {
        match bincode::serde::encode_into_std_write(
            self.serialize().with_fs_state(true),
            buf,
            Self::bincode_config(),
        ) {
            Ok(_num_written) => (),
            Err(err) => panic!("encoding should be infallible, but got {err}"),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn file(&self, file_id: LocalFileId) -> Option<fs::File<'_>> {
        match self.fs.file(file_id) {
            puff::file::FileState::Visible(file) => {
                Some(fs::File::new(file, self.state()))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn file_mut(
        &mut self,
        file_id: LocalFileId,
    ) -> Option<fs::FileMut<'_>> {
        let (state, fs) = self.state_mut();

        match fs.file_mut(file_id) {
            puff::file::FileMutState::Visible(file) => {
                Some(fs::FileMut::new(file, state))
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[cfg(feature = "mock")]
    #[inline]
    pub fn from_mock(
        peer_id: PeerId,
        directory: mock::fs::MockDirectory,
    ) -> Self {
        use core::pin::pin;

        use ::fs::{Directory as _, File as _, Node, Symlink as _};
        use futures_lite::FutureExt;

        async fn push_dir(
            dir: mock::fs::MockDirectory,
            builder: &mut ProjectBuilder,
        ) -> Result<(), Box<dyn core::error::Error>> {
            use futures_lite::StreamExt;

            let mut stream = pin!(dir.list_nodes().await?);

            while let Some(node_res) = stream.next().await {
                match node_res? {
                    Node::File(file) => {
                        let contents = file.read().await?;
                        match str::from_utf8(&contents) {
                            Ok(str) => {
                                builder.push_text_file(file.path(), str)?;
                            },
                            Err(_) => {
                                builder
                                    .push_binary_file(file.path(), contents)?;
                            },
                        }
                    },
                    Node::Directory(dir) => {
                        builder.push_directory(dir.path())?;
                        push_dir(dir, builder).boxed_local().await?;
                    },
                    Node::Symlink(symlink) => {
                        let target_path = symlink.read_path().await?;
                        builder.push_symlink(symlink.path(), target_path)?;
                    },
                }
            }

            Ok(())
        }

        let mut builder = Self::builder(peer_id);

        if let Err(err) =
            futures_lite::future::block_on(push_dir(directory, &mut builder))
        {
            panic!("{err}");
        }

        builder.build()
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_binary_edit(
        &mut self,
        binary_edit: BinaryEdit,
    ) -> Option<binary::BinaryFileMut<'_>> {
        let Some(file_id) =
            self.fs.local_file_id_of_global_id(binary_edit.file_id)
        else {
            self.backlogs.binary.insert(binary_edit);
            return None;
        };

        let mut file_state = self
            .binary_file_mut(file_id)
            .expect("BinaryEdit can only be created by a BinaryFile");

        let did_change = file_state.integrate_edit(binary_edit);

        match file_state {
            binary::BinaryStateMut::Visible(file) => {
                did_change.then_some(file)
            },
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_cursor_creation(
        &mut self,
        cursor_creation: CursorCreation,
    ) -> Option<text::CursorRef<'_>> {
        let cursor = self
            .contexts
            .text
            .cursors
            .integrate_creation(cursor_creation, &self.fs)?;

        text::CursorRef::from_id(cursor.id().into(), self)
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_cursor_removal(
        &mut self,
        cursor_removal: CursorRemoval,
    ) -> Option<text::CursorId> {
        self.contexts
            .text
            .cursors
            .integrate_deletion(cursor_removal)
            .map(|(annotation_id, _)| annotation_id.into())
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_cursor_move(
        &mut self,
        cursor_move: CursorMove,
    ) -> Option<text::CursorRef<'_>> {
        let (cursor, was_updated) =
            self.contexts.text.cursors.integrate_op(cursor_move)?;

        if !was_updated {
            return None;
        }

        text::CursorRef::from_id(cursor.id().into(), self)
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_fs_op(
        &mut self,
        op: impl fs::FsOp,
    ) -> fs::SyncActions<'_> {
        op.integrate_into(self)
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_peer_disconnection(
        &mut self,
        peer_id: PeerId,
    ) -> (
        impl IntoIterator<Item = text::CursorId>,
        impl IntoIterator<Item = text::SelectionId>,
    ) {
        let deleted_cursors = self
            .cursors()
            .filter_map(|cursor| {
                (cursor.owner() == peer_id).then_some(cursor.id())
            })
            .collect::<SmallVec<[_; 2]>>();

        let deleted_selections = self
            .selections()
            .filter_map(|selection| {
                (selection.owner() == peer_id).then_some(selection.id())
            })
            .collect::<SmallVec<[_; 2]>>();

        for &cursor_id in &deleted_cursors {
            let deletion =
                AnnotationDeletion { annotation_id: cursor_id.into() };
            self.contexts.text.cursors.integrate_deletion(deletion);
        }

        for &selection_id in &deleted_selections {
            let deletion =
                AnnotationDeletion { annotation_id: selection_id.into() };
            self.contexts.text.selections.integrate_deletion(deletion);
        }

        (deleted_cursors, deleted_selections)
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_selection_creation(
        &mut self,
        selection_creation: SelectionCreation,
    ) -> Option<text::SelectionRef<'_>> {
        let selection = self
            .contexts
            .text
            .selections
            .integrate_creation(selection_creation, &self.fs)?;

        text::SelectionRef::from_id(selection.id().into(), self)
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_selection_removal(
        &mut self,
        selection_removal: SelectionRemoval,
    ) -> Option<text::SelectionId> {
        self.contexts
            .text
            .selections
            .integrate_deletion(selection_removal)
            .map(|(annotation_id, _)| annotation_id.into())
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_selection_move(
        &mut self,
        selection_move: SelectionMove,
    ) -> Option<text::SelectionRef<'_>> {
        let (selection, was_updated) =
            self.contexts.text.selections.integrate_op(selection_move)?;

        if !was_updated {
            return None;
        }

        text::SelectionRef::from_id(selection.id().into(), self)
    }

    /// TODO: docs.
    #[inline]
    pub fn integrate_text_edit(
        &mut self,
        text_edit: TextEdit,
    ) -> Option<(text::TextFileMut<'_>, text::TextReplacements)> {
        let Some(file_id) =
            self.fs.local_file_id_of_global_id(text_edit.file_id)
        else {
            self.backlogs.text.insert(text_edit);
            return None;
        };

        let mut file_state = self
            .text_file_mut(file_id)
            .expect("TextEdit can only be created by a TextFile");

        let replacements = file_state.integrate_edit(text_edit);

        match file_state {
            text::TextStateMut::Visible(file) => Some((file, replacements)),
            _ => None,
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn local_directory_of_global(
        &self,
        global_id: GlobalDirectoryId,
    ) -> Option<LocalDirectoryId> {
        self.fs.local_directory_id_of_global_id(global_id)
    }

    /// TODO: docs.
    #[inline]
    pub fn local_file_of_global(
        &self,
        global_id: GlobalFileId,
    ) -> Option<LocalFileId> {
        self.fs.local_file_id_of_global_id(global_id)
    }

    /// TODO: docs.
    #[inline]
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            backlogs: Backlogs::default(),
            contexts: Contexts::default(),
            fs: fs::Fs::new((), peer_id.into()),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn node_at_path(&self, path: &AbsPath) -> Option<fs::Node<'_>> {
        match self.fs.node_at_path(path)? {
            puff::node::Node::Directory(directory) => {
                Some(fs::Node::Directory(fs::Directory::new(
                    directory,
                    self.state(),
                )))
            },
            puff::node::Node::File(file) => {
                Some(fs::Node::File(fs::File::new(file, self.state())))
            },
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn node_at_path_mut(
        &mut self,
        path: &AbsPath,
    ) -> Option<fs::NodeMut<'_>> {
        let (state, fs) = self.state_mut();

        match fs.node_at_path_mut(path)? {
            puff::node::NodeMut::Directory(directory) => {
                Some(fs::NodeMut::Directory(fs::DirectoryMut::new(
                    directory, state,
                )))
            },
            puff::node::NodeMut::File(file) => {
                Some(fs::NodeMut::File(fs::FileMut::new(file, state)))
            },
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn peer_id(&self) -> PeerId {
        self.fs.peer_id().into()
    }

    /// TODO: docs.
    #[inline]
    pub fn root(&self) -> fs::Directory<'_> {
        fs::Directory::new(self.fs.root(), self.state())
    }

    /// TODO: docs.
    #[inline]
    pub fn root_mut(&mut self) -> fs::DirectoryMut<'_> {
        let (state, fs) = self.state_mut();
        fs::DirectoryMut::new(fs.root_mut(), state)
    }

    /// TODO: docs.
    #[inline]
    pub fn selection(
        &self,
        selection_id: text::SelectionId,
    ) -> Option<text::SelectionRef<'_>> {
        text::SelectionRef::from_id(selection_id, self)
    }

    /// TODO: docs.
    #[inline]
    pub fn selection_mut(
        &mut self,
        selection_id: text::SelectionId,
    ) -> Result<Option<text::SelectionMut<'_>>, LocalPeerIsNotOwnerError> {
        if selection_id.owner() == self.peer_id() {
            Ok(text::SelectionMut::from_id(selection_id, self))
        } else {
            Err(LocalPeerIsNotOwnerError)
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn selections(&self) -> text::Selections<'_> {
        text::Selections::new(self)
    }

    #[inline]
    pub(crate) fn from_builder(builder: ProjectBuilder) -> Self {
        Self {
            backlogs: Backlogs::default(),
            contexts: Contexts {
                binary: builder.binary_ctx,
                text: text::TextCtx::default(),
            },
            fs: builder.inner.build(),
        }
    }

    #[inline]
    pub(crate) fn state(&self) -> State<'_> {
        State { contexts: &self.contexts, peer_id: self.peer_id() }
    }

    #[inline]
    pub(crate) fn state_mut(&mut self) -> (StateMut<'_>, &mut fs::Fs) {
        let peer_id = self.peer_id();
        let state = StateMut {
            backlogs: &mut self.backlogs,
            contexts: &mut self.contexts,
            peer_id,
        };
        (state, &mut self.fs)
    }

    #[inline]
    pub(crate) fn text_ctx(&self) -> &text::TextCtx {
        &self.contexts.text
    }

    #[inline]
    pub(crate) fn text_ctx_mut(&mut self) -> &mut text::TextCtx {
        &mut self.contexts.text
    }

    #[inline]
    pub(crate) fn fs(&self) -> &fs::Fs {
        &self.fs
    }

    #[inline]
    pub(crate) fn fs_mut(&mut self) -> &mut fs::Fs {
        &mut self.fs
    }

    #[inline]
    fn binary_file_mut(
        &mut self,
        file_id: LocalFileId,
    ) -> Option<binary::BinaryStateMut<'_>> {
        let (state, fs) = self.state_mut();
        binary::BinaryStateMut::new(fs.file_mut(file_id), state)
    }

    #[cfg(feature = "serde")]
    fn bincode_config() -> impl bincode::config::Config {
        bincode::config::standard()
    }

    #[inline]
    fn text_file_mut(
        &mut self,
        file_id: LocalFileId,
    ) -> Option<text::TextStateMut<'_>> {
        let (state, fs) = self.state_mut();
        text::TextStateMut::new(fs.file_mut(file_id), state)
    }
}

#[cfg(feature = "mock")]
impl From<&Project> for mock::fs::MockFs {
    #[track_caller]
    #[inline]
    fn from(proj: &Project) -> Self {
        use ::fs::{Directory as _, File as _, Fs as _};
        use futures_lite::FutureExt;

        use crate::fs::{Directory, File, Node};

        async fn push_directory(
            dir: Directory<'_>,
            fs: &mut mock::fs::MockFs,
        ) -> Result<(), Box<dyn core::error::Error>> {
            let parent = fs.dir(dir.path()).await?;

            for child in dir.children() {
                match child {
                    Node::Directory(directory) => {
                        let name = directory.try_name().expect("not root");
                        parent.create_directory(name).await?;
                        push_directory(directory, fs).boxed_local().await?;
                    },
                    Node::File(file) => match file {
                        File::Binary(binary) => {
                            parent
                                .create_file(file.name())
                                .await?
                                .write(binary.contents())
                                .await?;
                        },
                        File::Symlink(symlink) => {
                            let target_path = symlink.target_path();
                            parent
                                .create_symlink(file.name(), target_path)
                                .await?;
                        },
                        File::Text(text) => {
                            parent
                                .create_file(file.name())
                                .await?
                                .write_chunks(text.contents().chunks())
                                .await?;
                        },
                    },
                }
            }

            Ok(())
        }

        let mut fs = mock::fs::MockFs::default();

        if let Err(err) = futures_lite::future::block_on(push_directory(
            proj.root(),
            &mut fs,
        )) {
            panic!("{err}");
        }

        fs
    }
}

impl<'proj> State<'proj> {
    /// Returns the [`PeerId`] of the local peer.
    #[inline]
    pub(crate) fn local_id(&self) -> PeerId {
        self.peer_id
    }

    #[inline]
    pub(crate) fn text_ctx(self) -> &'proj text::TextCtx {
        &self.contexts.text
    }
}

impl StateMut<'_> {
    #[inline]
    pub(crate) fn as_ref(&self) -> State<'_> {
        State { contexts: self.contexts, peer_id: self.peer_id }
    }

    #[inline]
    pub(crate) fn binary_backlog_mut(
        &mut self,
    ) -> &mut binary::BinaryEditBacklog {
        &mut self.backlogs.binary
    }

    #[inline]
    pub(crate) fn binary_ctx_mut(&mut self) -> &mut binary::BinaryCtx {
        &mut self.contexts.binary
    }

    /// Returns the [`PeerId`] of the local peer.
    #[inline]
    pub(crate) fn local_id(&self) -> PeerId {
        self.peer_id
    }

    #[inline]
    pub(crate) fn reborrow(&mut self) -> StateMut<'_> {
        StateMut {
            backlogs: self.backlogs,
            contexts: self.contexts,
            peer_id: self.peer_id,
        }
    }

    #[inline]
    pub(crate) fn text_backlog_mut(&mut self) -> &mut text::TextEditBacklog {
        &mut self.backlogs.text
    }

    #[inline]
    pub(crate) fn text_ctx_mut(&mut self) -> &mut text::TextCtx {
        &mut self.contexts.text
    }
}

impl Copy for State<'_> {}

impl Clone for State<'_> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

#[cfg(feature = "serde")]
mod serde_impls {
    use core::fmt;

    use serde::de;
    use serde::ser::{self, SerializeStruct};

    use super::*;

    impl Project {
        #[inline]
        pub(super) fn deserialize(peer_id: PeerId) -> DeserializeProject {
            DeserializeProject::new(peer_id)
        }

        #[inline]
        pub(super) fn serialize(&self) -> SerializeProject<'_> {
            SerializeProject::new(self)
        }
    }

    /// TODO: docs.
    pub(super) struct SerializeProject<'proj> {
        backlogs: &'proj Backlogs,
        contexts: &'proj Contexts,
        fs: fs::SerializeFs<'proj>,
    }

    /// TODO: docs.
    pub(super) struct DeserializeProject {
        inner: fs::DeserializeFs,
    }

    #[derive(serde::Deserialize)]
    #[serde(field_identifier, rename_all = "snake_case")]
    enum ProjectField {
        Fs,
        Contexts,
        Backlogs,
    }

    impl<'proj> SerializeProject<'proj> {
        const NAME: &'static str = "SerializeProject";

        #[inline]
        pub(super) fn with_fs_state(self, with_fs_state: bool) -> Self {
            Self { fs: self.fs.with_fs_state(with_fs_state), ..self }
        }

        #[inline]
        fn new(proj: &'proj Project) -> Self {
            Self {
                backlogs: &proj.backlogs,
                contexts: &proj.contexts,
                fs: proj.fs.serialize(),
            }
        }
    }

    impl DeserializeProject {
        #[inline]
        fn new(peer_id: PeerId) -> Self {
            Self { inner: fs::Fs::deserialize(peer_id.into()) }
        }

        #[inline]
        fn peer_id(&self) -> PeerId {
            PeerId::new(self.inner.peer_id())
        }
    }

    impl ProjectField {
        const AS_SLICE: &'static [&'static str] = &[
            Self::Fs.as_str(),
            Self::Contexts.as_str(),
            Self::Backlogs.as_str(),
        ];

        #[inline]
        const fn as_str(&self) -> &'static str {
            match self {
                Self::Fs => "fs",
                Self::Contexts => "contexts",
                Self::Backlogs => "backlogs",
            }
        }
    }

    impl Copy for SerializeProject<'_> {}

    impl Clone for SerializeProject<'_> {
        #[inline]
        fn clone(&self) -> Self {
            *self
        }
    }

    impl ser::Serialize for SerializeProject<'_> {
        #[inline]
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: ser::Serializer,
        {
            let mut proj = serializer
                .serialize_struct(Self::NAME, ProjectField::AS_SLICE.len())?;

            proj.serialize_field(ProjectField::Fs.as_str(), &self.fs)?;

            proj.serialize_field(
                ProjectField::Contexts.as_str(),
                &self.contexts,
            )?;

            proj.serialize_field(
                ProjectField::Backlogs.as_str(),
                &self.backlogs,
            )?;

            proj.end()
        }
    }

    impl Copy for DeserializeProject {}

    impl Clone for DeserializeProject {
        #[inline]
        fn clone(&self) -> Self {
            *self
        }
    }

    impl<'de> de::DeserializeSeed<'de> for DeserializeProject {
        type Value = Project;

        #[inline]
        fn deserialize<Ds>(
            self,
            deserializer: Ds,
        ) -> Result<Self::Value, Ds::Error>
        where
            Ds: de::Deserializer<'de>,
        {
            text::serde_impls::LOCAL_PEER_ID.set(Some(self.peer_id()));

            deserializer.deserialize_struct(
                SerializeProject::NAME,
                ProjectField::AS_SLICE,
                self,
            )
        }
    }

    impl<'de> de::Visitor<'de> for DeserializeProject {
        type Value = Project;

        #[inline]
        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            write!(
                formatter,
                "a map representing a {}",
                SerializeProject::NAME
            )
        }

        #[inline]
        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let mut fs: Option<fs::Fs> = None;
            let mut contexts: Option<Contexts> = None;
            let mut backlogs: Option<Backlogs> = None;

            while let Some(fs_field) = map.next_key::<ProjectField>()? {
                match fs_field {
                    ProjectField::Fs => {
                        fs = Some(map.next_value_seed(self.inner)?);
                    },
                    ProjectField::Contexts => {
                        contexts = Some(map.next_value()?);
                    },
                    ProjectField::Backlogs => {
                        backlogs = Some(map.next_value()?);
                    },
                }
            }

            let fs = fs.ok_or_else(|| {
                de::Error::missing_field(ProjectField::Fs.as_str())
            })?;

            let contexts = contexts.ok_or_else(|| {
                de::Error::missing_field(ProjectField::Contexts.as_str())
            })?;

            let backlogs = backlogs.ok_or_else(|| {
                de::Error::missing_field(ProjectField::Backlogs.as_str())
            })?;

            Ok(Project { fs, contexts, backlogs })
        }

        #[inline]
        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            let fs = seq
                .next_element_seed::<fs::DeserializeFs>(self.inner)?
                .ok_or_else(|| de::Error::invalid_length(0, &self))?;

            let contexts = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(1, &self))?;

            let backlogs = seq
                .next_element()?
                .ok_or_else(|| de::Error::invalid_length(2, &self))?;

            if seq.next_element::<de::IgnoredAny>()?.is_some() {
                return Err(de::Error::invalid_length(4, &self));
            }

            Ok(Project { fs, contexts, backlogs })
        }
    }
}
