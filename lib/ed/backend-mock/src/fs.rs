use core::convert::Infallible;
use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::sync::{Arc, Mutex};

use ed_core::ByteOffset;
use ed_core::fs::{
    self,
    AbsPath,
    AbsPathBuf,
    DirectoryEvent,
    Fs,
    FsEvent,
    FsEventKind,
    FsNodeKind,
    NodeName,
    NodeNameBuf,
};
use futures_lite::Stream;
use fxhash::FxHashMap;
use indexmap::IndexMap;

/// TODO: docs.
#[derive(Clone, Default)]
pub struct MockFs {
    inner: Arc<Mutex<FsInner>>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(u64);

#[derive(Debug, PartialEq)]
pub enum MockFsNode {
    File(MockFile),
    Directory(MockDirectory),
}

#[derive(Debug, Default)]
pub struct MockDirectory {
    children: IndexMap<NodeNameBuf, MockFsNode>,
}

#[derive(Debug)]
pub struct MockFile {
    contents: Vec<u8>,
}

pub enum DirEntry {
    Directory(DirectoryHandle),
    File(FileHandle),
}

pub struct DirectoryHandle {
    fs: MockFs,
    path: AbsPathBuf,
}

pub struct FileHandle {
    fs: MockFs,
    path: AbsPathBuf,
}

pub struct SymlinkHandle {
    fs: MockFs,
    path: AbsPathBuf,
}

pin_project_lite::pin_project! {
    pub struct ReadDir {
        dir_handle: DirectoryHandle,
        next_child_idx: usize,
    }
}

pin_project_lite::pin_project! {
    pub struct Watcher {
        fs: MockFs,
        path: AbsPathBuf,
        #[pin]
        inner: async_broadcast::Receiver<FsEvent<Timestamp>>,
    }

    impl PinnedDrop for Watcher {
        fn drop(this: Pin<&mut Self>) {
            this.fs.with_inner(|inner| inner.watchers.remove(&this.path));
        }
    }
}

struct FsInner {
    root: MockFsNode,
    timestamp: Timestamp,
    watchers: FxHashMap<AbsPathBuf, WatchChannel>,
}

struct WatchChannel {
    inactive_rx: async_broadcast::InactiveReceiver<FsEvent<Timestamp>>,
    tx: async_broadcast::Sender<FsEvent<Timestamp>>,
}

impl MockFs {
    pub fn new(root: MockDirectory) -> Self {
        Self { inner: Arc::new(Mutex::new(FsInner::new(root))) }
    }

    pub(crate) fn node_at_path_sync(
        &self,
        path: &AbsPath,
    ) -> Option<fs::FsNode<Self>> {
        let kind = self.with_inner(|inner| {
            inner.node_at_path(path).as_deref().map(MockFsNode::kind)
        })?;
        let node = match kind {
            FsNodeKind::File => fs::FsNode::File(FileHandle {
                fs: self.clone(),
                path: path.to_owned(),
            }),
            FsNodeKind::Directory => fs::FsNode::Directory(DirectoryHandle {
                fs: self.clone(),
                path: path.to_owned(),
            }),
            FsNodeKind::Symlink => unreachable!(),
        };
        Some(node)
    }

