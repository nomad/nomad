use core::cell::Cell;
use core::ops::Range;
use core::{any, fmt};
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::{env, io};

use abs_path::{AbsPath, AbsPathBuf, AbsPathFromPathError, node};
use async_net::TcpStream;
use collab_types::{Peer, PeerId};
use editor::context::Borrowed;
use editor::module::{Action, Module};
use editor::{ByteOffset, Context, Editor};
use executor::Executor;
use fs::Directory;
use futures_rustls::client::TlsStream;
use futures_rustls::{TlsConnector, rustls};
use futures_util::future::Either;
use mlua::{Function, Table};
use neovim::buffer::{BufferExt, BufferId, HighlightRangeHandle, Point};
use neovim::notify::ContextExt;
use neovim::{Neovim, mlua, oxi};
use nomad_collab_params::ulid;

use crate::editors::{ActionForSelectedSession, CollabEditor};
use crate::session::{SessionError, SessionInfos};
use crate::tcp_stream_ext::TcpStreamExt;
use crate::{Collab, config, join, leave, start, yank};

pub type SessionId = ulid::Ulid;

pub struct NeovimPeerSelection {
    selection_highlight_handle: HighlightRangeHandle,
}

/// Holds the state needed to display a remote peer's cursor in a buffer.
pub struct PeerCursor {
    /// The buffer the cursor is in.
    buffer: oxi::api::Buffer,

    /// The extmark ID of the highlight representing the cursor's head.
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
pub enum NeovimConnectToServerError {
    #[display("couldn't establish TCP connection with server: {_0}")]
    ConnectTcp(io::Error),

    #[display("couldn't establish TLS connection with server: {_0}")]
    ConnectTls(io::Error),

    #[display("couldn't obtain TLS certificates from OS: {_0}")]
    Certificates(futures_rustls::rustls::Error),
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

/// The highlight group used to highlight a remote peer's cursor.
struct PeerCursorHighlightGroup;

/// The highlight group used to highlight a remote peer's selection.
struct PeerSelectionHighlightGroup;

/// An [`AbsPath`] wrapper whose `Display` impl replaces the path's home
/// directory with `~`.
struct TildePath<'a> {
    path: &'a AbsPath,
    home_dir: Option<&'a AbsPath>,
}

/// A trait implemented by types that represent highlight groups used to
/// highlight a piece of UI (like a cursor or selection) that belongs to a
/// remote peer.
trait RemotePeerHighlightGroup: WithGroupIds {
    /// The prefix of each highlight group name.
    const NAME_PREFIX: &'static str;

    #[track_caller]
    fn create_all() {
        debug_assert!(
            Self::with_group_ids(|ids| ids.iter().all(|id| id.get() == 0)),
            "{}::create_all() has already been called",
            any::type_name::<Self>()
        );

        Self::with_group_ids(|group_ids| {
            for (group_idx, group_id) in group_ids.iter().enumerate() {
                group_id.set(Self::create(group_idx.saturating_add(1)));
            }
        });
    }

    #[track_caller]
    fn new(peer_id: PeerId) -> impl oxi::api::SetExtmarkHlGroup {
        Self::with_group_ids(|group_ids| {
            let group_idx = peer_id.into_u64().saturating_sub(1) as usize
                % group_ids.len();

            let group_id = group_ids[group_idx].get();

            debug_assert!(
                group_id > 0,
                "{}::create_all() has not been called",
                any::type_name::<Self>()
            );

            i64::from(group_id)
        })
    }

    /// Returns the `opts` to pass to [`set_hl`](oxi::api::set_hl) when
    /// creating the highlight group.
    fn set_hl_opts() -> oxi::api::opts::SetHighlightOpts;

    #[doc(hidden)]
    fn create(suffix: usize) -> u32 {
        let name = Self::name(suffix);

        oxi::api::set_hl(0, name.as_ref(), &Self::set_hl_opts())
            .expect("couldn't create highlight group");

        oxi::api::get_hl_id_by_name(name.as_ref())
            .expect("couldn't get highlight group ID")
    }

    #[doc(hidden)]
    fn name(suffix: usize) -> impl AsRef<str> {
        compact_str::format_compact!("{}{}", Self::NAME_PREFIX, suffix)
    }
}

trait WithGroupIds {
    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R;
}

impl PeerCursor {
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
            .end_row(highlight_range.end.newline_offset)
            .end_col(highlight_range.end.byte_offset)
            .hl_group(PeerCursorHighlightGroup::new(peer.id))
            .build();

