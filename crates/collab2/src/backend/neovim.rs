use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::ffi::OsString;
use std::path::PathBuf;
use std::{env, io};

use async_net::TcpStream;
use collab_server::message::Message;
use collab_server::nomad::{NomadConfig, NomadSessionId};
use collab_server::{SessionIntent, client};
use futures_util::io::{ReadHalf, WriteHalf};
use futures_util::{AsyncReadExt, Sink, Stream};
use mlua::{Function, Table};
use nvimx2::fs::{self, AbsPath, FsNodeName};
use nvimx2::neovim::{Neovim, NeovimBuffer, mlua, oxi};
use smol_str::ToSmolStr;

use crate::backend::*;

#[derive(Debug)]
pub struct NeovimCopySessionIdError {
    inner: clipboard::ClipboardError,
    session_id: NomadSessionId,
}

pin_project_lite::pin_project! {
    pub struct NeovimServerTx {
        #[pin]
        inner: client::ClientTx<WriteHalf<TcpStream>>,
    }
}

pin_project_lite::pin_project! {
    pub struct NeovimServerRx {
        #[pin]
        inner: client::ClientRx<ReadHalf<TcpStream>>,
    }
}

#[derive(Debug)]
pub enum NeovimDataDirError {
    Home(NeovimHomeDirError),
    XdgDataHomeNotAbsolute(String),
    XdgDataHomeNotUtf8(OsString),
}

#[derive(Debug)]
pub enum NeovimHomeDirError {
    CouldntFindHome,
    HomeDirNotAbsolute(PathBuf),
    HomeDirNotUtf8(PathBuf),
}

#[derive(Debug)]
pub struct NeovimServerTxError {
    inner: io::Error,
}

#[derive(Debug)]
pub struct NeovimServerRxError {
    inner: client::ClientRxError,
}

#[derive(Debug)]
pub struct NeovimSearchProjectRootError {
    inner: default_search_project_root::Error<Neovim>,
}

#[derive(Debug)]
pub enum NeovimNewSessionError {
    Knock(client::KnockError<NomadConfig>),
    TcpConnect(io::Error),
}

/// An [`AbsPath`] wrapper whose `Display` impl replaces the path's home
/// directory with `~`.
struct TildePath<'a> {
    path: &'a AbsPath,
    home_dir: Option<&'a AbsPath>,
}

impl CollabBackend for Neovim {
    type ServerRx = NeovimServerRx;
    type ServerTx = NeovimServerTx;
    type SessionId = NomadSessionId;

    type CopySessionIdError = NeovimCopySessionIdError;
    type DefaultDirForRemoteProjectsError = NeovimDataDirError;
    type HomeDirError = NeovimHomeDirError;
    type JoinSessionError = NeovimNewSessionError;
    type LspRootError = String;
    type SearchProjectRootError = NeovimSearchProjectRootError;
    type ServerRxError = NeovimServerRxError;
    type ServerTxError = NeovimServerTxError;
    type StartSessionError = NeovimNewSessionError;