    fn delete_node(&self, path: &AbsPath) -> Result<(), DeleteNodeError> {
        self.with_inner(|inner| inner.delete_node(path))
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

impl DirEntry {
    fn exists(&self) -> bool {
        match self {
            Self::Directory(dir_handle) => dir_handle.exists(),
            Self::File(file_handle) => file_handle.exists(),
        }
    }

    fn kind(&self) -> FsNodeKind {
        match self {
            Self::Directory(_) => FsNodeKind::Directory,
            Self::File(_) => FsNodeKind::File,
        }
    }

    fn path(&self) -> &AbsPath {
        match self {
            Self::Directory(handle) => &handle.path,
            Self::File(handle) => &handle.path,
        }
    }
}

impl DirectoryHandle {
    fn exists(&self) -> bool {
        self.fs.with_inner(|inner| {
            matches!(
                inner.node_at_path(&self.path),
                Some(MockFsNode::Directory(_))
            )
        })
    }

    fn with_dir<T>(
        &self,
        f: impl FnOnce(&mut MockDirectory) -> T,
    ) -> Result<T, DirEntryDoesNotExistError> {
        self.fs.with_inner(|inner| match inner.dir_at_path(&self.path) {
            Some(dir) => Ok(f(dir)),
            None => Err(DirEntryDoesNotExistError),
        })
    }
}

impl FileHandle {
    pub(crate) fn read_sync(
        &self,
    ) -> Result<Vec<u8>, DirEntryDoesNotExistError> {
        self.with_file(|file| file.contents().to_vec())
    }

    fn exists(&self) -> bool {
        self.with_file(|_| true).unwrap_or(false)
    }

    fn with_file<T>(
        &self,
        f: impl FnOnce(&mut MockFile) -> T,
    ) -> Result<T, DirEntryDoesNotExistError> {
        self.fs.with_inner(|inner| match inner.file_at_path(&self.path) {
            Some(file) => Ok(f(file)),
            None => Err(DirEntryDoesNotExistError),
        })
    }
}

impl FsInner {
    fn create_node(
        &mut self,
        path: &AbsPath,
        node: MockFsNode,
    ) -> Result<(), CreateNodeError> {
        let (parent_path, node_name) = match (path.parent(), path.node_name())
        {
            (Some(parent), Some(name)) => (parent, name),
            _ => {
                return Err(CreateNodeError::AlreadyExists(
                    NodeAlreadyExistsError {
                        kind: FsNodeKind::File,
                        path: path.to_owned(),
                    },
                ));
            },
        };

        let parent = self.node_at_path(parent_path).ok_or_else(|| {
            CreateNodeError::ParentDoesNotExist(parent_path.to_owned())
        })?;

        let node_kind = match parent {
            MockFsNode::Directory(parent) => {
                if let Some(child) = parent.children.get(node_name) {
                    return Err(CreateNodeError::AlreadyExists(
                        NodeAlreadyExistsError {
                            kind: child.kind(),
                            path: path.to_owned(),
                        },
                    ));
                }
                let kind = node.kind();
                parent.children.insert(node_name.to_owned(), node);
                kind
            },
            MockFsNode::File(_) => {
                return Err(CreateNodeError::ParentIsFile(
                    parent_path.to_owned(),
                ));
            },
        };

        let event = FsEvent {
            kind: match node_kind {
                FsNodeKind::File => FsEventKind::CreatedFile,
                FsNodeKind::Directory => FsEventKind::CreatedDir,
                FsNodeKind::Symlink => unreachable!(),
            },
            path: path.to_owned(),
            timestamp: self.timestamp,
        };

        for (watch_root, watcher) in &self.watchers {
            if event.path.starts_with(watch_root) {
                watcher.emit(event.clone());
            }
        }

        Ok(())
    }

    fn delete_node(&mut self, path: &AbsPath) -> Result<(), DeleteNodeError> {
        let parent_path = path.parent().ok_or(DeleteNodeError::NodeIsRoot)?;

        let node_name = path.node_name().expect("path is not root");

        let parent = self.dir_at_path(parent_path).ok_or_else(|| {
            DeleteNodeError::NodeDoesNotExist(path.to_owned())
        })?;

        if !parent.delete_child(node_name) {
            return Err(DeleteNodeError::NodeDoesNotExist(path.to_owned()));
        }

        Ok(())
    }

    fn dir_at_path(&mut self, path: &AbsPath) -> Option<&mut MockDirectory> {
        if path.is_root() {
            Some(self.root())
        } else {
            self.root().dir_at_path(path)
        }
    }

    fn file_at_path(&mut self, path: &AbsPath) -> Option<&mut MockFile> {
        self.root().file_at_path(path)
    }

    fn new(root: MockDirectory) -> Self {
        Self {
            root: MockFsNode::Directory(root),
            timestamp: Timestamp(0),
            watchers: FxHashMap::default(),
        }
    }

    fn node_at_path(&mut self, path: &AbsPath) -> Option<&mut MockFsNode> {
        if path.is_root() {
            Some(&mut self.root)
        } else {
            self.root().child_at_path(path)
        }
    }

    fn root(&mut self) -> &mut MockDirectory {
        match &mut self.root {
            MockFsNode::Directory(dir) => dir,
            _ => unreachable!("root is always a directory"),
        }
    }
}

impl MockFsNode {
    fn kind(&self) -> FsNodeKind {
        match self {
            Self::File(_) => FsNodeKind::File,
            Self::Directory(_) => FsNodeKind::Directory,
        }
    }
}

impl MockDirectory {
    // Should only be used by the `fs!` macro.
    #[doc(hidden)]
    #[track_caller]
    pub fn insert_child(
        &mut self,
        name: impl AsRef<NodeName>,
        child: impl Into<MockFsNode>,
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

    pub fn new() -> Self {
        Self::default()
    }

    fn child_at_path(&mut self, path: &AbsPath) -> Option<&mut MockFsNode> {
        let mut components = path.components();
        let node = self.children.get_mut(components.next()?)?;
        if components.as_path().is_root() {
            return Some(node);
        }
        let MockFsNode::Directory(dir) = node else { return None };
        dir.child_at_path(components.as_path())
    }

    fn clear(&mut self) {
        self.children.clear();
    }

    fn delete_child(&mut self, name: &NodeName) -> bool {
        self.children.swap_remove(name).is_some()
    }

    fn dir_at_path(&mut self, path: &AbsPath) -> Option<&mut Self> {
        match self.child_at_path(path)? {
            MockFsNode::Directory(dir) => Some(dir),
            _ => None,
        }
    }

    fn file_at_path(&mut self, path: &AbsPath) -> Option<&mut MockFile> {
        match self.child_at_path(path)? {
            MockFsNode::File(file) => Some(file),
            _ => None,
        }
    }
}

impl MockFile {
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    pub fn len(&self) -> ByteOffset {
        self.contents().len().into()
    }

    pub fn new<C: AsRef<[u8]>>(contents: C) -> Self {
        Self { contents: contents.as_ref().to_owned() }
    }

    pub fn write<C: AsRef<[u8]>>(&mut self, contents: C) {
        self.contents = contents.as_ref().to_owned();
    }
}

impl WatchChannel {
    const CAPACITY: usize = 16;

    fn emit(&self, event: FsEvent<Timestamp>) {
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

    fn rx(&self) -> async_broadcast::Receiver<FsEvent<Timestamp>> {
        self.inactive_rx.activate_cloned()
    }
}

impl fs::Fs for MockFs {
    type Directory = DirectoryHandle;
    type File = FileHandle;
    type Symlink = SymlinkHandle;
    type Timestamp = Timestamp;
    type Watcher = Watcher;

    type CreateDirectoryError = CreateNodeError;
    type CreateFileError = CreateNodeError;
    type NodeAtPathError = Infallible;
    type WatchError = Infallible;

    async fn create_directory<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Self::Directory, Self::CreateDirectoryError> {
        let path = path.as_ref();
        self.with_inner(|fs| {
            fs.create_node(path, MockFsNode::Directory(MockDirectory::new()))
        })?;
        Ok(DirectoryHandle { fs: self.clone(), path: path.to_owned() })
    }

    async fn create_file<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Self::File, Self::CreateFileError> {
        let path = path.as_ref();
        self.with_inner(|fs| {
            fs.create_node(path, MockFsNode::File(MockFile::new("")))
        })?;
        Ok(FileHandle { fs: self.clone(), path: path.to_owned() })
    }

    async fn node_at_path<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Option<fs::FsNode<Self>>, Self::NodeAtPathError> {
        Ok(self.node_at_path_sync(path.as_ref()))
    }

    fn now(&self) -> Self::Timestamp {
        self.with_inner(|inner| inner.timestamp)
    }

    async fn watch<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Self::Watcher, Self::WatchError> {
        let path = path.as_ref().to_owned();
        let rx = self.with_inner(|inner| {
            inner
                .watchers
                .entry(path.clone())
                .or_insert_with(WatchChannel::new)
                .rx()
        });
        Ok(Watcher { inner: rx, fs: self.clone(), path })
    }
}

impl From<MockDirectory> for MockFsNode {
    fn from(dir: MockDirectory) -> Self {
        Self::Directory(dir)
    }
}

impl From<MockFile> for MockFsNode {
    fn from(file: MockFile) -> Self {
        Self::File(file)
    }
}

impl PartialEq for MockFile {
    fn eq(&self, other: &Self) -> bool {
        self.contents == other.contents
    }
}

impl PartialEq for MockDirectory {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
    }
}

impl fs::Metadata for DirEntry {
    type Timestamp = Timestamp;
    type NameError = DirEntryDoesNotExistError;
    type NodeKindError = DirEntryDoesNotExistError;

    fn created_at(&self) -> Option<Timestamp> {
        None
    }

    fn last_modified_at(&self) -> Option<Timestamp> {
        None
    }

    #[track_caller]
    fn byte_len(&self) -> ByteOffset {
        match self {
            DirEntry::Directory(_) => 0usize.into(),
            DirEntry::File(file) => file
                .with_file(|file| file.len())
                .expect("file has been deleted"),
        }
    }

    async fn name(&self) -> Result<NodeNameBuf, Self::NameError> {
        self.exists()
            .then(|| self.path().node_name().expect("path is not root"))
            .map(ToOwned::to_owned)
            .ok_or(DirEntryDoesNotExistError)
    }

    async fn node_kind(&self) -> Result<FsNodeKind, Self::NodeKindError> {
        self.exists().then_some(self.kind()).ok_or(DirEntryDoesNotExistError)
    }
}

impl Stream for ReadDir {
    type Item = Result<DirEntry, ReadDirNextError>;

    fn poll_next(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let (name, kind) = match this.dir_handle.fs.with_inner(|inner| {
            Ok(inner
                .dir_at_path(&this.dir_handle.path)
                .ok_or(ReadDirNextError::DirWasDeleted)?
                .children
                .get_index(*this.next_child_idx)
                .map(|(name, node)| (name.to_owned(), node.kind())))
        }) {
            Ok(Some(tuple)) => tuple,
            Ok(None) => return Poll::Ready(None),
            Err(err) => return Poll::Ready(Some(Err(err))),
        };
        *this.next_child_idx += 1;
        let mut child_path = this.dir_handle.path.clone();
        child_path.push(name);
        let entry = match kind {
            FsNodeKind::File => DirEntry::File(FileHandle {
                fs: this.dir_handle.fs.clone(),
                path: child_path,
            }),
            FsNodeKind::Directory => DirEntry::Directory(DirectoryHandle {
                fs: this.dir_handle.fs.clone(),
                path: child_path,
            }),
            FsNodeKind::Symlink => unreachable!(),
        };
        Poll::Ready(Some(Ok(entry)))
    }
}

impl Stream for Watcher {
    type Item = Result<FsEvent<Timestamp>, Infallible>;

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

impl fmt::Debug for DirectoryHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.with_dir(|dir| fmt::Debug::fmt(dir, f)) {
            Ok(res) => res,
            Err(err) => fmt::Debug::fmt(&err, f),
        }
    }
}

impl PartialEq for DirectoryHandle {
    fn eq(&self, other: &Self) -> bool {
        self.with_dir(|l| other.with_dir(|r| l == r).unwrap_or(false))
            .unwrap_or(false)
    }
}

impl fs::Directory for DirectoryHandle {
    type EventStream = futures_lite::stream::Pending<DirectoryEvent<Self>>;
    type Fs = MockFs;
    type Metadata = DirEntry;

    type CreateDirectoryError = CreateNodeError;
    type CreateFileError = CreateNodeError;
    type ClearError = DirEntryDoesNotExistError;
    type DeleteError = DeleteNodeError;
    type ReadEntryError = ReadDirNextError;
    type ReadError = ReadDirError;

    async fn create_directory(
        &self,
        directory_name: &NodeName,
    ) -> Result<Self, Self::CreateDirectoryError> {
        self.fs.create_directory(self.path.clone().join(directory_name)).await
    }

    async fn create_file(
        &self,
        file_name: &NodeName,
    ) -> Result<FileHandle, Self::CreateFileError> {
        self.fs.create_file(self.path.clone().join(file_name)).await
    }

    async fn clear(&self) -> Result<(), Self::ClearError> {
        self.with_dir(|dir| dir.clear())
    }

    async fn delete(self) -> Result<(), Self::DeleteError> {
        self.fs.delete_node(&self.path)
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

    async fn parent(&self) -> Option<Self> {
        self.path
            .parent()
            .map(|path| Self { path: path.to_owned(), fs: self.fs.clone() })
    }

    fn path(&self) -> &AbsPath {
        &self.path
    }

    async fn watch(&self) -> Self::EventStream {
        todo!()
    }
}

impl fmt::Debug for FileHandle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.with_file(|dir| fmt::Debug::fmt(dir, f)) {
            Ok(res) => res,
            Err(err) => fmt::Debug::fmt(&err, f),
        }
    }
}

impl PartialEq for FileHandle {
    fn eq(&self, other: &Self) -> bool {
        self.with_file(|l| other.with_file(|r| l == r).unwrap_or(false))
            .unwrap_or(false)
    }
}

impl fs::File for FileHandle {
    type Fs = MockFs;

