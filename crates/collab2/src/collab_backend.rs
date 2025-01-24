use nvimx2::backend::{Backend, BufferId};
use nvimx2::fs::{self, AbsPathBuf};
use nvimx2::{AsyncCtx, notify};

/// TODO: docs.
pub trait CollabBackend: Backend<Fs: WithHomeDirFs> {
    /// TODO: docs.
    type SearchProjectRootError: notify::Error;

    /// Searches for the root of the project containing the buffer with the
    /// given ID.
    fn search_project_root(
        buffer_id: BufferId<Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::SearchProjectRootError>>;
}

/// TODO: docs.
pub trait WithHomeDirFs: fs::Fs {
    /// TODO: docs.
    type HomeDirError: notify::Error;

    /// TODO: docs.
    fn home_dir(
        &mut self,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::HomeDirError>>;
}

#[cfg(feature = "neovim")]
mod neovim {
    use mlua::{Function, Table};
    use nvimx2::backend::Buffer;
    use nvimx2::fs::{self, Fs};
    use nvimx2::neovim::{Neovim, NeovimBuffer, NeovimFs, mlua};

    use super::*;

    const MARKERS: Markers = root_markers::GitDirectory;

    pub type Markers = root_markers::GitDirectory;

    pub enum NeovimFindProjectRootError {
        LspRootDirNotAbsolute(fs::AbsPathNotAbsoluteError),
        CouldntFindRoot,
        HomeDir(NeovimHomeDirError),
        InvalidBufferPath(String),
        MarkedRoot(root_markers::FindRootError<NeovimFs, Markers>),
        IsParentDir(<NeovimFs as Fs>::NodeAtPathError),
    }

    pub enum NeovimHomeDirError {
        CouldntFindHome,
        InvalidHomeDir(fs::AbsPathFromPathError),
    }

    impl CollabBackend for Neovim {
        type SearchProjectRootError = NeovimFindProjectRootError;

        async fn search_project_root(
            buffer: NeovimBuffer,
            ctx: &mut AsyncCtx<'_, Self>,
        ) -> Result<AbsPathBuf, Self::SearchProjectRootError> {
            if let Some(lsp_root) = lsp_root(buffer) {
                return lsp_root.as_str().try_into().map_err(
                    NeovimFindProjectRootError::LspRootDirNotAbsolute,
                );
            }

            let buffer_path =
                buffer.name().parse::<AbsPathBuf>().map_err(|_| {
                    NeovimFindProjectRootError::InvalidBufferPath(
                        buffer.name().into_owned(),
                    )
                })?;

            let mut fs = ctx.fs();

            let home_dir = fs
                .home_dir()
                .await
                .map_err(NeovimFindProjectRootError::HomeDir)?;

            let args = root_markers::FindRootArgs {
                marker: MARKERS,
                start_from: &buffer_path,
                stop_at: Some(&home_dir),
            };

            if let Some(res) = args.find(&mut fs).await.transpose() {
                return res.map_err(NeovimFindProjectRootError::MarkedRoot);
            }

            let buffer_parent = buffer_path
                .parent()
                .ok_or(NeovimFindProjectRootError::CouldntFindRoot)?;

            fs.is_dir(buffer_parent)
                .await
                .map_err(NeovimFindProjectRootError::IsParentDir)?
                .then(|| buffer_parent.to_owned())
                .ok_or(NeovimFindProjectRootError::CouldntFindRoot)
        }
    }

    impl WithHomeDirFs for NeovimFs {
        type HomeDirError = NeovimHomeDirError;

        async fn home_dir(
            &mut self,
        ) -> Result<AbsPathBuf, Self::HomeDirError> {
            match home::home_dir() {
                Some(home_dir) if !home_dir.as_os_str().is_empty() => home_dir
                    .try_into()
                    .map_err(NeovimHomeDirError::InvalidHomeDir),
                _ => Err(NeovimHomeDirError::CouldntFindHome),
            }
        }
    }

    /// Returns the root directory of the first language server attached to the
    /// given buffer, if any.
    fn lsp_root(buffer: NeovimBuffer) -> Option<String> {
        let lua = mlua::lua();

        let get_clients = lua
            .globals()
            .get::<Table>("vim")
            .ok()?
            .get::<Table>("lsp")
            .ok()?
            .get::<Function>("get_clients")
            .ok()?;

        let opts = lua.create_table().ok()?;
        opts.set("bufnr", buffer).ok()?;

        get_clients
            .call::<Table>(opts)
            .ok()?
            .get::<Table>(1)
            .ok()?
            .get::<Table>("config")
            .ok()?
            .get::<String>("root_dir")
            .ok()
    }

    impl notify::Error for NeovimFindProjectRootError {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            todo!()
        }
    }

