use nvim::api::types;

use crate::{ChunkExt, WarningMsg};

/// TODO: docs
pub struct CommandArgs {
    /// TODO: docs
    args: Vec<String>,

    /// TODO: docs
    consumed: usize,
}

impl From<types::CommandArgs> for CommandArgs {
    #[inline]
    fn from(args: types::CommandArgs) -> Self {
        Self::new(args.fargs)
    }
}

impl CommandArgs {
    /// TODO: docs
    #[inline]
    pub fn as_slice(&self) -> &[String] {
        &self.args[self.consumed..]
    }

    /// TODO: docs
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// TODO: docs
    #[inline]
    pub fn len(&self) -> usize {
        self.args.len() - self.consumed
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn new(args: Vec<String>) -> Self {
        Self { args, consumed: 0 }
    }

    /// TODO: docs
    #[inline]
    pub(crate) fn split_first(&mut self) -> Option<&str> {
        self.args
            .get(self.consumed)
            .map(String::as_str)
            .inspect(|_| self.consumed += 1)
    }
}

impl IntoIterator for CommandArgs {
    type Item = String;
    type IntoIter = std::iter::Skip<std::vec::IntoIter<Self::Item>>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        self.args.into_iter().skip(self.consumed)
    }
}

impl TryFrom<CommandArgs> for () {
    type Error = CommandArgsWrongNumError;

    #[inline]
    fn try_from(args: CommandArgs) -> Result<Self, Self::Error> {
        if args.is_empty() {
            Ok(())
        } else {
            Err(CommandArgsWrongNumError::new(0, args))
        }
    }
}

impl TryFrom<CommandArgs> for String {
    type Error = CommandArgsWrongNumError;

    #[inline]
    fn try_from(args: CommandArgs) -> Result<Self, Self::Error> {
        if args.len() == 1 {
            Ok(args.into_iter().next().expect("just checked len"))
        } else {
            Err(CommandArgsWrongNumError::new(1, args))
        }
    }
}

/// An error returned when a command's arguments are not the expected number.
pub struct CommandArgsWrongNumError {
    expected: usize,
    got: CommandArgs,
}

impl CommandArgsWrongNumError {
    /// Creates a new [`CommandArgsWrongNumError`].
    #[inline]
    pub fn new(num_expected: usize, got: CommandArgs) -> Self {
        Self { expected: num_expected, got }
    }
}

impl From<CommandArgsWrongNumError> for WarningMsg {
    #[inline]
    fn from(err: CommandArgsWrongNumError) -> WarningMsg {
        let mut msg = WarningMsg::new();

        msg.add(format!(
            "expected {num} argument{plural}, but got ",
            num = err.expected,
            plural = (err.expected != 1).then_some("s").unwrap_or_default()
        ));

        let num_args = err.got.len();

        if num_args == 0 {
            msg.add("none");
            return msg;
        }

        for (idx, arg) in err.got.into_iter().enumerate() {
            msg.add(arg.highlight());

            let is_last = idx + 1 == num_args;

            if is_last {
                break;
            }

            let is_second_to_last = idx + 2 == num_args;

            if is_second_to_last {
                msg.add(" and ");
            } else {
                msg.add(", ");
            }
        }

        msg
    }
}
