use core::fmt;
use core::ops::Range;
use std::ffi::OsString;
use std::path::PathBuf;
use std::{env, io};

use abs_path::{AbsPath, AbsPathBuf, AbsPathFromPathError, node};
use collab_types::Peer;
use collab_types::nomad::ulid;
use editor::module::{Action, Module};
use editor::{ByteOffset, Context, Editor};
use executor::Executor;
use fs::Directory;
use mlua::{Function, Table};
use neovim::buffer::{BufferExt, BufferId, HighlightRangeHandle, Point};
use neovim::notify::ContextExt;
use neovim::{Neovim, mlua, oxi};

use crate::editors::{ActionForSelectedSession, CollabEditor};
use crate::session::{SessionError, SessionInfos};
use crate::{Collab, config, join, leave, start, yank};

pub type SessionId = ulid::Ulid;

pub struct NeovimPeerSelection {
    selection_highlight_handle: HighlightRangeHandle,
}

pub struct PeerTooltip {
    /// The buffer this tooltip is in.
    buffer: oxi::api::Buffer,

    /// The extmark ID of the highlight representing the remote peer's cursor.
    cursor_extmark_id: u32,

    /// The ID of the namespace the
    /// [`cursor_extmark_id`](Self::cursor_extmark_id) belongs to.
    namespace_id: u32,

    /// The remote peer this tooltip is for.
    peer: Peer,
}

#[derive(Debug, derive_more::Display, cauchy::Error, cauchy::PartialEq)]
#[display("couldn't copy {} to clipboard: {}", session_id, inner)]
pub struct NeovimCopySessionIdError {
    inner: clipboard::ClipboardError,
    session_id: SessionId,
}

#[derive(Debug, derive_more::Display, cauchy::Error)]
#[display("couldn't connect to the server: {inner}")]
pub struct NeovimConnectToServerError {
    inner: io::Error,
}

#[derive(Debug, derive_more::Display, cauchy::Error)]
pub enum NeovimDataDirError {
    #[display("{_0}")]
    Home(NeovimHomeDirError),

    #[display("found data directory at {_0:?}, but it's not an absolute path")]
    XdgDataHomeNotAbsolute(String),

    #[display(
        "found data directory at {_0:?}, but it's not a valid UTF-8 string"
    )]
    XdgDataHomeNotUtf8(OsString),
}

#[derive(Debug, derive_more::Display, cauchy::Error)]
pub enum NeovimHomeDirError {
    #[display("Couldn't find the home directory")]
    CouldntFindHome,

    #[display("Found home directory at {_0:?}, but it's not an absolute path")]
    HomeDirNotAbsolute(PathBuf),

    #[display(
        "Found home directory at {_0:?}, but it's not a valid UTF-8 string"
    )]
    HomeDirNotUtf8(PathBuf),
}

#[derive(Debug, derive_more::Display, cauchy::Error)]
#[display("LSP root at {root_dir} is not an absolute path")]
pub struct NeovimLspRootError {
    root_dir: String,
}

/// An [`AbsPath`] wrapper whose `Display` impl replaces the path's home
/// directory with `~`.
struct TildePath<'a> {
    path: &'a AbsPath,
    home_dir: Option<&'a AbsPath>,
}

impl PeerTooltip {
    /// Creates a new tooltip representing the given remote peer's cursor at
    /// the given byte offset in the given buffer.
    fn create(
        peer: Peer,
        mut buffer: oxi::api::Buffer,
        cursor_offset: ByteOffset,
        namespace_id: u32,
    ) -> Self {
        let highlight_range = Self::highlight_range(&buffer, cursor_offset);

        let opts = oxi::api::opts::SetExtmarkOpts::builder()
            .end_row(highlight_range.end.line_idx)
            .end_col(highlight_range.end.byte_offset)
            .hl_group("TermCursor")
            .build();

        let cursor_extmark_id = buffer
            .set_extmark(
                namespace_id,
                highlight_range.start.line_idx,
                highlight_range.start.byte_offset,
                &opts,
            )
            .expect("couldn't set extmark");

        Self { buffer, cursor_extmark_id, peer, namespace_id }
    }

