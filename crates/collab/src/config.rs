//! TODO: docs.

use core::{fmt, iter, net, str};
use std::borrow::Cow;
use std::net::ToSocketAddrs;
use std::{io, vec};

use abs_path::AbsPathBuf;
use either::Either;
use rustls_pki_types::DnsName;
use serde::de::{Deserialize, Deserializer};

const DEFAULT_DOMAIN: &str = "collab.nomad.foo";
const DEFAULT_PORT: u16 = 3000;

/// TODO: docs.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The address of the server to connect to when starting or joining an
    /// editing session.
    pub(crate) server_address: ServerAddress,

    /// TODO: docs.
    pub(crate) store_remote_projects_under: Option<AbsPathBuf>,
}

/// TODO: docs.
#[derive(Clone)]
pub struct ServerAddress {
    kind: AddressKind,
    port: u16,
}

#[derive(Clone)]
enum AddressKind {
    Localhost,
    Ip(net::IpAddr),
    Domain(DnsName<'static>),
}

impl Default for ServerAddress {
    fn default() -> Self {
        let Ok(dns_name) = DnsName::try_from(DEFAULT_DOMAIN) else {
            unreachable!("{DEFAULT_DOMAIN:?} is a valid DNS name")
        };
        Self { kind: AddressKind::Domain(dns_name), port: DEFAULT_PORT }
    }
}

impl fmt::Debug for ServerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for ServerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind, self.port)
    }
}

impl str::FromStr for ServerAddress {
    type Err = core::convert::Infallible;

    fn from_str(_s: &str) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl<'de> Deserialize<'de> for ServerAddress {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Cow::<str>::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

impl ToSocketAddrs for ServerAddress {
    type Iter =
        Either<iter::Once<net::SocketAddr>, vec::IntoIter<net::SocketAddr>>;

    fn to_socket_addrs(&self) -> io::Result<Self::Iter> {
        let ip_addr = match &self.kind {
            AddressKind::Localhost => {
                net::IpAddr::V4(net::Ipv4Addr::LOCALHOST)
            },
            AddressKind::Ip(ip_addr) => *ip_addr,
            AddressKind::Domain(dns_name) => {
                return (dns_name.as_ref(), self.port)
                    .to_socket_addrs()
                    .map(Either::Right);
            },
        };
        Ok(Either::Left(iter::once(net::SocketAddr::new(ip_addr, self.port))))
    }
}

impl fmt::Display for AddressKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AddressKind::Localhost => f.write_str("localhost"),
            AddressKind::Ip(ip) => write!(f, "{ip}"),
            AddressKind::Domain(domain) => f.write_str(domain.as_ref()),
        }
    }
}