    impl notify::Error for NeovimHomeDirError {
        fn to_message(&self) -> (notify::Level, notify::Message) {
            todo!()
        }
    }
}

#[cfg(feature = "neovim")]
mod root_markers {
    use futures_util::stream::{self, StreamExt};
    use futures_util::{pin_mut, select};
    use nvimx2::fs::{self, DirEntry};

    pub struct FindRootArgs<'a, M> {
        /// The marker used to determine if a directory is the root.
        pub(super) marker: M,

        /// The path to the first directory to search for markers in.
        ///
        /// If this points to a file, the search will start from its parent.
        pub(super) start_from: &'a fs::AbsPath,

        /// The path to the last directory to search for markers in, if any.
        ///
        /// If set and no root marker is found within it, the search is cut
        /// short instead of continuing with its parent.
        pub(super) stop_at: Option<&'a fs::AbsPath>,
    }

    pub struct GitDirectory;

    pub trait RootMarker<Fs: fs::Fs> {
        type Error;

        fn matches(
            &self,
            dir_entry: &Fs::DirEntry,
        ) -> impl Future<Output = Result<bool, Self::Error>>;
    }

    pub enum FindRootError<Fs: fs::Fs, M: RootMarker<Fs>> {
        /// TODO: docs.
        DirEntry {
            /// TODO: docs.
            path: fs::AbsPathBuf,
            /// TODO: docs.
            err: DirEntryError<Fs>,
        },

        /// TODO: docs.
        Marker(M::Error),

        /// TODO: docs.
        NodeAtStartPath(Fs::NodeAtPathError),

        /// TODO: docs.
        ReadDir {
            /// TODO: docs.
            dir_path: fs::AbsPathBuf,
            /// TODO: docs.
            err: Fs::ReadDirError,
        },

        /// TODO: docs.
        StartPathNotFound,
    }

    pub enum DirEntryError<Fs: fs::Fs> {
        Access(Fs::DirEntryError),
        Name(<Fs::DirEntry as fs::DirEntry>::NameError),
        NodeKind(<Fs::DirEntry as fs::DirEntry>::NodeKindError),
    }

    impl<M> FindRootArgs<'_, M> {
        pub(super) async fn find<Fs>(
            self,
            fs: &mut Fs,
        ) -> Result<Option<fs::AbsPathBuf>, FindRootError<Fs, M>>
        where
            Fs: fs::Fs,
            M: RootMarker<Fs>,
        {
            let node_kind = fs
                .node_at_path(self.start_from)
                .await
                .map_err(FindRootError::NodeAtStartPath)?
                .ok_or(FindRootError::StartPathNotFound)?
                .kind();

            let mut dir = match node_kind {
                fs::FsNodeKind::Directory => self.start_from,
                fs::FsNodeKind::File => self
                    .start_from
                    .parent()
                    .expect("path is of file, so it must have a parent"),
                fs::FsNodeKind::Symlink => todo!("can't handle symlinks yet"),
            }
            .to_owned();

            loop {
                if self.contains_marker(&dir, fs).await? {
                    return Ok(Some(dir));
                }
                if self.stop_at == Some(&*dir) {
                    return Ok(None);
                }
                if !dir.pop() {
                    return Ok(None);
                }
            }
        }

        async fn contains_marker<Fs>(
            &self,
            dir_path: &fs::AbsPath,
            fs: &mut Fs,
        ) -> Result<bool, FindRootError<Fs, M>>
        where
            Fs: fs::Fs,
            M: RootMarker<Fs>,
        {
            let read_dir = fs
                .read_dir(dir_path)
                .await
                .map_err(|err| FindRootError::ReadDir {
                    dir_path: dir_path.to_owned(),
                    err,
                })?
                .fuse();

            pin_mut!(read_dir);

            let mut check_marker_matches = stream::FuturesUnordered::new();

            loop {
                select! {
                    read_res = read_dir.select_next_some() => {
                        let dir_entry =
                            read_res.map_err(|err| FindRootError::DirEntry {
                                path: dir_path.to_owned(),
                                err: DirEntryError::Access(err),
                            })?;

                        let fut = async move {
                            self.marker
                                .matches(&dir_entry)
                                .await
                                .map_err(FindRootError::Marker)
                        };

                        check_marker_matches.push(fut);
                    },

                    marker_res = check_marker_matches.select_next_some() => {
                        match marker_res {
                            Ok(true) => return Ok(true),
                            Ok(false) => continue,
                            Err(err) => return Err(err),
                        }
                    },

                    complete => return Ok(false),
                }
            }
        }
    }

    impl<Fs: fs::Fs> RootMarker<Fs> for GitDirectory {
        type Error = DirEntryError<Fs>;

        async fn matches(
            &self,
            dir_entry: &Fs::DirEntry,
        ) -> Result<bool, Self::Error> {
            Ok(dir_entry.name().await.map_err(DirEntryError::Name)?.as_ref()
                == ".git"
                && dir_entry
                    .is_directory()
                    .await
                    .map_err(DirEntryError::NodeKind)?)
        }
    }
}
