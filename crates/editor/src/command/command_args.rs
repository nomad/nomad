use core::error::Error;
use core::fmt;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;
use core::str::FromStr;

use smol_str::ToSmolStr;

use crate::editor::ByteOffset;
use crate::notify;

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct CommandArgs<'a, CursorOffset = ()> {
    inner: &'a str,
    cursor_offset: CursorOffset,
}

/// A group of adjacent non-whitespace characters in a [`CommandArgs`].
#[derive(Debug, Copy, Clone)]
pub struct CommandArg<'a> {
    /// The argument's text, guaranteed to be non-empty and not contain
    /// whitespace.
    word: &'a str,

    /// The offset of argument word in the original [`CommandArgs`].
    offset: ByteOffset,

    /// The index of this argument in the original [`CommandArgs`].
    index: usize,
}

/// An iterator over the [`CommandArg`]s of a [`CommandArgs`].
#[derive(Clone)]
pub struct CommandArgsIter<'a> {
    /// The portion of the original [`CommandArgs::inner`] that hasn't been
    /// yielded yet.
    inner: &'a str,

    /// The offset of `inner` in the original [`CommandArgs`].
    offset: ByteOffset,

    /// The number of arguments yielded so far.
    num_yielded: usize,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub enum CursorPosition<'a> {
    /// TODO: docs.
    InArg(CommandArg<'a>, ByteOffset),

    /// TODO: docs.
    BetweenArgs(Option<CommandArg<'a>>, Option<CommandArg<'a>>),
}

/// A compatibility wrapper from `FromStr` to `TryFrom<CommandArgs<'_>>`.
#[derive(
    Debug,
    Copy,
    Clone,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Deserialize,
)]
#[serde(transparent)]
pub struct Parse<T>(pub T);

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

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub enum ParseFromCommandArgsError<'a, T> {
    /// TODO: docs.
    FromStr(CommandArg<'a>, T),

    /// TODO: docs.
    WrongNum(CommandArgsWrongNumError<'a>),
}

impl<'a, C> CommandArgs<'a, C> {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.inner
    }

    /// TODO: docs.
    #[inline]
    pub fn byte_len(&self) -> ByteOffset {
        self.as_str().len()
    }

    /// TODO: docs.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// TODO: docs.
    #[inline]
    pub fn iter(&self) -> CommandArgsIter<'a> {
        CommandArgsIter { inner: self.as_str(), offset: 0, num_yielded: 0 }
    }

    /// TODO: docs.
    #[inline]
    pub fn len(&self) -> usize {
        self.iter().count()
    }
}

impl<'a> CommandArgs<'a> {
    /// Creates a new [`CommandArgs`] from the given arguments string.
    #[inline]
    pub fn new(args: &'a str) -> Self {
        Self { inner: args, cursor_offset: () }
    }

    #[inline]
    pub(crate) fn pop_front(&mut self) -> Option<CommandArg<'a>> {
        let mut iter = self.iter();
        let first = iter.next();
        *self = iter.remainder();
        first
    }
}

impl<'a> CommandArgs<'a, ByteOffset> {
    /// The offset of the cursor in the arguments string.
    pub fn cursor_offset(&self) -> ByteOffset {
        self.cursor_offset
    }

    /// TODO: docs.
    #[inline]
    pub fn cursor_pos(&self) -> CursorPosition<'a> {
        let mut prev = None;
        for arg in self.iter() {
            if self.cursor_offset() < arg.offset() {
                return CursorPosition::BetweenArgs(prev, Some(arg));
            }
            if self.cursor_offset() <= arg.offset() + arg.len() {
                return CursorPosition::InArg(
                    arg,
                    self.cursor_offset() - arg.offset(),
                );
            }
            prev = Some(arg);
        }
        CursorPosition::BetweenArgs(prev, None)
    }

    /// Creates a new [`CommandArgs`] from the given arguments string and
    /// cursor offset.
    #[track_caller]
    #[inline]
    pub fn new(args: &'a str, cursor_offset: ByteOffset) -> Self {
        assert!(cursor_offset <= args.len(), "cursor offset out of bounds");
        Self { inner: args, cursor_offset }
    }
}