    async fn confirm_start(
        project_root: &fs::AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
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

    async fn copy_session_id(
        session_id: Self::SessionId,
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<(), Self::CopySessionIdError> {
        clipboard::set(session_id)
            .map_err(|inner| NeovimCopySessionIdError { inner, session_id })
    }

    async fn default_dir_for_remote_projects(
        ctx: &mut AsyncCtx<'_, Self>,
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
                .join(<&FsNodeName>::try_from(".local").expect("valid"))
                .join(<&FsNodeName>::try_from("share").expect("valid")),
            Err(env::VarError::NotUnicode(xdg_data_home)) => {
                return Err(NeovimDataDirError::XdgDataHomeNotUtf8(
                    xdg_data_home,
                ));
            },
        };

        Ok(data_dir
            .join(<&FsNodeName>::try_from("nomad").expect("valid"))
            .join(<&FsNodeName>::try_from("remote-projects").expect("valid")))
    }

    async fn home_dir(
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<AbsPathBuf, Self::HomeDirError> {
        match home::home_dir() {
            Some(home_dir) if !home_dir.as_os_str().is_empty() => {
                home_dir.as_path().try_into().map_err(|err| match err {
                    fs::AbsPathFromPathError::NotAbsolute => {
                        NeovimHomeDirError::HomeDirNotAbsolute(home_dir)
                    },
                    fs::AbsPathFromPathError::NotUtf8 => {
                        NeovimHomeDirError::HomeDirNotUtf8(home_dir)
                    },
                })
            },
            _ => Err(NeovimHomeDirError::CouldntFindHome),
        }
    }

    async fn join_session(
        args: JoinArgs<'_, Self>,
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<SessionInfos<Self>, Self::JoinSessionError> {
        let (reader, writer) = TcpStream::connect(&**args.server_address)
            .await
            .map_err(NeovimNewSessionError::TcpConnect)?
            .split();

        let knock = collab_server::Knock::<NomadConfig> {
            auth_infos: args.auth_infos.clone().into(),
            session_intent: SessionIntent::JoinExisting(args.session_id),
        };

        let github_handle = knock.auth_infos.github_handle.clone();

        let welcome = client::Knocker::new(reader, writer)
            .knock(knock)
            .await
            .map_err(NeovimNewSessionError::Knock)?;

        Ok(SessionInfos {
            host_id: welcome.host_id,
            local_peer: Peer::new(welcome.peer_id, github_handle),
            project_name: welcome.project_name,
            remote_peers: welcome.other_peers,
            server_rx: NeovimServerRx { inner: welcome.rx },
            server_tx: NeovimServerTx { inner: welcome.tx },
            session_id: welcome.session_id,
        })
    }

    fn lsp_root(
        buffer: NeovimBuffer,
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError> {
        /// Returns the root directory of the first language server
        /// attached to the given buffer, if any.
        fn inner(buffer: NeovimBuffer) -> Option<String> {
            let lua = mlua::lua();

            let opts = lua.create_table().ok()?;
            opts.raw_set("bufnr", buffer).ok()?;

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

        inner(buffer)
            .map(|root_dir| root_dir.parse().map_err(|_| root_dir))
            .transpose()
    }

    async fn search_project_root(
        buffer: NeovimBuffer,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<AbsPathBuf, Self::SearchProjectRootError> {
        default_search_project_root::search(buffer, ctx)
            .await
            .map_err(|inner| NeovimSearchProjectRootError { inner })
    }

    async fn select_session<'pairs>(
        sessions: &'pairs [(fs::AbsPathBuf, Self::SessionId)],
        action: ActionForSelectedSession,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Option<&'pairs (fs::AbsPathBuf, Self::SessionId)> {
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

    async fn start_session(
        args: StartArgs<'_>,
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<SessionInfos<Self>, Self::StartSessionError> {
        let (reader, writer) = TcpStream::connect(&**args.server_address)
            .await
            .map_err(NeovimNewSessionError::TcpConnect)?
            .split();

        let knock = collab_server::Knock::<NomadConfig> {
            auth_infos: args.auth_infos.clone().into(),
            session_intent: SessionIntent::StartNew(
                args.project_name.to_owned(),
            ),
        };

        let github_handle = knock.auth_infos.github_handle.clone();

        let welcome = client::Knocker::new(reader, writer)
            .knock(knock)
            .await
            .map_err(NeovimNewSessionError::Knock)?;

        Ok(SessionInfos {
            host_id: welcome.host_id,
            local_peer: Peer::new(welcome.peer_id, github_handle),
            project_name: welcome.project_name,
            remote_peers: welcome.other_peers,
            server_rx: NeovimServerRx { inner: welcome.rx },
            server_tx: NeovimServerTx { inner: welcome.tx },
            session_id: welcome.session_id,
        })
    }
}

impl Sink<Message> for NeovimServerTx {
    type Error = NeovimServerTxError;

    fn poll_ready(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<Message>::poll_ready(self.project().inner, ctx)
            .map_err(|err| NeovimServerTxError { inner: err })
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: Message,
    ) -> Result<(), Self::Error> {
        Sink::<Message>::start_send(self.project().inner, item)
            .map_err(|err| NeovimServerTxError { inner: err })
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<Message>::poll_flush(self.project().inner, ctx)
            .map_err(|err| NeovimServerTxError { inner: err })
    }

    fn poll_close(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<Message>::poll_close(self.project().inner, ctx)
            .map_err(|err| NeovimServerTxError { inner: err })
    }
}

impl Stream for NeovimServerRx {
    type Item = Result<Message, NeovimServerRxError>;

    fn poll_next(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        self.project()
            .inner
            .poll_next(ctx)
            .map_err(|err| NeovimServerRxError { inner: err })
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

impl notify::Error for NeovimCopySessionIdError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        msg.push_str("couldn't copy ")
            .push_info(self.session_id.to_smolstr())
            .push_str(" to clipboard: ")
            .push_str(self.inner.to_smolstr());
        (notify::Level::Error, msg)
    }
}

impl notify::Error for NeovimSearchProjectRootError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        use default_search_project_root::Error::*;

        let mut msg = notify::Message::new();

        match &self.inner {
            BufNameNotAbsolutePath(buf_name) => {
                if buf_name.is_empty() {
                    msg.push_str("the current buffer's name is empty");
                } else {
                    msg.push_str("buffer name ")
                        .push_invalid(buf_name)
                        .push_str(" is not an absolute path");
                }
            },
            Lsp(lsp_root) => {
                msg.push_str("LSP root at ")
                    .push_invalid(lsp_root)
                    .push_str(" is not an absolute path");
            },
            FindRoot(err) => return err.to_message(),
            HomeDir(err) => return err.to_message(),
            InvalidBufId(buf) => {
                msg.push_str("there's no buffer whose handle is ")
                    .push_invalid(buf.handle().to_smolstr());
            },
            CouldntFindRoot(buffer_path) => {
                msg.push_str("couldn't find project root for buffer at ")
                    .push_info(buffer_path.to_smolstr())
                    .push_str(", please pass one explicitly");
            },
        }

        (notify::Level::Error, msg)
    }
}

impl notify::Error for NeovimNewSessionError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        match self {
            Self::Knock(err) => match err {
                client::KnockError::SendKnock(err) => {
                    msg.push_str("couldn't send start request to server: ")
                        .push_str(err.to_smolstr());
                },
                client::KnockError::RecvWelcome(err) => {
                    msg.push_str(
                        "couldn't receive start response from server: ",
                    )
                    .push_str(err.to_smolstr());
                },
                client::KnockError::Bouncer(err) => {
                    msg.push_str("authentication failed: ")
                        .push_str(err.to_smolstr());
                },
                client::KnockError::SessionEndedBeforeJoining => {
                    unreachable!();
                },
            },
            Self::TcpConnect(err) => {
                msg.push_str("couldn't connect to the server: ")
                    .push_str(err.to_smolstr());
            },
        }
        (notify::Level::Error, msg)
    }
}

impl notify::Error for NeovimDataDirError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();

        match self {
            Self::Home(err) => return err.to_message(),
            Self::XdgDataHomeNotAbsolute(data_dir) => {
                msg.push_str("found data directory at ")
                    .push_invalid(data_dir)
                    .push_str(", but it's not an absolute path");
            },
            Self::XdgDataHomeNotUtf8(data_dir) => {
                msg.push_str("found data directory at ")
                    .push_invalid(data_dir.display().to_smolstr())
                    .push_str(", but it's not a valid UTF-8 string");
            },
        }

        (notify::Level::Error, msg)
    }
}

impl notify::Error for NeovimHomeDirError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();

        match self {
            Self::CouldntFindHome => {
                msg.push_str("couldn't find home directory");
            },
            Self::HomeDirNotAbsolute(home_dir) => {
                msg.push_str("found home directory at ")
                    .push_invalid(home_dir.display().to_smolstr())
                    .push_str(", but it's not an absolute path");
            },
            Self::HomeDirNotUtf8(home_dir) => {
                msg.push_str("found home directory at ")
                    .push_invalid(home_dir.display().to_smolstr())
                    .push_str(", but it's not a valid UTF-8 string");
            },
        }

        (notify::Level::Error, msg)
    }
}

impl notify::Error for NeovimServerTxError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        msg.push_str("couldn't send message to the server: ")
            .push_str(self.inner.to_string());
        (notify::Level::Error, msg)
    }
}

impl notify::Error for NeovimServerRxError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();
        msg.push_str("couldn't receive message from the server: ")
            .push_str(self.inner.to_string());
        (notify::Level::Error, msg)
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
