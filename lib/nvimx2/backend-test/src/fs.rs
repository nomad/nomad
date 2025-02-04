use core::convert::Infallible;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::borrow::Cow;
use std::fs::Metadata;
use std::sync::{Arc, Mutex};

use futures_lite::Stream;
use fxhash::FxHashMap;
use indexmap::IndexMap;
use nvimx_core::fs::{
    AbsPath,
    AbsPathBuf,
    DirEntry,
    Fs,
    FsEvent,
    FsNode,
    FsNodeKind,
    FsNodeName,
    FsNodeNameBuf,
};

/// TODO: docs.
#[derive(Clone)]
pub struct TestFs {
    inner: Arc<Mutex<TestFsInner>>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TestTimestamp(u64);

#[derive(PartialEq)]
pub enum TestFsNode {
    File(TestFile),
    Directory(TestDirectory),
}

#[derive(Default)]
pub struct TestDirectory {
    children: IndexMap<FsNodeNameBuf, TestFsNode>,
}

pub struct TestFile {
    contents: Vec<u8>,
}

pub enum TestDirEntry {
    Directory(TestDirectoryHandle),
    File(TestFileHandle),
}

pub struct TestDirectoryHandle {
    fs: TestFs,
    path: AbsPathBuf,
}

pub struct TestFileHandle {
    fs: TestFs,
    path: AbsPathBuf,
}

pin_project_lite::pin_project! {
    pub struct TestReadDir {
        dir_handle: TestDirectoryHandle,
        next_child_idx: usize,
    }
}

pin_project_lite::pin_project! {
    pub struct TestWatcher {
        fs: TestFs,
        path: AbsPathBuf,
        #[pin]
        inner: async_broadcast::Receiver<FsEvent<TestFs>>,
    }

    impl PinnedDrop for TestWatcher {
        fn drop(this: Pin<&mut Self>) {
            this.fs.with_inner(|inner| inner.watchers.remove(&this.path));
        }
    }
}

struct TestFsInner {
    root: TestFsNode,
    timestamp: TestTimestamp,
    watchers: FxHashMap<AbsPathBuf, TestWatchChannel>,
}

struct TestWatchChannel {
    inactive_rx: async_broadcast::InactiveReceiver<FsEvent<TestFs>>,
    tx: async_broadcast::Sender<FsEvent<TestFs>>,
}

impl TestFs {
    pub fn new(root: TestDirectory) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TestFsInner {
                root: TestFsNode::Directory(root),
                timestamp: TestTimestamp(0),
                watchers: FxHashMap::default(),
            })),
        }
    }

    #[allow(clippy::unwrap_used)]
    fn with_inner<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&mut TestFsInner) -> T,
    {
        let mut inner = self.inner.lock().unwrap();
        f(&mut inner)
    }
}

impl TestDirEntry {
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

impl TestDirectoryHandle {
    fn exists(&self) -> bool {
        self.fs.with_inner(|inner| {
            matches!(
                inner.node_at_path(&self.path),
                Some(TestFsNode::Directory(_))
            )
        })
    }
}

impl TestFileHandle {
    fn exists(&self) -> bool {
        self.fs.with_inner(|inner| {
            matches!(inner.node_at_path(&self.path), Some(TestFsNode::File(_)))
        })
    }
}

impl TestFsInner {
    fn dir_at_path(&self, path: &AbsPath) -> Option<&TestDirectory> {
        if path.is_root() {
            Some(self.root())
        } else {
            self.root().dir_at_path(path)
        }
    }

    fn file_at_path(&self, path: &AbsPath) -> Option<&TestFile> {
        self.root().file_at_path(path)
    }

    fn node_at_path(&self, path: &AbsPath) -> Option<&TestFsNode> {
        if path.is_root() {
            Some(&self.root)
        } else {
            self.root().child_at_path(path)
        }
    }

    fn root(&self) -> &TestDirectory {
        match &self.root {
            TestFsNode::Directory(dir) => dir,
            _ => unreachable!("root is always a directory"),
        }
    }
}

impl TestFsNode {
    fn kind(&self) -> FsNodeKind {
        match self {
            Self::File(_) => FsNodeKind::File,
            Self::Directory(_) => FsNodeKind::Directory,
        }
    }
}

impl TestDirectory {
    #[track_caller]
    pub fn insert_child(
        &mut self,
        name: impl AsRef<FsNodeName>,
        child: impl Into<TestFsNode>,
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

    fn child_at_path(&self, path: &AbsPath) -> Option<&TestFsNode> {
        let mut components = path.components();
        let node = self.children.get(components.next()?)?;
        match node {
            TestFsNode::Directory(dir) => {
                let path = components.as_path();
                if path.is_root() {
                    Some(node)
                } else {
                    dir.child_at_path(path)
                }
            },
            TestFsNode::File(_) => components.next().is_none().then_some(node),
        }
    }

    fn dir_at_path(&self, path: &AbsPath) -> Option<&Self> {
        match self.child_at_path(path)? {
            TestFsNode::Directory(dir) => Some(dir),
            _ => None,
        }
    }

    fn file_at_path(&self, path: &AbsPath) -> Option<&TestFile> {
        match self.child_at_path(path)? {
            TestFsNode::File(file) => Some(file),
            _ => None,
        }
    }
}

impl TestFile {
    pub fn contents(&self) -> &[u8] {
        &self.contents
    }