impl<'a> CommandArg<'a> {
    /// TODO: docs.
    #[inline]
    pub fn as_str(&self) -> &'a str {
        self.word
    }

    /// The index of this argument in the original [`CommandArgs`].
    #[inline]
    pub fn index(&self) -> usize {
        self.index
    }

    /// Returns `true` if this is the first argument in the original
    /// [`CommandArgs`].
    #[inline]
    pub fn is_first(&self) -> bool {
        self.index == 0
    }

    /// The offset of argument word in the original [`CommandArgs`].
    #[inline]
    pub fn offset(&self) -> ByteOffset {
        self.offset
    }
}

impl<'a> CommandArgsIter<'a> {
    #[inline]
    pub(crate) fn remainder(self) -> CommandArgs<'a> {
        CommandArgs::<()>::new(self.inner)
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

impl fmt::Debug for CommandArgs<'_, ()> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl fmt::Debug for CommandArgs<'_, ByteOffset> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (pre, post) = self.inner.split_at(self.cursor_offset);
        write!(f, "\"{}|{}\"", pre.escape_debug(), post.escape_debug())
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
        self.word
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
        let arg_len = trimmed.find(' ').unwrap_or(trimmed.len());
        let (arg, rest) = trimmed.split_at(arg_len);
        self.inner = rest;
        let arg_offset = self.offset + len_whitespace;
        self.offset = arg_offset + arg_len;
        let index = self.num_yielded;
        self.num_yielded += 1;
        (arg_len > 0).then_some(CommandArg {
            word: arg,
            offset: arg_offset,
            index,
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

impl<'a, T: FromStr> TryFrom<CommandArgs<'a>> for Parse<T> {
    type Error = ParseFromCommandArgsError<'a, T::Err>;

    #[inline]
    fn try_from(args: CommandArgs<'a>) -> Result<Self, Self::Error> {
        let [arg] =
            <[CommandArg<'a>; 1]>::try_from(args).map_err(
                |err| match err {
                    CommandArgsIntoSeqError::Item(_never) => {
                        unreachable!()
                    },
                    CommandArgsIntoSeqError::WrongNum(err) => {
                        ParseFromCommandArgsError::WrongNum(err)
                    },
                },
            )?;

        arg.as_str()
            .parse()
            .map(Parse)
            .map_err(|err| ParseFromCommandArgsError::FromStr(arg, err))
    }
}

impl<T: notify::Error> notify::Error for CommandArgsIntoSeqError<'_, T> {
    #[inline]
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::Item(err) => err.to_message(),
            Self::WrongNum(err) => err.to_message(),
        }
    }
}

impl notify::Error for CommandArgsWrongNumError<'_> {
    #[inline]
    fn to_message(&self) -> (notify::Level, notify::Message) {
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

impl<T: Error> notify::Error for ParseFromCommandArgsError<'_, T> {
    #[inline]
    fn to_message(&self) -> (notify::Level, notify::Message) {
        match self {
            Self::FromStr(arg, err) => {
                let mut message = notify::Message::from_str("couldn't parse ");
                message
                    .push_invalid(arg.as_str())
                    .push_str(": ")
                    .push_str(err.to_smolstr());
                (notify::Level::Error, message)
            },
            Self::WrongNum(err) => err.to_message(),
        }
    }
}

/// Stable version of [`MaybeUninit::uninit_array`].
///
/// Remove this when std's implementation is stabilized.
#[inline]
fn maybe_uninit_uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    // SAFETY: we're initializing a bunch of `MaybeUninit`s, which don't
    // require initialization.
    unsafe { mem::MaybeUninit::uninit().assume_init() }
}

/// Stable version of [`MaybeUninit::array_assume_init`].
///
/// Remove this when std's implementation is stabilized.
#[inline]
unsafe fn maybe_uninit_array_assume_init<T, const N: usize>(
    array: [MaybeUninit<T>; N],
) -> [T; N] {
    // SAFETY: up to the caller.
    unsafe { (&array as *const [MaybeUninit<T>; N] as *const [T; N]).read() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_args_iter() {
        let args = CommandArgs::<()>::new("  foo bar  baz   ");
        let mut iter = args.iter();
        assert_eq!(iter.next().unwrap(), "foo");
        assert_eq!(iter.next().unwrap(), "bar");
        assert_eq!(iter.next().unwrap(), "baz");
        assert!(iter.next().is_none());
    }
}
