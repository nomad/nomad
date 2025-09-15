use std::io;
use std::net::ToSocketAddrs;

use editor::{Context, Editor};

/// An extension trait for `async_net`'s `TcpStream` which allows it to connect
/// to any [`ToSocketAddrs`] (instead of their custom
/// [`AsyncToSocketAddrs`](async_net::AsyncToSocketAddrs), which is `Sealed`),
/// by performing the blocking DNS lookups in a background thread.
pub(crate) trait TcpStreamExt {
    fn connect<Addrs>(
        addrs: Addrs,
        ctx: &mut Context<impl Editor>,
    ) -> impl Future<Output = io::Result<async_net::TcpStream>>
    where
        Addrs: ToSocketAddrs + Send + 'static,
        Addrs::Iter: Send;
}

impl TcpStreamExt for async_net::TcpStream {
    async fn connect<Addrs>(
        addrs: Addrs,
        ctx: &mut Context<impl Editor>,
    ) -> io::Result<Self>
    where
        Addrs: ToSocketAddrs + Send + 'static,
        Addrs::Iter: Send,
    {
        let mut last_err = None;

        let socket_addrs = ctx
            .spawn_background(async move { addrs.to_socket_addrs() })
            .await?;

        for addr in socket_addrs {
            match async_io::Async::<std::net::TcpStream>::connect(addr).await {
                Ok(stream) => return Ok(stream.into()),
                Err(err) => last_err = Some(err),
            }
        }

        Err(last_err.unwrap_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "could not connect to any of the addresses",
            )
        }))
    }
}