    type DeleteError = DeleteNodeError;
    type Error = DirEntryDoesNotExistError;
    type WriteError = DirEntryDoesNotExistError;

    async fn byte_len(&self) -> Result<ByteOffset, Self::Error> {
        self.with_file(|file| file.len())
    }

    async fn delete(self) -> Result<(), Self::DeleteError> {
        self.fs.delete_node(&self.path)
    }

    async fn parent(&self) -> <Self::Fs as fs::Fs>::Directory {
        DirectoryHandle {
            fs: self.fs.clone(),
            path: self.path.parent().expect("has a parent").to_owned(),
        }
    }

    fn path(&self) -> &AbsPath {
        &self.path
    }

    async fn write<C: AsRef<[u8]>>(
        &mut self,
        new_contents: C,
    ) -> Result<(), Self::WriteError> {
        self.with_file(|file| file.write(new_contents.as_ref()))
    }
}

impl fmt::Debug for SymlinkHandle {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        unreachable!()
    }
}

impl PartialEq for SymlinkHandle {
    fn eq(&self, _: &Self) -> bool {
        unreachable!()
    }
}

impl fs::Symlink for SymlinkHandle {
    type Fs = MockFs;

    type DeleteError = DeleteNodeError;
    type FollowError = Infallible;

    async fn delete(self) -> Result<(), Self::DeleteError> {
        self.fs.delete_node(&self.path)
    }

