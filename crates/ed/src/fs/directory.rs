use core::error::Error;

use abs_path::AbsPathBuf;
use futures_util::stream::{self, Stream, StreamExt};
use futures_util::{FutureExt, pin_mut, select_biased};

use crate::fs::{self, AbsPath, File, Fs, Metadata, NodeName, Symlink};

/// TODO: docs.
pub trait Directory: Send + Sync + Sized {
    /// TODO: docs.
    type EventStream: Stream<Item = DirectoryEvent<Self::Fs>> + Send + Unpin;

    /// TODO: docs.
    type Fs: Fs;

    /// TODO: docs.
    type CreateDirectoryError: Error + Send;

    /// TODO: docs.
    type CreateFileError: Error + Send;

    /// TODO: docs.
    type CreateSymlinkError: Error + Send;

    /// TODO: docs.
    type ClearError: Error + Send;

    /// TODO: docs.
    type DeleteError: Error + Send;

    /// TODO: docs.
    type ListError: Error + Send;

    /// TODO: docs.
    type ParentError: Error + Send;

    /// TODO: docs.
    type ReadMetadataError: Error + Send;

    /// TODO: docs.
    fn create_directory(
        &self,
        directory_name: &NodeName,
    ) -> impl Future<
        Output = Result<
            <Self::Fs as Fs>::Directory,
            Self::CreateDirectoryError,
        >,
    > + Send;

    /// TODO: docs.
    fn create_file(
        &self,
        file_name: &NodeName,
    ) -> impl Future<
        Output = Result<<Self::Fs as Fs>::File, Self::CreateFileError>,
    > + Send;

    /// TODO: docs.
    fn create_symlink(
        &self,
        symlink_name: &NodeName,
        target_path: &str,
    ) -> impl Future<
        Output = Result<<Self::Fs as Fs>::Symlink, Self::CreateSymlinkError>,
    > + Send;

    /// TODO: docs.
    fn clear(&self) -> impl Future<Output = Result<(), Self::ClearError>>;

    /// TODO: docs.
    fn delete(
        self,
    ) -> impl Future<Output = Result<(), Self::DeleteError>> + Send;

    /// TODO: docs.
    #[inline]
    fn id(&self) -> <Self::Fs as Fs>::NodeId {
        fs::Metadata::id(&self.meta())
    }

    /// TODO: docs.
    fn meta(&self) -> <Self::Fs as Fs>::Metadata;

    /// TODO: docs.
    #[inline]
    fn name(&self) -> Option<&NodeName> {
        self.path().node_name()
    }

    /// TODO: docs.
    fn parent(
        &self,
    ) -> impl Future<
        Output = Result<
            Option<<Self::Fs as Fs>::Directory>,
            Self::ParentError,
        >,
    > + Send;

    /// TODO: docs.
    fn path(&self) -> &AbsPath;

    /// TODO: docs.
    #[allow(clippy::type_complexity)]
    fn list_metas(
        &self,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<
                    <Self::Fs as Fs>::Metadata,
                    Self::ReadMetadataError,
                >,
            > + Send
            + Unpin
            + use<Self>,
            Self::ListError,
        >,
    > + Send;

    /// TODO: docs.
    #[allow(clippy::type_complexity)]
    fn list_nodes(
        &self,
    ) -> impl Future<
        Output = Result<
            impl Stream<
                Item = Result<fs::FsNode<Self::Fs>, ReadNodeError<Self::Fs>>,
            > + Send,
            Self::ListError,
        >,
    > + Send
    where
        Self: AsRef<Self::Fs>,
        <Self::Fs as fs::Fs>::Directory:
            Directory<ReadMetadataError = Self::ReadMetadataError>,
    {
        async move {
            let metas = self.list_metas().await?.fuse();
            let get_nodes = stream::FuturesUnordered::new();
            Ok(stream::unfold(
                (metas, get_nodes),
                move |(mut metas, mut get_nodes)| async move {
                    let node_res = loop {
                        select_biased! {
                            meta_res = metas.select_next_some() => {
                                let metadata = match meta_res {
                                    Ok(meta) => meta,
                                    Err(err) => {
                                        break Err(ReadNodeError::ReadMetadata(
                                            err,
                                        ));
                                    }
                                };
                                let node_name = match metadata.name() {
                                    Ok(name) => name,
                                    Err(err) => {
                                        break Err(ReadNodeError::MetadataName(
                                            err,
                                        ));
                                    }
                                };
                                let node_path = self.path().join(node_name);
                                get_nodes.push(async move {
                                    self.as_ref()
                                        .node_at_path(&node_path)
                                        .await
                                });
                            },
                            node_res = get_nodes.select_next_some() => {
                                match node_res {
                                    Ok(Some(node)) => break Ok(node),
                                    // The node must've been deleted.
                                    Ok(None) => {},
                                    Err(err) => {
                                        break Err(ReadNodeError::NodeAtPath(
                                            err
                                        ));
                                    },
                                }
                            },
                            complete => return None,
                        }
                    };
                    Some((node_res, (metas, get_nodes)))
                },
            ))
        }
    }

    /// TODO: docs.
    #[inline]
    fn replicate_from<Src>(
        &self,
        src: &Src,
    ) -> impl Future<Output = Result<(), ReplicateError<Self::Fs, Src::Fs>>> + Send
    where
        <Self::Fs as fs::Fs>::Directory: Directory<
                CreateDirectoryError = Self::CreateDirectoryError,
                CreateFileError = Self::CreateFileError,
                CreateSymlinkError = Self::CreateSymlinkError,
            >,
        Src: Directory + AsRef<Src::Fs>,
        <Src::Fs as Fs>::Directory: Directory<
                ReadMetadataError = Src::ReadMetadataError,
                ListError = Src::ListError,
            > + AsRef<Src::Fs>,
    {
        async move {
            let list_nodes = src
                .list_nodes()
                .await
                .map_err(ReplicateError::ListDirectory)?
                .fuse();

            let mut replicate_nodes = stream::FuturesUnordered::new();

            pin_mut!(list_nodes);

            loop {
                select_biased! {
                    node_res = list_nodes.select_next_some() => {
                        let node = node_res.map_err(ReplicateError::ReadNode)?;
                        replicate_nodes.push(async move {
                            replicate_node(self, &node).await
                        });
                    },
                    replicate_res = replicate_nodes.select_next_some() => {
                        replicate_res?;
                    },
                    complete => return Ok(()),
                }
            }
        }
    }

    /// TODO: docs.
    fn watch(&self) -> Self::EventStream;
}

/// TODO: docs.
pub enum DirectoryEvent<Fs: fs::Fs> {
    /// TODO: docs.
    Creation(NodeCreation<Fs>),