    /// Returns the [`Point`] range to be highlighted to represent the remote
    /// peer's cursor at the given byte offset.
    fn highlight_range(
        buffer: &oxi::api::Buffer,
        cursor_offset: ByteOffset,
    ) -> Range<Point> {
        debug_assert!(cursor_offset <= buffer.byte_len());

        let highlight_start = buffer.point_of_byte(cursor_offset);

        let is_cursor_at_eol = buffer
            .byte_len_of_line(highlight_start.line_idx)
            == highlight_start.byte_offset;

        let highlight_end =
            // If the cursor is at the end of the line, we set the end of the
            // highlighted range to the start of the next line.
            //
            // Apparently this works even if the cursor is on the last line,
            // and nvim_buf_set_extmark won't complain about it.
            if is_cursor_at_eol {
                Point::new(highlight_start.line_idx + 1, 0)
            }
            // If the cursor is in the middle of a line, we set the end of the
            // highlighted range one byte after the start.
            //
            // This works because Neovim already handles offset clamping for
            // us, so even if the grapheme to the immediate right of the cursor
            // is multi-byte, Neovim will automatically extend the highlight's
            // end to the end of the grapheme.
            else {
                Point::new(
                    highlight_start.line_idx,
                    highlight_start.byte_offset + 1,
                )
            };

        highlight_start..highlight_end
    }

    /// Moves the tooltip to the given offset.
    fn r#move(&mut self, cursor_offset: ByteOffset) {
        let highlight_range =
            Self::highlight_range(&self.buffer, cursor_offset);

        let opts = oxi::api::opts::SetExtmarkOpts::builder()
            .id(self.cursor_extmark_id)
            .end_row(highlight_range.end.line_idx)
            .end_col(highlight_range.end.byte_offset)
            .hl_group("TermCursor")
            .build();

        let new_extmark_id = self
            .buffer
            .set_extmark(
                self.namespace_id,
                highlight_range.start.line_idx,
                highlight_range.start.byte_offset,
                &opts,
            )
            .expect("couldn't set extmark");

        debug_assert_eq!(new_extmark_id, self.cursor_extmark_id);
    }

    /// Removes the tooltip from the buffer.
    fn remove(mut self) {
        self.buffer
            .del_extmark(self.namespace_id, self.cursor_extmark_id)
            .expect("couldn't delete extmark");
    }
}

impl CollabEditor for Neovim {
    type Io = async_net::TcpStream;
    type PeerSelection = NeovimPeerSelection;
    type PeerTooltip = PeerTooltip;
    type ProjectFilter = Option<gitignore::GitIgnore>;
    type ServerParams = collab_types::nomad::NomadParams;

    type ConnectToServerError = NeovimConnectToServerError;
    type CopySessionIdError = NeovimCopySessionIdError;
    type DefaultDirForRemoteProjectsError = NeovimDataDirError;
    type HomeDirError = NeovimHomeDirError;
    type LspRootError = NeovimLspRootError;
    type ProjectFilterError = gitignore::CreateError;

    async fn confirm_start(
        project_root: &AbsPath,
        ctx: &mut Context<Self>,
    ) -> bool {
        let prompt = format!(
            "Start collaborating on the project at \"{}\"?",
            TildePath {
                path: project_root,
                home_dir: Self::home_dir(ctx).await.ok().as_deref(),
            }
        );

        let options = ["Yes", "No"];

        let Ok(choice) = oxi::api::call_function::<_, u8>(
            "confirm",
            (prompt, options.join("\n")),
        ) else {
            return false;
        };

        match choice {
            0 | 2 => false,
            1 => true,
            _ => unreachable!("only provided {} options", options.len()),
        }
    }

    async fn connect_to_server(
        server_addr: config::ServerAddress,
        _: &mut Context<Self>,
    ) -> Result<Self::Io, Self::ConnectToServerError> {
        async_net::TcpStream::connect(&*server_addr)
            .await
            .map_err(|inner| NeovimConnectToServerError { inner })
    }

    async fn copy_session_id(
        session_id: SessionId,
        _: &mut Context<Self>,
    ) -> Result<(), Self::CopySessionIdError> {
        clipboard::set(session_id)
            .map_err(|inner| NeovimCopySessionIdError { inner, session_id })
    }