    async fn follow(
        &self,
    ) -> Result<Option<fs::FsNode<MockFs>>, Self::FollowError> {
        unreachable!()
    }

    async fn follow_recursively(
        &self,
    ) -> Result<Option<fs::FsNode<MockFs>>, Self::FollowError> {
        unreachable!()
    }

    fn path(&self) -> &AbsPath {
        &self.path
    }
}

impl Default for FsInner {
    fn default() -> Self {
        Self::new(MockDirectory::default())
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("dir entry does not exist")]
pub struct DirEntryDoesNotExistError;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum DeleteNodeError {
    #[error("cannot delete root")]
    NodeIsRoot,

    #[error("no node at {:?}", .0)]
    NodeDoesNotExist(AbsPathBuf),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReadDirError {
    #[error("no node at path")]
    NoNodeAtPath,
    #[error("no directory at path")]
    NoDirAtPath,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReadDirNextError {
    #[error("directory has been deleted")]
    DirWasDeleted,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum CreateNodeError {
    #[error(transparent)]
    AlreadyExists(NodeAlreadyExistsError),

    #[error("parent directory does not exist: {:?}", .0)]
    ParentDoesNotExist(AbsPathBuf),

    #[error("node at {:?} is a file, not a directory", .0)]
    ParentIsFile(AbsPathBuf),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("a {:?} already exists at {:?}", .kind, .path)]
pub struct NodeAlreadyExistsError {
    kind: FsNodeKind,
    path: AbsPathBuf,
}

impl From<Infallible> for ReadDirError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
