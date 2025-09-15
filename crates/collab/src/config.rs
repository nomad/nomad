//! TODO: docs.

use core::{fmt, iter, net, num, str};
use std::borrow::Cow;
use std::net::ToSocketAddrs;
use std::{io, vec};

use abs_path::AbsPathBuf;
use either::Either;
use rustls_pki_types::{DnsName, ServerName};
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
    pub(crate) host: Host,
    pub(crate) port: u16,
}

/// The type of error that can occur when parsing a `ServerAddress` from a
/// string.
#[derive(Debug, derive_more::Display, cauchy::Error)]
pub enum ServerAddressParseError {
    /// The input string is not formatted as `{host}:{port}`.
    #[display("expected a string formatted as {{host}}:{{port}}")]
    InvalidFormat,

    /// The host is not a valid domain name, IP address, or `localhost`.
    #[display("expected host to be a domain name, IP address, or 'localhost'")]
    InvalidHost,

    /// The port is not a valid number.
    #[display("invalid port: {_0}")]
    InvalidPort(num::ParseIntError),
}

#[derive(Clone)]
pub(crate) enum Host {
    Ip(net::IpAddr),
    Domain(DnsName<'static>),
}

impl Default for ServerAddress {
    fn default() -> Self {
        let Ok(dns_name) = DnsName::try_from(DEFAULT_DOMAIN) else {
            unreachable!("{DEFAULT_DOMAIN:?} is a valid DNS name")
        };
        Self { host: Host::Domain(dns_name), port: DEFAULT_PORT }
    }
}

impl fmt::Debug for ServerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for ServerAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.host, self.port)
    }
}

impl str::FromStr for ServerAddress {
    type Err = ServerAddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (host, port) = s
            .rsplit_once(':')
            .ok_or(ServerAddressParseError::InvalidFormat)?;

        let port =
            port.parse().map_err(ServerAddressParseError::InvalidPort)?;

        let host = if host == "localhost" {
            Host::Ip(net::IpAddr::V4(net::Ipv4Addr::LOCALHOST))
        } else if let Ok(ip_addr) = host.parse() {
            Host::Ip(ip_addr)
        } else if let Ok(dns_name) = DnsName::try_from(host) {
            Host::Domain(dns_name.to_owned())
        } else {
            return Err(ServerAddressParseError::InvalidHost);
        };

        Ok(Self { host, port })
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
        match &self.host {
            Host::Ip(ip_addr) => Ok(Either::Left(iter::once(
                net::SocketAddr::new(*ip_addr, self.port),
            ))),
            Host::Domain(dns_name) => (dns_name.as_ref(), self.port)
                .to_socket_addrs()
                .map(Either::Right),
        }
    }
}

impl fmt::Display for Host {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ip(ip) => write!(f, "{ip}"),
            Self::Domain(domain) => f.write_str(domain.as_ref()),
        }
    }
}

impl From<Host> for ServerName<'static> {
    fn from(host: Host) -> Self {
        match host {
            Host::Ip(ip_addr) => Self::IpAddress(ip_addr.into()),
            Host::Domain(dns_name) => Self::DnsName(dns_name),
        }
    }
}
