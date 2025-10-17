use core::fmt;
use core::ops::Range;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::{env, io};

use abs_path::{AbsPath, AbsPathBuf, AbsPathFromPathError, node};
use async_net::TcpStream;
use collab_types::Peer;
use compact_str::format_compact;
use editor::context::Borrowed;
use editor::module::{Action, Module};
use editor::{ByteOffset, Context, Editor};
use executor::Executor;
use fs::Directory;
use futures_rustls::client::TlsStream;
use futures_rustls::{TlsConnector, rustls};
use futures_util::future::Either;
use mlua::{Function, Table};
use neovim::buffer::{BufferExt, BufferId};
use neovim::notify::{self, NotifyContextExt};
use neovim::{Neovim, mlua, oxi};
use nomad_collab_params::ulid;

use crate::editors::neovim::{
    NeovimPeerCursor,
    NeovimPeerHandle,
    NeovimPeerSelection,
    NeovimProgressReporter,
    PeerCursorHighlightGroup,
    PeerHandleHighlightGroup,
    PeerHighlightGroup,
    PeerSelectionHighlightGroup,
};
use crate::editors::{ActionForSelectedSession, CollabEditor};
use crate::session::{SessionError, SessionInfos};
use crate::tcp_stream_ext::TcpStreamExt;
use crate::{Collab, config, leave, yank};

pub type SessionId = ulid::Ulid;

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

/// An [`AbsPath`] wrapper whose `Display` impl replaces the path's home
/// directory with `~`.
struct TildePath<'a> {
    path: &'a AbsPath,
    home_dir: Option<&'a AbsPath>,
}

impl CollabEditor for Neovim {
    type Io = Either<TlsStream<TcpStream>, TcpStream>;
    type PeerSelection = NeovimPeerSelection;
    type PeerTooltip = (NeovimPeerCursor, NeovimPeerHandle);
    type ProgressReporter = NeovimProgressReporter;
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

        let should_start = match choice {
            0 | 2 => false,
            1 => true,
            _ => unreachable!("only provided {} options", options.len()),
        };

        // Skip one tick of the event loop. This seems to mitigate a rendering
        // bug that causes the dreaded "Press ENTER" prompt to appear if some
        // text is emitted to the message area right after this function
        // completes. See [this] for an example.
        //
        // [this]: https://github.com/user-attachments/assets/7b61ec1d-736d-4fc9-bb5e-14bbec0d1d52
        neovim::utils::schedule(|| ()).await;

        should_start
    }

    async fn connect_to_server(
        server_addr: config::ServerAddress<'static>,
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
        let buffer = oxi::api::Buffer::from(buffer_id);

        let namespace_id = ctx.with_editor(|nvim| nvim.namespace_id());

        NeovimPeerSelection::create(
            remote_peer.id,
            buffer,
            selected_range,
            namespace_id,
        )
    }

    fn create_peer_tooltip(
        remote_peer: Peer,
        tooltip_offset: ByteOffset,
        buffer_id: Self::BufferId,
        ctx: &mut Context<Self>,
    ) -> Self::PeerTooltip {
        let buffer = oxi::api::Buffer::from(buffer_id);

        let namespace_id = ctx.with_editor(|nvim| nvim.namespace_id());

        let cursor = NeovimPeerCursor::create(
            remote_peer.id,
            buffer.clone(),
            tooltip_offset,
            namespace_id,
        );

        let handle = NeovimPeerHandle::create(
            remote_peer,
            buffer,
            tooltip_offset,
            namespace_id,
        );

        (cursor, handle)
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
        _: &mut Context<Self>,
    ) {
        selection.r#move(selected_range);
    }

    fn move_peer_tooltip(
        (cursor, handle): &mut Self::PeerTooltip,
        new_offset: ByteOffset,
        _: &mut Context<Self>,
    ) {
        cursor.r#move(new_offset);
        handle.r#move(new_offset);
    }

    fn on_init(_: &mut Context<Self, Borrowed>) {
        PeerCursorHighlightGroup::create_all();
        PeerHandleHighlightGroup::create_all();
        PeerSelectionHighlightGroup::create_all();
    }

    fn on_leave_error(error: leave::LeaveError, ctx: &mut Context<Self>) {
        ctx.notify_error(error.to_string());
    }

    fn on_peer_left(_: &Peer, _: &AbsPath, _: &mut Context<Self>) {}

    fn on_peer_joined(_: &Peer, _: &AbsPath, _: &mut Context<Self>) {}

    fn on_session_ended(_: &SessionInfos<Self>, _: &mut Context<Self>) {}

    fn on_session_error(error: SessionError<Self>, ctx: &mut Context<Self>) {
        ctx.notify_error(error.to_string());
    }

    fn on_session_left(_: &SessionInfos<Self>, _: &mut Context<Self>) {}

    async fn on_session_started(
        infos: &SessionInfos<Self>,
        ctx: &mut Context<Self>,
    ) {
        // Skip one tick of the event loop. This seems to mitigate a rendering
        // bug that can happen if some other text is already being shown in the
        // message area. See [without] vs [with] for an example.
        //
        // [without]: https://github.com/user-attachments/assets/ccca9f36-21fd-46a1-851d-98b321880c54
        // [with]: https://github.com/user-attachments/assets/031d24e9-e030-4611-872c-1b51d3076e23
        neovim::utils::schedule(|| ()).await;

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
            Ok(()) => {
                let mut chunks = notify::Chunks::default();

                chunks
                    .push(
                        "Session ID copied to clipboard. You can also yank \
                         it later by executing:",
                    )
                    .push_newline()
                    .push_highlighted(":", "Comment")
                    .push_highlighted(
                        format_compact!(
                            "Mad {} {}",
                            Collab::<Self>::NAME,
                            yank::Yank::<Self>::NAME
                        ),
                        "Title",
                    );

                ctx.notify_info(chunks);
            },
            Err(err) => ctx.notify_error(err.to_string()),
        }
    }

    fn on_yank_error(error: yank::YankError<Self>, ctx: &mut Context<Self>) {
        ctx.notify_error(error.to_string());
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
        selection: Self::PeerSelection,
        _ctx: &mut Context<Self>,
    ) {
        selection.remove();
    }

    fn remove_peer_tooltip(
        (cursor, handle): Self::PeerTooltip,
        _: &mut Context<Self>,
    ) {
        cursor.remove();
        handle.remove();
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
