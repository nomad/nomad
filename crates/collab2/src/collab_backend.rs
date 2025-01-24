use nvimx2::backend::{Backend, BufferId};
use nvimx2::fs::AbsPathBuf;
use nvimx2::{AsyncCtx, notify};

/// TODO: docs.
pub trait CollabBackend: Backend {
    /// TODO: docs.
    type FindProjectRootError: notify::Error;

    /// Tries to find the absolute path to the root of the project containing
    /// the buffer with the given ID.
    fn find_project_root(
        buffer_id: BufferId<Self>,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> impl Future<Output = Result<AbsPathBuf, Self::FindProjectRootError>>;
}

#[cfg(feature = "neovim")]
mod neovim {
    use mlua::{Function, Table};
    use nvimx2::fs::{self, Fs};
    use nvimx2::neovim::{Neovim, NeovimBuffer, NeovimFs, mlua};

    use super::*;

    pub enum NeovimFindProjectRootError {
        LspRootDirNotAbsolute(fs::AbsPathNotAbsoluteError),
        CouldntFindRoot,
        MarkedRoot(root_markers::FindRootError<NeovimFs>),
        IsParentDir(<NeovimFs as Fs>::NodeAtPathError),
    }

    impl CollabBackend for Neovim {
        type FindProjectRootError = NeovimFindProjectRootError;

        async fn find_project_root(
            buffer: NeovimBuffer,
            ctx: &mut AsyncCtx<'_, Self>,
        ) -> Result<AbsPathBuf, Self::FindProjectRootError> {
            if let Some(lsp_root) = lsp_root(buffer) {
                return lsp_root.as_str().try_into().map_err(
                    NeovimFindProjectRootError::LspRootDirNotAbsolute,
                );
            }

            let buffer_path: AbsPathBuf = todo!();

            let home_dir: AbsPathBuf = todo!();

            let mut fs = ctx.fs();

            if let Some(res) = root_markers::find_root(
                &buffer_path,
                Some(&home_dir),
                root_markers::GitDirectory,
                &fs,
            )
            .await
            .transpose()
            {
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
}

#[cfg(feature = "neovim")]
mod root_markers {
    use nvimx2::fs::{self, DirEntry};

    ///
    pub(super) async fn find_root<Rm, Fs>(
        start_from: &fs::AbsPath,
        stop_at: Option<&fs::AbsPath>,
        marker: Rm,
        fs: &Fs,
    ) -> Result<Option<fs::AbsPathBuf>, FindRootError<Fs>>
    where
        Rm: RootMarker<Fs>,
        Fs: fs::Fs,
    {
        todo!();
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

        #[inline]
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