    /// TODO: docs.
    Deletion(NodeDeletion<Fs>),

    /// TODO: docs.
    Move(NodeMove<Fs>),
}

/// TODO: docs.
pub struct NodeCreation<Fs: fs::Fs> {
    /// TODO: docs.
    pub node_id: Fs::NodeId,

    /// TODO: docs.
    pub node_path: AbsPathBuf,

    /// TODO: docs.
    pub parent_id: Fs::NodeId,
}

/// TODO: docs.
pub struct NodeDeletion<Fs: fs::Fs> {
    /// The ID of the node that was deleted.
    pub node_id: Fs::NodeId,

    /// The path to the node at the time of its deletion.
    pub node_path: AbsPathBuf,

    /// TODO: docs.
    pub deletion_root_id: Fs::NodeId,
}

/// TODO: docs.
pub struct NodeMove<Fs: fs::Fs> {
    /// The ID of the node that was moved.
    pub node_id: Fs::NodeId,

    /// The path to the node before it was moved.
    pub old_path: AbsPathBuf,

    /// The path to the node after it was moved.
    pub new_path: AbsPathBuf,

    /// TODO: docs.
    pub move_root_id: Fs::NodeId,
}

/// TODO: docs.
#[derive(
    cauchy::Debug,
    derive_more::Display,
    cauchy::Error,
    cauchy::PartialEq,
    cauchy::Eq,
)]
#[display("{_0}")]
pub enum ReadNodeError<Fs: fs::Fs> {
    /// TODO: docs.
    MetadataName(fs::MetadataNameError),

    /// TODO: docs.
    NodeAtPath(Fs::NodeAtPathError),

    /// TODO: docs.
    ReadMetadata(<Fs::Directory as Directory>::ReadMetadataError),
}

/// TODO: docs.
#[derive(
    cauchy::Debug,
    derive_more::Display,
    cauchy::Error,
    cauchy::PartialEq,
    cauchy::Eq,
    cauchy::From,
)]
#[display("{_0}")]
pub enum ReplicateError<Dst: Fs, Src: Fs> {
    /// TODO: docs.
    CreateDirectory(<Dst::Directory as Directory>::CreateDirectoryError),

    /// TODO: docs.
    CreateFile(<Dst::Directory as fs::Directory>::CreateFileError),

    /// TODO: docs.
    CreateSymlink(<Dst::Directory as fs::Directory>::CreateSymlinkError),

    /// TODO: docs.
    ListDirectory(<Src::Directory as Directory>::ListError),

    /// TODO: docs.
    ReadFile(<Src::File as fs::File>::ReadError),

    /// TODO: docs.
    ReadNode(ReadNodeError<Src>),

    /// TODO: docs.
    ReadSymlink(<Src::Symlink as fs::Symlink>::ReadError),

    /// TODO: docs.
    WriteFile(<Dst::File as fs::File>::WriteError),
}

#[inline]
async fn replicate_node<Dst: Directory, Src: fs::Fs>(
    dst_dir: &Dst,
    src_node: &fs::FsNode<Src>,
) -> Result<(), ReplicateError<Dst::Fs, Src>>
where
    <Dst::Fs as fs::Fs>::Directory: Directory<
            CreateDirectoryError = Dst::CreateDirectoryError,
            CreateFileError = Dst::CreateFileError,
            CreateSymlinkError = Dst::CreateSymlinkError,
        >,
    Src::Directory: AsRef<Src>,
{
    match src_node {
        fs::FsNode::Directory(src_dir) => {
            let src_dir_name = src_dir.name().expect("dir is not root");

            let dst_dir = dst_dir
                .create_directory(src_dir_name)
                .await
                .map_err(ReplicateError::CreateDirectory)?;

            dst_dir.replicate_from(src_dir).boxed().await?;
        },
        fs::FsNode::File(src_file) => {
            let mut dst_file = dst_dir
                .create_file(src_file.name())
                .await
                .map_err(ReplicateError::CreateFile)?;

            let src_contents =
                src_file.read().await.map_err(ReplicateError::ReadFile)?;

            dst_file
                .write(src_contents)
                .await
                .map_err(ReplicateError::WriteFile)?;
        },
        fs::FsNode::Symlink(src_symlink) => {
            let src_target_path = src_symlink
                .read_path()
                .await
                .map_err(ReplicateError::ReadSymlink)?;

            dst_dir
                .create_symlink(src_symlink.name(), &src_target_path)
                .await
                .map_err(ReplicateError::CreateSymlink)?;
        },
    }

    Ok(())
}
