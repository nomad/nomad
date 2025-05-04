use core::convert::Infallible;
use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::sync::{Arc, Mutex};

use abs_path::{AbsPath, AbsPathBuf, NodeName, NodeNameBuf};
use cauchy::PartialEq;
use ed_core::ByteOffset;
use ed_core::fs::{
    self,
    Directory,
    DirectoryEvent,
    FileEvent,
    Fs,
    FsEvent,
    NodeKind,
    SymlinkEvent,
};
use futures_lite::Stream;
use fxhash::FxHashMap;
use indexmap::IndexMap;

/// TODO: docs.
#[derive(Clone, Default)]
pub struct MockFs {
    inner: Arc<Mutex<FsInner>>,
}

pub struct MockDirectory {
    fs: MockFs,
    metadata: MockMetadata,
    path: AbsPathBuf,
}

pub struct MockFile {
    fs: MockFs,
    metadata: MockMetadata,
    path: AbsPathBuf,
}

pub struct MockSymlink {
    fs: MockFs,
    metadata: MockMetadata,
    path: AbsPathBuf,
}

#[derive(Debug, Clone)]
pub struct MockMetadata {
    byte_len: ByteOffset,
    created_at: MockTimestamp,
    last_modified_at: MockTimestamp,
    name: NodeNameBuf,
    node_id: MockNodeId,
    node_kind: NodeKind,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MockTimestamp(u64);

#[derive(
    Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
pub struct MockNodeId(u64);

pin_project_lite::pin_project! {
    pub struct ReadDir {
        dir_handle: MockDirectory,
        next_child_idx: usize,
    }
}

pin_project_lite::pin_project! {
    pub struct Watcher {
        fs: MockFs,
        path: AbsPathBuf,
        #[pin]
        inner: async_broadcast::Receiver<FsEvent<MockTimestamp>>,
    }

    impl PinnedDrop for Watcher {
        fn drop(this: Pin<&mut Self>) {
            this.fs.with_inner(|inner| inner.watchers.remove(&this.path));
        }
    }
}

struct FsInner {
    next_node_id: MockNodeId,
    root: Node,
    timestamp: MockTimestamp,
    watchers: FxHashMap<AbsPathBuf, WatchChannel>,
}

#[derive(Debug, cauchy::PartialEq, cauchy::Eq)]
#[doc(hidden)]
pub struct DirectoryInner {
    children: IndexMap<NodeNameBuf, Node>,
    #[partial_eq(skip)]
    metadata: MockMetadata,
}

#[derive(Debug, cauchy::PartialEq, cauchy::Eq)]
#[doc(hidden)]
pub struct FileInner {
    contents: Vec<u8>,
    #[partial_eq(skip)]
    metadata: MockMetadata,
}

#[derive(Debug, cauchy::PartialEq, cauchy::Eq)]
#[doc(hidden)]
pub struct SymlinkInner {
    target_path: String,
    #[partial_eq(skip)]
    metadata: MockMetadata,
}

#[derive(Debug, PartialEq, Eq, cauchy::From)]
#[doc(hidden)]
pub enum Node {
    File(#[from] FileInner),
    Directory(#[from] DirectoryInner),
    Symlink(#[from] SymlinkInner),
}

struct WatchChannel {
    inactive_rx: async_broadcast::InactiveReceiver<FsEvent<MockTimestamp>>,
    tx: async_broadcast::Sender<FsEvent<MockTimestamp>>,
}

impl MockFs {
    /// Returns a handle to the root of the filesystem.
    pub fn root(&self) -> MockDirectory {
        MockDirectory {
            fs: self.clone(),
            metadata: self.with_inner(|inner| inner.root().metadata.clone()),
            path: AbsPathBuf::root(),
        }
    }

    #[doc(hidden)]
    pub fn new(root: DirectoryInner) -> Self {
        Self { inner: Arc::new(Mutex::new(FsInner::new(root))) }
    }

    fn delete_node(&self, path: &AbsPath) -> Result<(), DeleteNodeError> {
        self.with_inner(|inner| inner.delete_node(path))
    }

    fn next_node_id(&self) -> MockNodeId {
        self.with_inner(|inner| inner.next_node_id.post_inc())
    }

    #[allow(clippy::unwrap_used)]
    fn with_inner<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut FsInner) -> T,
    {
        let mut inner = self.inner.lock().unwrap();
        f(&mut inner)
    }
}

impl MockDirectory {
    fn create_node(
        &self,
        node_name: &NodeName,
        node_kind: NodeKind,
        target_path: Option<&str>,
    ) -> Result<MockMetadata, CreateNodeError> {
        let metadata = MockMetadata {
            byte_len: target_path.map(|p| p.len().into()).unwrap_or_default(),
            created_at: self.fs.now(),
            last_modified_at: self.fs.now(),
            name: node_name.to_owned(),
            node_id: self.fs.next_node_id(),
            node_kind,
        };

        let node = match node_kind {
            NodeKind::File => Node::File(FileInner {
                contents: Vec::new(),
                metadata: metadata.clone(),
            }),
            NodeKind::Directory => Node::Directory(DirectoryInner {
                children: Default::default(),
                metadata: metadata.clone(),
            }),
            NodeKind::Symlink => Node::Symlink(SymlinkInner {
                target_path: target_path
                    .expect("target_path is set for symlinks")
                    .to_owned(),
                metadata: metadata.clone(),
            }),
        };

        self.with_inner(|dir| dir.create_node(&self.path, node_name, node))??;

        Ok(metadata)
    }

    fn with_inner<T>(
        &self,
        f: impl FnOnce(&mut DirectoryInner) -> T,
    ) -> Result<T, GetNodeError> {
        self.fs.with_inner(|inner| {
            match inner
                .node_at_path(&self.path)
                .ok_or(GetNodeError::DoesNotExist(self.path.clone()))?
            {
                Node::Directory(dir) => Ok(f(dir)),
                other => Err(GetNodeError::WrongKind {
                    expected: NodeKind::Directory,
                    actual: other.kind(),
                    path: self.path.clone(),
                }),
            }
        })
    }
}

impl MockFile {
    pub(crate) fn read_sync(&self) -> Result<Vec<u8>, GetNodeError> {
        self.with_inner(|file| file.contents.to_vec())
    }

    fn with_inner<T>(
        &self,
        f: impl FnOnce(&mut FileInner) -> T,
    ) -> Result<T, GetNodeError> {
        self.fs.with_inner(|inner| {
            match inner
                .node_at_path(&self.path)
                .ok_or(GetNodeError::DoesNotExist(self.path.clone()))?
            {
                Node::File(file) => Ok(f(file)),
                other => Err(GetNodeError::WrongKind {
                    expected: NodeKind::File,
                    actual: other.kind(),
                    path: self.path.clone(),
                }),
            }
        })
    }
}

impl MockSymlink {
    fn clone_priv(&self) -> Self {
        Self {
            fs: self.fs.clone(),
            metadata: self.metadata.clone(),
            path: self.path.clone(),
        }
    }

    fn with_inner<T>(
        &self,
        f: impl FnOnce(&mut SymlinkInner) -> T,
    ) -> Result<T, GetNodeError> {
        self.fs.with_inner(|inner| {
            match inner
                .node_at_path(&self.path)
                .ok_or(GetNodeError::DoesNotExist(self.path.clone()))?
            {
                Node::Symlink(symlink) => Ok(f(symlink)),
                other => Err(GetNodeError::WrongKind {
                    expected: NodeKind::Symlink,
                    actual: other.kind(),
                    path: self.path.clone(),
                }),
            }
        })
    }
}

impl MockNodeId {
    const ROOT: Self = Self(0);

    fn post_inc(&mut self) -> Self {
        let id = self.0;
        self.0 += 1;
        Self(id)
    }
}

impl FsInner {
    fn delete_node(&mut self, path: &AbsPath) -> Result<(), DeleteNodeError> {
        let parent_path = path.parent().ok_or(DeleteNodeError::NodeIsRoot)?;

        let node_name = path.node_name().expect("path is not root");

        let parent = self
            .node_at_path(parent_path)
            .and_then(|node| match node {
                Node::Directory(dir) => Some(dir),
                _ => None,
            })
            .ok_or_else(|| {
                DeleteNodeError::NodeDoesNotExist(path.to_owned())
            })?;

        if !parent.delete_child(node_name) {
            return Err(DeleteNodeError::NodeDoesNotExist(path.to_owned()));
        }

        Ok(())
    }

    fn new(mut root: DirectoryInner) -> Self {
        fn update_metadatas(
            next_node_id: &mut MockNodeId,
            parent: &mut DirectoryInner,
        ) {
            for (name, node) in parent.children.iter_mut() {
                let meta = node.metadata_mut();
                meta.name = name.clone();
                meta.node_id = next_node_id.post_inc();
            }
            for node in parent.children.values_mut() {
                if let Node::Directory(dir) = node {
                    update_metadatas(next_node_id, dir);
                }
            }
        }

        let mut next_node_id = MockNodeId(0);
        update_metadatas(&mut next_node_id, &mut root);
        Self {
            next_node_id,
            root: Node::Directory(root),
            timestamp: MockTimestamp(0),
            watchers: FxHashMap::default(),
        }
    }

    fn node_at_path(&mut self, path: &AbsPath) -> Option<&mut Node> {
        if path.is_root() {
            Some(&mut self.root)
        } else {
            self.root().child_at_path(path)
        }
    }

    fn root(&mut self) -> &mut DirectoryInner {
        match &mut self.root {
            Node::Directory(dir) => dir,
            _ => unreachable!("root is always a directory"),
        }
    }
}

impl Node {
    fn kind(&self) -> NodeKind {
        match self {
            Self::File(_) => NodeKind::File,
            Self::Directory(_) => NodeKind::Directory,
            Self::Symlink(_) => NodeKind::Symlink,
        }
    }

    fn metadata(&self) -> &MockMetadata {
        match self {
            Self::File(file) => &file.metadata,
            Self::Directory(dir) => &dir.metadata,
            Self::Symlink(symlink) => &symlink.metadata,
        }
    }

    fn metadata_mut(&mut self) -> &mut MockMetadata {
        match self {
            Self::File(file) => &mut file.metadata,
            Self::Directory(dir) => &mut dir.metadata,
            Self::Symlink(symlink) => &mut symlink.metadata,
        }
    }
}

impl DirectoryInner {
    /// Should only be used by the `fs!` macro.
    #[doc(hidden)]
    #[track_caller]
    pub fn insert_child(
        &mut self,
        name: impl AsRef<NodeName>,
        child: impl Into<Node>,
    ) -> &mut Self {
        let name = name.as_ref();
        match self.children.entry(name.to_owned()) {
            indexmap::map::Entry::Occupied(_) => {
                panic!("duplicate child name: {name:?}");
            },
            indexmap::map::Entry::Vacant(entry) => {
                entry.insert(child.into());
            },
        }
        self
    }

    /// Should only be used by the `fs!` macro.
    #[doc(hidden)]
    pub fn new() -> Self {
        Self {
            children: Default::default(),
            metadata: MockMetadata {
                byte_len: 0usize.into(),
                created_at: MockTimestamp(0),
                last_modified_at: MockTimestamp(0),
                name: "temp".parse().expect("it's valid"),
                node_id: MockNodeId(0),
                node_kind: NodeKind::Directory,
            },
        }
    }

    fn child_at_path(&mut self, path: &AbsPath) -> Option<&mut Node> {
        let mut components = path.components();
        let node = self.children.get_mut(components.next()?)?;
        if components.as_path().is_root() {
            return Some(node);
        }
        let Node::Directory(dir) = node else { return None };
        dir.child_at_path(components.as_path())
    }

    fn clear(&mut self) {
        self.children.clear();
    }

    fn create_node(
        &mut self,
        this_path: &AbsPath,
        name: &NodeName,
        node: Node,
    ) -> Result<(), CreateNodeError> {
        if self.children.contains_key(name) {
            Err(CreateNodeError::AlreadyExists(NodeAlreadyExistsError {
                kind: node.kind(),
                path: this_path.join(name),
            }))
        } else {
            self.children.insert(name.to_owned(), node);
            Ok(())
        }
    }

    fn delete_child(&mut self, name: &NodeName) -> bool {
        self.children.swap_remove(name).is_some()
    }
}

impl FileInner {
    /// Should only be used by the `fs!` macro.
    #[doc(hidden)]
    pub fn new<C: AsRef<[u8]>>(contents: C) -> Self {
        let contents = contents.as_ref();
        Self {
            contents: contents.to_owned(),
            metadata: MockMetadata {
                byte_len: contents.len().into(),
                created_at: MockTimestamp(0),
                last_modified_at: MockTimestamp(0),
                name: "temp".parse().expect("it's valid"),
                node_id: MockNodeId(0),
                node_kind: NodeKind::File,
            },
        }
    }

    fn write<C: AsRef<[u8]>>(&mut self, contents: C) {
        self.contents = contents.as_ref().to_owned();
    }
}

impl SymlinkInner {
    /// Should only be used by the `fs!` macro.
    #[doc(hidden)]
    pub fn new<P: AsRef<str>>(target_path: P) -> Self {
        let target_path = target_path.as_ref();
        Self {
            target_path: target_path.to_owned(),
            metadata: MockMetadata {
                byte_len: target_path.len().into(),
                created_at: MockTimestamp(0),
                last_modified_at: MockTimestamp(0),
                name: "temp".parse().expect("it's valid"),
                node_id: MockNodeId(0),
                node_kind: NodeKind::Symlink,
            },
        }
    }
}

impl WatchChannel {
    const CAPACITY: usize = 16;

    fn emit(&self, event: FsEvent<MockTimestamp>) {
        if self.tx.receiver_count() > 0 {
            self.tx
                .broadcast_blocking(event)
                .expect("there's at least one active receiver");
        }
    }

    fn new() -> Self {
        let (tx, rx) = async_broadcast::broadcast(Self::CAPACITY);
        Self { tx, inactive_rx: rx.deactivate() }
    }

    fn rx(&self) -> async_broadcast::Receiver<FsEvent<MockTimestamp>> {
        self.inactive_rx.activate_cloned()
    }
}

impl fs::Fs for MockFs {
    type Directory = MockDirectory;
    type File = MockFile;
    type Symlink = MockSymlink;
    type Metadata = MockMetadata;
    type NodeId = MockNodeId;
    type Timestamp = MockTimestamp;

    type CreateDirectoriesError = CreateNodeError;
    type NodeAtPathError = Infallible;

    async fn create_all_missing_directories<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Self::Directory, Self::CreateDirectoriesError> {
        let mut existing_path = path.as_ref();

        let mut dir = loop {
            let Ok(maybe_node) = self.node_at_path(existing_path).await;

            let Some(node) = maybe_node else {
                existing_path = existing_path.parent().expect("not root");
                continue;
            };

            match node {
                fs::FsNode::Directory(dir) => break dir,
                other => {
                    return Err(CreateNodeError::AlreadyExists(
                        NodeAlreadyExistsError {
                            kind: other.kind(),
                            path: existing_path.to_owned(),
                        },
                    ));
                },
            }
        };

        let Some(mut missing_components) = path
            .as_ref()
            .strip_prefix(existing_path)
            .map(|path| path.components())
        else {
            return Ok(dir);
        };

        loop {
            match missing_components.next() {
                Some(dir_name) => dir = dir.create_directory(dir_name).await?,
                None => return Ok(dir),
            }
        }
    }

    async fn node_at_path<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Option<fs::FsNode<Self>>, Self::NodeAtPathError> {
        let path = path.as_ref();
        Ok(self.with_inner(|inner| {
            inner.node_at_path(path).map(|node| {
                let metadata = node.metadata().clone();
                match metadata.node_kind {
                    NodeKind::File => fs::FsNode::File(MockFile {
                        fs: self.clone(),
                        metadata,
                        path: path.to_owned(),
                    }),
                    NodeKind::Directory => {
                        fs::FsNode::Directory(MockDirectory {
                            fs: self.clone(),
                            metadata,
                            path: path.to_owned(),
                        })
                    },
                    NodeKind::Symlink => fs::FsNode::Symlink(MockSymlink {
                        fs: self.clone(),
                        metadata,
                        path: path.to_owned(),
                    }),
                }
            })
        }))
    }

    fn now(&self) -> Self::Timestamp {
        self.with_inner(|inner| inner.timestamp)
    }
}

impl fs::Directory for MockDirectory {
    type EventStream = futures_lite::stream::Pending<DirectoryEvent<MockFs>>;
    type Fs = MockFs;

    type CreateDirectoryError = CreateNodeError;
    type CreateFileError = CreateNodeError;
    type CreateSymlinkError = CreateNodeError;
    type ClearError = GetNodeError;
    type DeleteError = DeleteNodeError;
    type ParentError = GetNodeError;
    type ReadEntryError = ReadDirNextError;
    type ReadError = ReadDirError;

    async fn create_directory(
        &self,
        dir_name: &NodeName,
    ) -> Result<Self, Self::CreateDirectoryError> {
        let metadata =
            self.create_node(dir_name, NodeKind::Directory, None)?;

        Ok(Self {
            fs: self.fs.clone(),
            metadata,
            path: self.path.clone().join(dir_name),
        })
    }

    async fn create_file(
        &self,
        file_name: &NodeName,
    ) -> Result<MockFile, Self::CreateFileError> {
        let metadata = self.create_node(file_name, NodeKind::File, None)?;

        Ok(MockFile {
            fs: self.fs.clone(),
            metadata,
            path: self.path.clone().join(file_name),
        })
    }

    async fn create_symlink(
        &self,
        symlink_name: &NodeName,
        target_path: &str,
    ) -> Result<MockSymlink, Self::CreateSymlinkError> {
        let metadata = self.create_node(
            symlink_name,
            NodeKind::Symlink,
            Some(target_path),
        )?;

        Ok(MockSymlink {
            fs: self.fs.clone(),
            metadata,
            path: self.path.clone().join(symlink_name),
        })
    }

    async fn clear(&self) -> Result<(), Self::ClearError> {
        self.with_inner(|dir| dir.clear())
    }

    async fn delete(self) -> Result<(), Self::DeleteError> {
        self.fs.delete_node(&self.path)
    }

    fn meta(&self) -> MockMetadata {
        self.metadata.clone()
    }

    async fn read(&self) -> Result<ReadDir, Self::ReadError> {
        let fs::FsNode::Directory(dir_handle) = self
            .fs
            .node_at_path(&*self.path)
            .await?
            .ok_or(ReadDirError::NoNodeAtPath)?
        else {
            return Err(ReadDirError::NoDirAtPath);
        };
        Ok(ReadDir { dir_handle, next_child_idx: 0 })
    }

    async fn parent(&self) -> Result<Option<Self>, Self::ParentError> {
        let Some(parent_path) = self.path.parent() else { return Ok(None) };
        let Ok(maybe_node) = self.fs.node_at_path(parent_path).await;
        match maybe_node.expect("parent must exist") {
            fs::FsNode::Directory(parent) => Ok(Some(parent)),
            other => Err(GetNodeError::WrongKind {
                expected: NodeKind::Directory,
                actual: other.kind(),
                path: parent_path.to_owned(),
            }),
        }
    }

    fn path(&self) -> &AbsPath {
        &self.path
    }

    fn watch(&self) -> Self::EventStream {
        todo!()
    }
}

impl fs::File for MockFile {
    type EventStream = futures_lite::stream::Pending<FileEvent<MockFs>>;
    type Fs = MockFs;

    type DeleteError = DeleteNodeError;
    type ParentError = GetNodeError;
    type ReadError = GetNodeError;
    type WriteError = GetNodeError;

    async fn delete(self) -> Result<(), Self::DeleteError> {
        self.fs.delete_node(&self.path)
    }

    fn meta(&self) -> MockMetadata {
        self.metadata.clone()
    }

    async fn parent(&self) -> Result<MockDirectory, Self::ParentError> {
        let parent_path = self.path.parent().expect("can't be root");
        let Ok(maybe_node) = self.fs.node_at_path(parent_path).await;
        match maybe_node.expect("parent must exist") {
            fs::FsNode::Directory(parent) => Ok(parent),
            other => Err(GetNodeError::WrongKind {
                expected: NodeKind::Directory,
                actual: other.kind(),
                path: parent_path.to_owned(),
            }),
        }
    }

    fn path(&self) -> &AbsPath {
        &self.path
    }

    async fn read(&self) -> Result<Vec<u8>, Self::ReadError> {
        self.with_inner(|file| file.contents.clone())
    }

    fn watch(&self) -> Self::EventStream {
        todo!()
    }

    async fn write<C: AsRef<[u8]>>(
        &mut self,
        new_contents: C,
    ) -> Result<(), Self::WriteError> {
        self.with_inner(|file| file.write(new_contents.as_ref()))
    }
}

impl fs::Symlink for MockSymlink {
    type EventStream = futures_lite::stream::Pending<SymlinkEvent<MockFs>>;
    type Fs = MockFs;

    type DeleteError = DeleteNodeError;
    type FollowError = FollowError;
    type ParentError = GetNodeError;
    type ReadError = GetNodeError;

    async fn delete(self) -> Result<(), Self::DeleteError> {
        self.fs.delete_node(&self.path)
    }

    async fn follow(
        &self,
    ) -> Result<Option<fs::FsNode<MockFs>>, Self::FollowError> {
        use std::path::MAIN_SEPARATOR;

        let target_path = self.read_path().await?;

        let mut stack = self.path.components().collect::<Vec<_>>();
        let target_components = target_path.split(MAIN_SEPARATOR);
        for component in target_components {
            if component == "." {
                continue;
            } else if component == ".." {
                if stack.pop().is_none() {
                    return Err(FollowError::InvalidTargetPath {
                        symlink_path: self.path.clone(),
                        target_path: target_path.clone(),
                    });
                }
            } else {
                let component =
                    <&NodeName>::try_from(component).map_err(|_| {
                        FollowError::InvalidTargetPath {
                            symlink_path: self.path.clone(),
                            target_path: target_path.clone(),
                        }
                    })?;
                stack.push(component);
            }
        }

        match self
            .fs
            .node_at_path(stack.into_iter().collect::<AbsPathBuf>())
            .await
        {
            Ok(maybe_node) => Ok(maybe_node),
        }
    }

    async fn follow_recursively(
        &self,
    ) -> Result<Option<fs::FsNode<MockFs>>, Self::FollowError> {
        let mut symlink = self.clone_priv();
        loop {
            let Some(node) = symlink.follow().await? else { return Ok(None) };
            match node {
                fs::FsNode::Symlink(new_symlink) => symlink = new_symlink,
                other => return Ok(Some(other)),
            }
        }
    }

    fn meta(&self) -> MockMetadata {
        self.metadata.clone()
    }

    async fn parent(&self) -> Result<MockDirectory, Self::ParentError> {
        let parent_path = self.path.parent().expect("can't be root");
        let Ok(maybe_node) = self.fs.node_at_path(parent_path).await;
        match maybe_node.expect("parent must exist") {
            fs::FsNode::Directory(parent) => Ok(parent),
            other => Err(GetNodeError::WrongKind {
                expected: NodeKind::Directory,
                actual: other.kind(),
                path: parent_path.to_owned(),
            }),
        }
    }

    fn path(&self) -> &AbsPath {
        &self.path
    }

    async fn read_path(&self) -> Result<String, Self::ReadError> {
        self.with_inner(|symlink| symlink.target_path.clone())
    }

    fn watch(&self) -> Self::EventStream {
        todo!()
    }
}

impl fs::Metadata for MockMetadata {
    type Fs = MockFs;

    fn byte_len(&self) -> ByteOffset {
        self.byte_len
    }

    fn created_at(&self) -> Option<MockTimestamp> {
        Some(self.created_at)
    }

    fn id(&self) -> MockNodeId {
        self.node_id
    }

    fn last_modified_at(&self) -> Option<MockTimestamp> {
        Some(self.last_modified_at)
    }

    fn name(&self) -> Result<&NodeName, fs::MetadataNameError> {
        if self.node_id == MockNodeId::ROOT {
            Err(fs::MetadataNameError::MetadataIsForRoot)
        } else {
            Ok(&self.name)
        }
    }

    fn node_kind(&self) -> NodeKind {
        self.node_kind
    }
}

impl Stream for ReadDir {
    type Item = Result<MockMetadata, ReadDirNextError>;

    fn poll_next(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let metadata = match this.dir_handle.fs.with_inner(|inner| {
            Ok(inner
                .node_at_path(&this.dir_handle.path)
                .and_then(|node| match node {
                    Node::Directory(dir) => Some(dir),
                    _ => None,
                })
                .ok_or(ReadDirNextError::DirWasDeleted)?
                .children
                .get_index(*this.next_child_idx)
                .map(|(_name, node)| node.metadata().clone()))
        }) {
            Ok(Some(meta)) => meta,
            Ok(None) => return Poll::Ready(None),
            Err(err) => return Poll::Ready(Some(Err(err))),
        };
        *this.next_child_idx += 1;
        Poll::Ready(Some(Ok(metadata)))
    }
}

impl Stream for Watcher {
    type Item = Result<FsEvent<MockTimestamp>, Infallible>;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project()
            .inner
            .poll_next(ctx)
            .map(|maybe_item| maybe_item.map(Ok))
    }
}

impl fmt::Debug for MockDirectory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.with_inner(|dir| fmt::Debug::fmt(dir, f)) {
            Ok(res) => res,
            Err(err) => fmt::Debug::fmt(&err, f),
        }
    }
}

