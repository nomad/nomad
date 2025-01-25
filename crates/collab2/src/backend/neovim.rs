use core::fmt;
use core::pin::Pin;
use core::task::{Context, Poll};
use std::io;
use std::path::PathBuf;

use async_net::TcpStream;
use collab_server::configs::nomad;
use collab_server::{SessionIntent, client, message};
use futures_util::io::{ReadHalf, WriteHalf};
use futures_util::{AsyncReadExt, Sink, Stream};
use mlua::{Function, Table};
use nvimx2::fs::{self, AbsPath};
use nvimx2::neovim::{Neovim, NeovimBuffer, NeovimFs, mlua, oxi};
use smol_str::ToSmolStr;

use crate::backend::*;

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

pub struct NeovimServerTxError {
    inner: io::Error,
}

pub struct NeovimServerRxError {
    inner: client::ClientRxError,
}

pub struct NeovimSearchProjectRootError {
    inner: default_search_project_root::Error<Neovim>,
}

pub enum NeovimStartSessionError {
    Knock(client::KnockError<nomad::NomadAuthenticator>),
    TcpConnect(io::Error),
}

pub enum NeovimHomeDirError {
    CouldntFindHome,
    InvalidHomeDir(PathBuf, fs::AbsPathFromPathError),
}

/// An [`AbsPath`] wrapper whose `Display` impl replaces the path's home
/// directory with `~`.
struct TildePath<'a> {
    path: &'a AbsPath,
    home_dir: Option<&'a AbsPath>,
}

impl CollabBackend for Neovim {
    type SearchProjectRootError = NeovimSearchProjectRootError;
    type ServerTx = NeovimServerTx;
    type ServerRx = NeovimServerRx;
    type ServerTxError = NeovimServerTxError;
    type ServerRxError = NeovimServerRxError;
    type StartSessionError = NeovimStartSessionError;

    async fn confirm_start(
        project_root: &fs::AbsPath,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> bool {
        let prompt = format!(
            "Start collaborating on the project at \"{}\"?",
            TildePath {
                path: project_root,
                home_dir: ctx.fs().home_dir().await.ok().as_deref(),
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

    async fn search_project_root(
        buffer: NeovimBuffer,
        ctx: &mut AsyncCtx<'_, Self>,
    ) -> Result<AbsPathBuf, Self::SearchProjectRootError> {
        default_search_project_root::search(buffer, ctx)
            .await
            .map_err(|inner| NeovimSearchProjectRootError { inner })
    }

    async fn start_session(
        args: StartArgs<'_>,
        _: &mut AsyncCtx<'_, Self>,
    ) -> Result<StartInfos<Self>, Self::StartSessionError> {
        let (reader, writer) = TcpStream::connect(&**args.server_address)
            .await
            .map_err(NeovimStartSessionError::TcpConnect)?
            .split();

        let knock = collab_server::Knock {
            auth_infos: args.auth_infos.clone().into(),
            session_intent: SessionIntent::StartNew,
        };

        let welcome =
            client::Knocker::<_, _, nomad::NomadConfig>::new(reader, writer)
                .knock(knock)
                .await
                .map_err(NeovimStartSessionError::Knock)?;

        Ok(StartInfos {
            peer_id: welcome.peer_id,
            server_tx: NeovimServerTx { inner: welcome.tx },
            server_rx: NeovimServerRx { inner: welcome.rx },
        })
    }
}

impl CollabBuffer<Neovim> for NeovimBuffer {
    type LspRootError = String;

    fn lsp_root(
        buffer: NeovimBuffer,
        _: &mut AsyncCtx<'_, Neovim>,
    ) -> Result<Option<AbsPathBuf>, Self::LspRootError> {
        /// Returns the root directory of the first language server
        /// attached to the given buffer, if any.
        fn inner(buffer: NeovimBuffer) -> Option<String> {
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

        inner(buffer)
            .map(|root_dir| root_dir.parse().map_err(|_| root_dir))
            .transpose()
    }
}

impl CollabFs for NeovimFs {
    type HomeDirError = NeovimHomeDirError;

    async fn home_dir(&mut self) -> Result<AbsPathBuf, Self::HomeDirError> {
        match home::home_dir() {
            Some(home_dir) if !home_dir.as_os_str().is_empty() => {
                home_dir.as_path().try_into().map_err(|err| {
                    NeovimHomeDirError::InvalidHomeDir(home_dir, err)
                })
            },
            _ => Err(NeovimHomeDirError::CouldntFindHome),
        }
    }
}

impl Sink<message::Message> for NeovimServerTx {
    type Error = NeovimServerTxError;

    fn poll_ready(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<message::Message>::poll_ready(self.project().inner, ctx)
            .map_err(|err| NeovimServerTxError { inner: err })
    }

    fn start_send(
        self: Pin<&mut Self>,
        item: message::Message,
    ) -> Result<(), Self::Error> {
        Sink::<message::Message>::start_send(self.project().inner, item)
            .map_err(|err| NeovimServerTxError { inner: err })
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<message::Message>::poll_flush(self.project().inner, ctx)
            .map_err(|err| NeovimServerTxError { inner: err })
    }

    fn poll_close(
        self: Pin<&mut Self>,
        ctx: &mut Context<'_>,
    ) -> Poll<Result<(), Self::Error>> {
        Sink::<message::Message>::poll_close(self.project().inner, ctx)
            .map_err(|err| NeovimServerTxError { inner: err })
    }
}

impl Stream for NeovimServerRx {
    type Item = Result<message::Message, NeovimServerRxError>;

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

impl notify::Error for NeovimStartSessionError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}

impl notify::Error for NeovimHomeDirError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        let mut msg = notify::Message::new();

        match self {
            NeovimHomeDirError::CouldntFindHome => {
                msg.push_str("couldn't find home directory");
            },
            NeovimHomeDirError::InvalidHomeDir(
                home_dir,
                fs::AbsPathFromPathError::NotAbsolute,
            ) => {
                msg.push_str("found home directory at ")
                    .push_str(home_dir.display().to_smolstr())
                    .push_str(", but it's not an absolute path");
            },
            NeovimHomeDirError::InvalidHomeDir(
                home_dir,
                fs::AbsPathFromPathError::NotUtf8,
            ) => {
                msg.push_str("found home directory at ")
                    .push_str(home_dir.display().to_smolstr())
                    .push_str(", but it's not a valid UTF-8 string");
            },
        }

        (notify::Level::Error, msg)
    }
}

impl notify::Error for NeovimServerTxError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}

impl notify::Error for NeovimServerRxError {
    fn to_message(&self) -> (notify::Level, notify::Message) {
        todo!();
    }
}

impl fmt::Display for TildePath<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(home_dir) = self.home_dir else {
            return self.path.fmt(f);
        };

        if self.path.starts_with(home_dir) && self.path != home_dir {
            write!(f, "~{}", &self.path[home_dir.len()..])
        } else {
            self.path.fmt(f)
        }
    }
}