    fn create_peer_selection(
        _remote_peer: Peer,
        selected_range: Range<ByteOffset>,
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Self::PeerSelection {
        ctx.with_borrowed(|ctx| {
            let buffer = ctx.buffer(buffer_id).expect("invalid buffer ID");
            let hl_handle = buffer.highlight_range(selected_range, "Visual");
            NeovimPeerSelection { selection_highlight_handle: hl_handle }
        })
    }

    fn create_peer_tooltip(
        remote_peer: Peer,
        tooltip_offset: ByteOffset,
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Self::PeerTooltip {
        let buffer = oxi::api::Buffer::from(buffer_id);
        let namespace_id = ctx.with_editor(|nvim| nvim.namespace_id());
        PeerTooltip::create(remote_peer, buffer, tooltip_offset, namespace_id)
    }

    async fn default_dir_for_remote_projects(
        ctx: &mut Context<Self>,
    ) -> Result<AbsPathBuf, Self::DefaultDirForRemoteProjectsError> {
        let data_dir = match env::var("XDG_DATA_HOME") {
            Ok(xdg_data_home) => {
                xdg_data_home.parse::<AbsPathBuf>().map_err(|_| {
                    NeovimDataDirError::XdgDataHomeNotAbsolute(xdg_data_home)
                })?
            },
            Err(env::VarError::NotPresent) => Self::home_dir(ctx)
                .await
                .map_err(NeovimDataDirError::Home)?
                .join(node!(".local"))
                .join(node!("share")),
            Err(env::VarError::NotUnicode(xdg_data_home)) => {
                return Err(NeovimDataDirError::XdgDataHomeNotUtf8(
                    xdg_data_home,
                ));
            },
        };

        Ok(data_dir.join(node!("nomad")).join(node!("remote-projects")))
    }

    async fn home_dir(
        _: &mut Context<Self>,
    ) -> Result<AbsPathBuf, Self::HomeDirError> {
        match home::home_dir() {
            Some(home_dir) if !home_dir.as_os_str().is_empty() => {
                home_dir.as_path().try_into().map_err(|err| match err {
                    AbsPathFromPathError::NotAbsolute => {
                        NeovimHomeDirError::HomeDirNotAbsolute(home_dir)
                    },
                    AbsPathFromPathError::NotUtf8 => {
                        NeovimHomeDirError::HomeDirNotUtf8(home_dir)
                    },
                })
            },
            _ => Err(NeovimHomeDirError::CouldntFindHome),
        }
    }

    fn lsp_root(
        buffer_id: BufferId,
        _: &mut Context<Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError> {
        /// Returns the root directory of the first language server
        /// attached to the given buffer, if any.
        fn inner(buffer_id: BufferId) -> Option<String> {
            let lua = mlua::lua();

            let opts = lua.create_table().ok()?;
            opts.raw_set("bufnr", oxi::api::Buffer::from(buffer_id).handle())
                .ok()?;

            get_lua_value::<Function>(&["vim", "lsp", "get_clients"])?
                .call::<Table>(opts)
                .ok()?
                .get::<Table>(1)
                .ok()?
                .get::<Table>("config")
                .ok()?
                .get::<String>("root_dir")
                .ok()
        }

        let Some(root_dir) = inner(buffer_id) else { return Ok(None) };

        root_dir
            .parse::<AbsPathBuf>()
            .map(Some)
            .map_err(|_| NeovimLspRootError { root_dir })
    }

    fn move_peer_selection<'ctx>(
        selection: &mut Self::PeerSelection,
        selected_range: Range<ByteOffset>,
        ctx: &'ctx mut Context<Self>,
    ) {
        ctx.with_editor(|nvim| {
            nvim.highlight_range(&selection.selection_highlight_handle)
                .expect("invalid buffer ID")
                .r#move(selected_range);
        });
    }

    fn move_peer_tooltip(
        tooltip: &mut Self::PeerTooltip,
        tooltip_offset: ByteOffset,
        _: &mut Context<Self>,
    ) {
        tooltip.r#move(tooltip_offset);
    }

    fn on_join_error(error: join::JoinError<Self>, ctx: &mut Context<Self>) {
        match error {
            join::JoinError::UserNotLoggedIn => {
                ctx.notify_error(
                    "You must be logged in to join a collaborative editing \
                     session. You can log in by executing ':Mad login'",
                );
            },
            other => ctx.notify_error(other),
        }
    }

    fn on_leave_error(error: leave::LeaveError, ctx: &mut Context<Self>) {
        ctx.notify_error(error);
    }

    fn on_session_error(error: SessionError<Self>, ctx: &mut Context<Self>) {
        ctx.notify_error(error);
    }

    async fn on_session_started(
        infos: &SessionInfos<Self>,
        ctx: &mut Context<Self>,
    ) {
        let prompt = format!(
            "Started a new collaborative editing session at {} with ID \
             {}.\nYou can share this ID with other peers to let them join \
             the session. Would you like to copy it to the clipboard?",
            TildePath {
                path: &infos.project_root_path,
                home_dir: Self::home_dir(ctx).await.ok().as_deref(),
            },
            infos.session_id,
        );

        let options = ["Yes", "No"];

        let Ok(choice) = oxi::api::call_function::<_, u8>(
            "confirm",
            (prompt, options.join("\n")),
        ) else {
            return;
        };

        match choice {
            0 | 2 => return,
            1 => {},
            _ => unreachable!("only provided {} options", options.len()),
        }

        match Self::copy_session_id(infos.session_id, ctx).await {
            Ok(()) => ctx.notify_info(format_args!(
                "Session ID copied to clipboard. You can also yank it later \
                 by executing ':Mad {} {}'",
                Collab::<Self>::NAME,
                yank::Yank::<Self>::NAME,
            )),
            Err(err) => ctx.notify_error(err),
        }
    }

    fn on_start_error(
        error: start::StartError<Self>,
        ctx: &mut Context<Self>,
    ) {
        match error {
            start::StartError::UserDidNotConfirm => (),
            start::StartError::UserNotLoggedIn => {
                ctx.notify_error(
                    "You must be logged in to start collaborating. You can \
                     log in by executing ':Mad login'",
                );
            },
            other => ctx.notify_error(other),
        }
    }

    fn on_yank_error(error: yank::YankError<Self>, ctx: &mut Context<Self>) {
        ctx.notify_error(error);
    }

    fn project_filter(
        project_root: &<Self::Fs as fs::Fs>::Directory,
        ctx: &mut Context<Self>,
    ) -> Result<Self::ProjectFilter, Self::ProjectFilterError> {
        let create_res = ctx.with_editor(|nvim| {
            let spawner = nvim.executor().background_spawner();
            gitignore::GitIgnore::new(project_root.path(), spawner)
        });

        match create_res {
            Ok(gitignore) => Ok(Some(gitignore)),

            Err(err) => match &err {
                // If 'git' is not in $PATH that likely means the project
                // is not inside a Git repository.
                gitignore::CreateError::GitNotInPath
                | gitignore::CreateError::PathNotInGitRepository => Ok(None),

                gitignore::CreateError::CommandFailed(_)
                | gitignore::CreateError::InvalidPath => Err(err),
            },
        }
    }

    fn remove_peer_selection(
        _selection: Self::PeerSelection,
        _ctx: &mut Context<Self>,
    ) {
        // Dropping the selection will automatically remove the highlight, so
        // we don't have to do anything here.
    }

    fn remove_peer_tooltip(tooltip: Self::PeerTooltip, _: &mut Context<Self>) {
        tooltip.remove();
    }

    async fn select_session<'pairs>(
        sessions: &'pairs [(AbsPathBuf, SessionId)],
        action: ActionForSelectedSession,
        ctx: &mut Context<Self>,
    ) -> Option<&'pairs (AbsPathBuf, SessionId)> {
        let select = get_lua_value::<Function>(&["vim", "ui", "select"])?;

        let home_dir = Self::home_dir(ctx).await.ok();

        let items = {
            let t = mlua::lua().create_table().ok()?;
            for (idx, (path, _)) in sessions.iter().enumerate() {
                let path = TildePath { path, home_dir: home_dir.as_deref() };
                t.raw_set(idx, path.to_string()).ok()?;
            }
            t
        };

        let prompt = match action {
            ActionForSelectedSession::CopySessionId => {
                "Choose which session to yank the ID of: "
            },
            ActionForSelectedSession::Leave => {
                "Choose which session to leave: "
            },
        };

        let opts = {
            let t = mlua::lua().create_table().ok()?;
            t.raw_set("prompt", prompt).ok()?;
            t
        };

        let (idx_tx, idx_rx) = flume::bounded(1);

        let on_choice = mlua::lua()
            .create_function(
                move |_, (_, lua_idx): (mlua::Value, Option<u8>)| {
                    let idx = lua_idx.map(|idx| idx - 1);
                    let _ = idx_tx.send(idx);
                    Ok(())
                },
            )
            .ok()?;

        select.call::<()>((items, opts, on_choice)).ok()?;

        idx_rx
            .recv_async()
            .await
            .ok()?
            .and_then(|idx| sessions.get(idx as usize))
    }

    fn should_remote_save_cause_local_save(buf: &Self::Buffer<'_>) -> bool {
        !buf.is_focused()
    }
}

#[track_caller]
fn get_lua_value<T: mlua::FromLua>(namespace: &[&str]) -> Option<T> {
    assert!(!namespace.is_empty());
    let lua = mlua::lua();
    let mut table = lua.globals();
    let mut keys = namespace.iter();
    loop {
        let key = keys.next().expect("not done");
        if keys.as_slice().is_empty() {
            return table.get::<T>(*key).ok();
        } else {
            table = table.get::<Table>(*key).ok()?;
        }
    }
}

impl fmt::Display for TildePath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(home_dir) = self.home_dir else {
            return fmt::Display::fmt(&self.path, f);
        };

        if self.path.starts_with(home_dir) && self.path != home_dir {
            write!(f, "~{}", &self.path[home_dir.len()..])
        } else {
            fmt::Display::fmt(&self.path, f)
        }
    }
}
