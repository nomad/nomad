use core::num::ParseIntError;
use core::{fmt, str};

use nomad::neovim::{CommandArgs, DiagnosticMessage};

#[derive(Copy, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct SessionId(pub(crate) collab_server::SessionId);

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0.into_u64())
    }
}

impl str::FromStr for SessionId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(s, 16).map(collab_server::SessionId::new).map(Self)
    }
}

impl TryFrom<&mut CommandArgs> for SessionId {
    type Error = DiagnosticMessage;

    fn try_from(args: &mut CommandArgs) -> Result<Self, Self::Error> {
        let [id] = args.as_slice() else {
            todo!();
        };
        id.parse::<Self>().map_err(|err| {
            todo!();
        })
    }
}