impl PartialEq for MockDirectory {
    fn eq(&self, other: &Self) -> bool {
        self.with_inner(|l| other.with_inner(|r| l == r).unwrap_or(false))
            .unwrap_or(false)
    }
}

impl fmt::Debug for MockFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.with_inner(|dir| fmt::Debug::fmt(dir, f)) {
            Ok(res) => res,
            Err(err) => fmt::Debug::fmt(&err, f),
        }
    }
}

impl PartialEq for MockFile {
    fn eq(&self, other: &Self) -> bool {
        self.with_inner(|l| other.with_inner(|r| l == r).unwrap_or(false))
            .unwrap_or(false)
    }
}

impl fmt::Debug for MockSymlink {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.with_inner(|dir| fmt::Debug::fmt(dir, f)) {
            Ok(res) => res,
            Err(err) => fmt::Debug::fmt(&err, f),
        }
    }
}

impl PartialEq for MockSymlink {
    fn eq(&self, other: &Self) -> bool {
        self.with_inner(|l| other.with_inner(|r| l == r).unwrap_or(false))
            .unwrap_or(false)
    }
}

impl Default for FsInner {
    fn default() -> Self {
        Self::new(DirectoryInner::new())
    }
}

#[derive(
    Debug, derive_more::Display, cauchy::Error, cauchy::From, PartialEq, Eq,
)]
#[display("{_0}")]
pub enum CreateNodeError {
    AlreadyExists(#[from] NodeAlreadyExistsError),
    GetParent(#[from] GetNodeError),
}

#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
pub enum DeleteNodeError {
    #[display("cannot delete root")]
    NodeIsRoot,
    #[display("no node at {_0:?}")]
    NodeDoesNotExist(AbsPathBuf),
}

#[derive(
    Debug, derive_more::Display, cauchy::Error, cauchy::From, PartialEq, Eq,
)]
pub enum FollowError {
    #[display(
        "symlink at {symlink_path:?} has an invalid target path: \
         {target_path:?}"
    )]
    InvalidTargetPath { symlink_path: AbsPathBuf, target_path: String },

    #[display("{_0:?}")]
    GetSymlink(#[from] GetNodeError),
}

#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
pub enum GetNodeError {
    #[display("no node at {_0:?}")]
    DoesNotExist(AbsPathBuf),
    #[display("expected a {expected:?}, got a {actual:?} at {path:?}")]
    WrongKind { expected: NodeKind, actual: NodeKind, path: AbsPathBuf },
}

#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
pub enum ReadDirError {
    #[display("no node at path")]
    NoNodeAtPath,
    #[display("no directory at path")]
    NoDirAtPath,
}

#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
pub enum ReadDirNextError {
    #[display("directory has been deleted")]
    DirWasDeleted,
}

#[derive(Debug, derive_more::Display, cauchy::Error, PartialEq, Eq)]
#[display("a {kind:?} already exists at {path:?}")]
pub struct NodeAlreadyExistsError {
    kind: NodeKind,
    path: AbsPathBuf,
}

impl From<Infallible> for ReadDirError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