        let cursor_extmark_id = buffer
            .set_extmark(
                namespace_id,
                highlight_range.start.newline_offset,
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
        debug_assert!(cursor_offset <= buffer.num_bytes());

        let mut highlight_start = buffer.point_of_byte(cursor_offset);

        let is_cursor_at_eol = buffer
            .num_bytes_in_line_after(highlight_start.newline_offset)
            == highlight_start.byte_offset;

        if is_cursor_at_eol {
            // If the cursor is after the uneditable eol, set the start
            // position to the end of the previous line.
            if cursor_offset == buffer.num_bytes()
                && buffer.has_uneditable_eol()
            {
                let highlight_end = highlight_start;
                highlight_start.newline_offset -= 1;
                highlight_start.byte_offset = buffer
                    .num_bytes_in_line_after(highlight_start.newline_offset);
                return highlight_start..highlight_end;
            }
        }

        let highlight_end =
            // If the cursor is at the end of the line, we set the end of the
            // highlighted range to the start of the next line.
            //
            // Apparently this works even if the cursor is on the last line,
            // and nvim_buf_set_extmark won't complain about it.
            if is_cursor_at_eol {
                Point::new(highlight_start.newline_offset + 1, 0)
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
                    highlight_start.newline_offset,
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
            .end_row(highlight_range.end.newline_offset)
            .end_col(highlight_range.end.byte_offset)
            .hl_group(PeerCursorHighlightGroup::new(self.peer.id))
            .build();

        let new_extmark_id = self
            .buffer
            .set_extmark(
                self.namespace_id,
                highlight_range.start.newline_offset,
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
    type Io = Either<TlsStream<TcpStream>, TcpStream>;
    type PeerSelection = NeovimPeerSelection;
    type PeerTooltip = PeerCursor;
    type ProjectFilter = Option<gitignore::GitIgnore>;
    type ServerParams = nomad_collab_params::NomadParams;

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
        ctx: &mut Context<Self>,
    ) -> Result<Self::Io, Self::ConnectToServerError> {
        let tcp_stream =
            <TcpStream as TcpStreamExt>::connect(server_addr.clone(), ctx)
                .await
                .map_err(NeovimConnectToServerError::ConnectTcp)?;

        // If we're connecting to a loopback address we're probably testing
        // against a local server without TLS, so use plain TCP.
        if let config::Host::Ip(ip) = &server_addr.host
            && ip.is_loopback()
        {
            return Ok(Either::Right(tcp_stream));
        }

        let tls_connector = tls_connector(ctx)
            .await
            .map_err(NeovimConnectToServerError::Certificates)?;

        tls_connector
            .connect(server_addr.host.into(), tcp_stream)
            .await
            .map(Either::Left)
            .map_err(NeovimConnectToServerError::ConnectTls)
    }

    async fn copy_session_id(
        session_id: SessionId,
        _: &mut Context<Self>,
    ) -> Result<(), Self::CopySessionIdError> {
        clipboard::set(session_id)
            .map_err(|inner| NeovimCopySessionIdError { inner, session_id })
    }

    fn create_peer_selection(
        remote_peer: Peer,
        selected_range: Range<ByteOffset>,
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Self::PeerSelection {
        ctx.with_borrowed(|ctx| {
            let buffer = ctx.buffer(buffer_id).expect("invalid buffer ID");
            let _hl_group = PeerSelectionHighlightGroup::new(remote_peer.id);
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
        PeerCursor::create(remote_peer, buffer, tooltip_offset, namespace_id)
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

    fn move_peer_selection(
        selection: &mut Self::PeerSelection,
        selected_range: Range<ByteOffset>,
        ctx: &mut Context<Self>,
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

    fn on_init(_: &mut Context<Self, Borrowed>) {
        PeerCursorHighlightGroup::create_all();
        PeerSelectionHighlightGroup::create_all();
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

async fn tls_connector(
    ctx: &mut Context<impl Editor>,
) -> Result<&TlsConnector, rustls::Error> {
    static TLS_CONNECTOR: OnceLock<TlsConnector> = OnceLock::new();

    if let Some(connector) = TLS_CONNECTOR.get() {
        return Ok(connector);
    }

    // Getting the certificates from the OS blocks, so we do it in a
    // background thread.
    let client_config = ctx
        .spawn_background(async {
            use rustls_platform_verifier::ConfigVerifierExt;
            rustls::ClientConfig::with_platform_verifier()
        })
        .await?;

    Ok(TLS_CONNECTOR
        .get_or_init(|| TlsConnector::from(Arc::new(client_config))))
}

impl PeerCursorHighlightGroup {
    thread_local! {
        static GROUP_IDS: Cell<[u32; 16]> = const { Cell::new([0; _]) };
    }
}

impl PeerSelectionHighlightGroup {
    thread_local! {
        static GROUP_IDS: Cell<[u32; 16]> = const { Cell::new([0; _]) };
    }
}

impl WithGroupIds for PeerCursorHighlightGroup {
    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R {
        Self::GROUP_IDS.with(|ids| fun(ids.as_array_of_cells().as_slice()))
    }
}

impl WithGroupIds for PeerSelectionHighlightGroup {
    fn with_group_ids<R>(fun: impl FnOnce(&[Cell<u32>]) -> R) -> R {
        Self::GROUP_IDS.with(|ids| fun(ids.as_array_of_cells().as_slice()))
    }
}

impl RemotePeerHighlightGroup for PeerCursorHighlightGroup {
    const NAME_PREFIX: &str = "NomadCollabPeerCursor";

    fn set_hl_opts() -> oxi::api::opts::SetHighlightOpts {
        oxi::api::opts::SetHighlightOpts::builder().link("Cursor").build()
    }
}

impl RemotePeerHighlightGroup for PeerSelectionHighlightGroup {
    const NAME_PREFIX: &str = "NomadCollabPeerSelection";

    fn set_hl_opts() -> oxi::api::opts::SetHighlightOpts {
        oxi::api::opts::SetHighlightOpts::builder().link("Visual").build()
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
