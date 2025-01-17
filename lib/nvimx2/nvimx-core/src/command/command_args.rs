use core::fmt;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;

use smol_str::ToSmolStr;

use crate::{ByteOffset, notify};

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct CommandArgs<'a> {
    inner: &'a str,
}

/// A group of adjacent non-whitespace characters in a [`CommandArgs`].
#[derive(Copy, Clone)]
pub struct CommandArg<'a> {
    inner: &'a str,
    idx: CommandArgIdx,
}

/// The index of a [`CommandArg`] in a [`CommandArgs`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandArgIdx {
    pub(crate) start: ByteOffset,
    pub(crate) end: ByteOffset,
}

/// An iterator over the [`CommandArg`]s of a [`CommandArgs`].
#[derive(Clone)]
pub struct CommandArgsIter<'a> {
    inner: &'a str,
    last_idx_end: ByteOffset,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub enum CommandCursor<'a> {
    /// TODO: docs.
    InArg {
        /// TODO: docs.
        arg: CommandArg<'a>,

        /// TODO: docs.
        offset: ByteOffset,
    },

    /// TODO: docs.
    BetweenArgs {
        /// TODO: docs.
        prev: Option<CommandArg<'a>>,

        /// TODO: docs.
        next: Option<CommandArg<'a>>,
    },
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub enum CommandArgsIntoSeqError<'a, T> {
    /// TODO: docs.
    Item(T),

    /// TODO: docs.
    WrongNum(CommandArgsWrongNumError<'a>),
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub struct CommandArgsWrongNumError<'a> {
    args: CommandArgs<'a>,
    actual_num: usize,
    expected_num: usize,
}

impl<'a> CommandArgs<'a> {
    /// TODO: docs.
    #[inline]
    pub fn arg(&self, idx: CommandArgIdx) -> Option<CommandArg<'a>> {
        (self.inner.len() <= idx.end).then_some(CommandArg {
            idx,
            inner: &self.inner[idx.start.into()..idx.end.into()],
        })
    }

    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.inner
    }

    /// TODO: docs.
    #[inline]
    pub fn byte_len(&self) -> ByteOffset {
        self.as_str().len().into()
    }

    /// TODO: docs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// TODO: docs.
    #[inline]
    pub fn iter(&self) -> CommandArgsIter<'a> {
        CommandArgsIter { inner: self.as_str(), last_idx_end: 0usize.into() }
    }

    /// TODO: docs.
    #[inline]
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    /// TODO: docs.
    #[inline]
    pub fn new(args: &'a str) -> Self {
        Self { inner: args }
    }

    /// TODO: docs.
    #[inline]
    pub fn to_cursor(&self, offset: ByteOffset) -> CommandCursor<'a> {
        debug_assert!(offset <= self.inner.len());

        let mut prev = None;
        for arg in self.iter() {
            let idx = arg.idx();
            if offset < idx.start {
                return CommandCursor::BetweenArgs { prev, next: Some(arg) };
            }
            if offset <= idx.end {
                return CommandCursor::InArg {
                    arg,
                    offset: offset - idx.start,
                };
            }
            prev = Some(arg);
        }
        CommandCursor::BetweenArgs { prev, next: None }
    }

    #[inline]
    pub(crate) fn pop_front(&mut self) -> Option<CommandArg<'a>> {
        let mut iter = self.iter();
        let first = iter.next();
        *self = iter.remainder();
        first
    }
}

impl<'a> CommandArg<'a> {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.inner
    }

    /// TODO: docs.
    #[inline]
    pub fn end(&self) -> ByteOffset {
        self.idx.end
    }

    /// Returns the index of the argument in the [`CommandArgs`].
    #[inline]
    pub fn idx(&self) -> CommandArgIdx {
        self.idx
    }

    /// TODO: doc.
    #[inline]
    pub fn start(&self) -> ByteOffset {
        self.idx.start
    }
}

impl<'a> CommandArgsIter<'a> {
    #[inline]
    pub(crate) fn remainder(self) -> CommandArgs<'a> {
        CommandArgs { inner: self.inner }
    }
}

struct ArgsList<'a>(CommandArgsIter<'a>);

impl fmt::Debug for ArgsList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DebugAsStr<'a>(CommandArg<'a>);
        impl fmt::Debug for DebugAsStr<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self.0.as_ref(), f)
            }
        }

        f.debug_list().entries(self.0.clone().map(DebugAsStr)).finish()
    }
}

impl fmt::Debug for CommandArgs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CommandArgs").field(&ArgsList(self.iter())).finish()
    }
}

impl fmt::Debug for CommandArg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CommandArg").field(self).finish()
    }
}

impl AsRef<str> for CommandArg<'_> {
    #[inline]
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for CommandArg<'_> {
    type Target = str;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.inner
    }
}

impl PartialEq<str> for CommandArg<'_> {
    #[inline]
    fn eq(&self, s: &str) -> bool {
        &**self == s
    }
}

impl PartialEq<&str> for CommandArg<'_> {
    #[inline]
    fn eq(&self, s: &&str) -> bool {
        self == *s
    }
}

impl PartialEq<CommandArg<'_>> for str {
    #[inline]
    fn eq(&self, arg: &CommandArg<'_>) -> bool {
        arg == self
    }
}

