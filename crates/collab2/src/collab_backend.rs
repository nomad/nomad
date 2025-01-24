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

    pub enum NeovimFindProjectRootError {
        LspRootDirNotAbsolute(fs::AbsPathNotAbsoluteError),
        CouldntFindRoot,
        HomeDir(NeovimHomeDirError),
        InvalidBufferPath(String),
        MarkedRoot(root_markers::FindRootError<NeovimFs>),
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
                marker: root_markers::GitDirectory,
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
    use nvimx2::fs::{self, DirEntry};

    pub(super) struct FindRootArgs<'a, M> {
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

    impl<M> FindRootArgs<'_, M> {
        pub(super) async fn find<Fs>(
            self,
            fs: &mut Fs,
        ) -> Result<Option<fs::AbsPathBuf>, FindRootError<Fs>>
        where
            Fs: fs::Fs,
            M: RootMarker<Fs>,
        {
            todo!();
        }
    }

    pub(super) trait RootMarker<Fs: fs::Fs> {
        type Error;

        fn matches(
            &self,
            dir_entry: &Fs::DirEntry,
        ) -> impl Future<Output = Result<bool, Self::Error>>;
    }

    pub(super) struct GitDirectory;

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

    pub(super) enum FindRootError<Fs: fs::Fs> {
        /// TODO: docs.
        DirEntry {
            /// TODO: docs.
            path: fs::AbsPathBuf,
            /// TODO: docs.
            err: DirEntryError<Fs>,
        },

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

    pub(super) enum DirEntryError<Fs: fs::Fs> {
        Access(Fs::DirEntryError),
        Name(<Fs::DirEntry as fs::DirEntry>::NameError),
        NodeKind(<Fs::DirEntry as fs::DirEntry>::NodeKindError),
    }
}