    pub fn new<C: AsRef<[u8]>>(contents: C) -> Self {
        Self { contents: contents.as_ref().to_owned() }
    }
}

impl TestWatchChannel {
    const CAPACITY: usize = 16;

    fn new() -> Self {
        let (tx, rx) = async_broadcast::broadcast(Self::CAPACITY);
        Self { tx, inactive_rx: rx.deactivate() }
    }

    fn rx(&self) -> async_broadcast::Receiver<FsEvent<TestFs>> {
        self.inactive_rx.activate_cloned()
    }
}

impl Fs for TestFs {
    type Timestamp = TestTimestamp;
    type DirEntry = TestDirEntry;
    type Directory<Path> = TestDirectoryHandle;
    type File<Path> = TestFileHandle;
    type ReadDir = TestReadDir;
    type Watcher = TestWatcher;
    type DirEntryError = TestReadDirNextError;
    type NodeAtPathError = Infallible;
    type ReadDirError = TestReadDirError;
    type WatchError = Infallible;

    async fn node_at_path<P: AsRef<AbsPath>>(
        &self,
        path: P,
    ) -> Result<Option<FsNode<Self, P>>, Self::NodeAtPathError> {
        let path = path.as_ref();
        let Some(kind) = self.with_inner(|inner| {
            inner.node_at_path(path).map(TestFsNode::kind)
        }) else {
            return Ok(None);
        };
        let node = match kind {
            FsNodeKind::File => FsNode::File(TestFileHandle {
                fs: self.clone(),
                path: path.to_owned(),
            }),
            FsNodeKind::Directory => FsNode::Directory(TestDirectoryHandle {
                fs: self.clone(),
                path: path.to_owned(),
            }),
            FsNodeKind::Symlink => todo!("can't handle symlinks yet"),
        };
        Ok(Some(node))
    }

    fn now(&self) -> Self::Timestamp {
        self.with_inner(|inner| inner.timestamp)
    }

    async fn read_dir<P: AsRef<AbsPath>>(
        &self,
        dir_path: P,
    ) -> Result<Self::ReadDir, Self::ReadDirError> {
        let FsNode::Directory(dir_handle) = self
            .node_at_path(dir_path)
            .await?
            .ok_or(TestReadDirError::NoNodeAtPath)?
        else {
            return Err(TestReadDirError::NoDirAtPath);
        };
        Ok(TestReadDir { dir_handle, next_child_idx: 0 })
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
                .or_insert_with(TestWatchChannel::new)
                .rx()
        });
        Ok(TestWatcher { inner: rx, fs: self.clone(), path })
    }
}

impl From<TestDirectory> for TestFsNode {
    fn from(dir: TestDirectory) -> Self {
        Self::Directory(dir)
    }
}

impl From<TestFile> for TestFsNode {
    fn from(file: TestFile) -> Self {
        Self::File(file)
    }
}

impl PartialEq for TestFile {
    fn eq(&self, other: &Self) -> bool {
        self.contents == other.contents
    }
}

impl PartialEq for TestDirectory {
    fn eq(&self, other: &Self) -> bool {
        self.children == other.children
    }
}

impl DirEntry for TestDirEntry {
    type MetadataError = TestDirEntryDoesNotExistError;
    type NameError = TestDirEntryDoesNotExistError;
    type NodeKindError = TestDirEntryDoesNotExistError;

    async fn metadata(&self) -> Result<Metadata, Self::MetadataError> {
        todo!()
    }

    async fn name(&self) -> Result<Cow<'_, FsNodeName>, Self::NameError> {
        self.exists()
            .then(|| self.path().fs_node_name().expect("path is not root"))
            .map(Cow::Borrowed)
            .ok_or(TestDirEntryDoesNotExistError)
    }

    async fn node_kind(&self) -> Result<FsNodeKind, Self::NodeKindError> {
        self.exists()
            .then_some(self.kind())
            .ok_or(TestDirEntryDoesNotExistError)
    }
}

impl Stream for TestReadDir {
    type Item = Result<TestDirEntry, TestReadDirNextError>;

    fn poll_next(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let (name, kind) = match this.dir_handle.fs.with_inner(|inner| {
            Ok(inner
                .dir_at_path(&this.dir_handle.path)
                .ok_or(TestReadDirNextError::DirWasDeleted)?
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
            FsNodeKind::File => TestDirEntry::File(TestFileHandle {
                fs: this.dir_handle.fs.clone(),
                path: child_path,
            }),
            FsNodeKind::Directory => {
                TestDirEntry::Directory(TestDirectoryHandle {
                    fs: this.dir_handle.fs.clone(),
                    path: child_path,
                })
            },
            FsNodeKind::Symlink => todo!("can't handle symlinks yet"),
        };
        Poll::Ready(Some(Ok(entry)))
    }
}

impl Stream for TestWatcher {
    type Item = Result<FsEvent<TestFs>, Infallible>;

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

#[derive(Debug, thiserror::Error)]
#[error("dir entry does not exist")]
pub struct TestDirEntryDoesNotExistError;

#[derive(Debug, thiserror::Error)]
pub enum TestReadDirError {
    #[error("no node at path")]
    NoNodeAtPath,
    #[error("no directory at path")]
    NoDirAtPath,
}

#[derive(Debug, thiserror::Error)]
pub enum TestReadDirNextError {
    #[error("directory has been deleted")]
    DirWasDeleted,
}

impl From<Infallible> for TestReadDirError {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