impl PartialEq<CommandArg<'_>> for &str {
    #[inline]
    fn eq(&self, arg: &CommandArg<'_>) -> bool {
        *self == arg
    }
}

impl fmt::Debug for CommandArgsIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("CommandArgsIter")
            .field(&ArgsList(self.clone()))
            .finish()
    }
}

impl<'a> Iterator for CommandArgsIter<'a> {
    type Item = CommandArg<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let args = self.inner;
        if args.is_empty() {
            return None;
        }
        let len_whitespace = args.len() - args.trim_start().len();
        let trimmed = &args[len_whitespace..];
        let len_arg = trimmed.find(' ').unwrap_or(trimmed.len());
        let (arg, rest) = trimmed.split_at(len_arg);
        self.inner = rest;
        let idx_start = self.last_idx_end + len_whitespace;
        let idx_end = idx_start + len_arg;
        self.last_idx_end = idx_end;
        (len_arg > 0).then_some(CommandArg {
            inner: arg,
            idx: CommandArgIdx { start: idx_start, end: idx_end },
        })
    }
}

impl<'a> TryFrom<CommandArgs<'a>> for () {
    type Error = CommandArgsWrongNumError<'a>;

    #[inline]
    fn try_from(args: CommandArgs<'a>) -> Result<Self, Self::Error> {
        args.is_empty().then_some(()).ok_or(CommandArgsWrongNumError {
            args,
            actual_num: args.len(),
            expected_num: 0,
        })
    }
}

impl<'a, const N: usize, T> TryFrom<CommandArgs<'a>> for [T; N]
where
    T: TryFrom<CommandArg<'a>>,
{
    type Error = CommandArgsIntoSeqError<'a, T::Error>;

    #[inline]
    fn try_from(args: CommandArgs<'a>) -> Result<Self, Self::Error> {
        let mut array = maybe_uninit_uninit_array::<T, N>();
        let mut num_initialized = 0;
        let mut iter = args.iter();

        let maybe_err = loop {
            let arg = match iter.next() {
                Some(arg) if num_initialized < N => arg,
                Some(_) => {
                    break Some(Self::Error::WrongNum(
                        CommandArgsWrongNumError {
                            args,
                            actual_num: num_initialized + 1 + iter.count(),
                            expected_num: N,
                        },
                    ));
                },
                None if num_initialized < N => {
                    break Some(Self::Error::WrongNum(
                        CommandArgsWrongNumError {
                            args,
                            actual_num: num_initialized,
                            expected_num: N,
                        },
                    ));
                },
                None => break None,
            };
            let item = match T::try_from(arg) {
                Ok(item) => item,
                Err(err) => break Some(Self::Error::Item(err)),
            };
            array[num_initialized] = MaybeUninit::new(item);
            num_initialized += 1;
        };

        if let Some(err) = maybe_err {
            // The initialized elements in the array must be dropped manually.
            for maybe_uninit in &mut array[..num_initialized] {
                // SAFETY: the first `num_initialized` elements have been
                // initialized.
                unsafe { maybe_uninit.assume_init_drop() };
            }
            Err(err)
        } else {
            // SAFETY: MaybeUninit is layout-transparent and all the elements
            // have been initialized.
            Ok(unsafe { maybe_uninit_array_assume_init(array) })
        }
    }
}

impl<T: notify::Error> notify::Error for CommandArgsIntoSeqError<'_, T> {
    #[inline]
    fn to_message(
        &self,
        namespace: &notify::Namespace,
    ) -> (notify::Level, notify::Message) {
        match self {
            Self::Item(err) => err.to_message(namespace),
            Self::WrongNum(err) => err.to_message(namespace),
        }
    }
}

impl notify::Error for CommandArgsWrongNumError<'_> {
    #[inline]
    fn to_message(
        &self,
        _: &notify::Namespace,
    ) -> (notify::Level, notify::Message) {
        debug_assert_ne!(self.args.len(), self.expected_num);

        let mut message = notify::Message::new();
        message
            .push_str("expected ")
            .push_expected(self.expected_num.to_smolstr())
            .push_str(" argument")
            .push_str(if self.expected_num == 1 { "" } else { "s" })
            .push_str(", but got ")
            .push_actual(self.actual_num.to_smolstr());

        if !self.args.is_empty() {
            message.push_str(": ").push_comma_separated(
                self.args.iter(),
                notify::SpanKind::Warning,
            );
        }

        (notify::Level::Error, message)
    }
}

/// Stable version of [`MaybeUninit::uninit_array`].
///
/// Remove this when std's implementation is stabilized.
#[inline]
fn maybe_uninit_uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    unsafe { mem::MaybeUninit::uninit().assume_init() }
}

/// Stable version of [`MaybeUninit::array_assume_init`].
///
/// Remove this when std's implementation is stabilized.
#[inline]
unsafe fn maybe_uninit_array_assume_init<T, const N: usize>(
    array: [MaybeUninit<T>; N],
) -> [T; N] {
    unsafe { (&array as *const [MaybeUninit<T>; N] as *const [T; N]).read() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_args_iter() {
        let args = CommandArgs::new("  foo bar  baz   ");
        let mut iter = args.iter();
        assert_eq!(iter.next().unwrap(), "foo");
        assert_eq!(iter.next().unwrap(), "bar");
        assert_eq!(iter.next().unwrap(), "baz");
        assert!(iter.next().is_none());
    }
}
