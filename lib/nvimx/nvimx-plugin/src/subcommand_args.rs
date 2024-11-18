use core::fmt;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;

use nvimx_common::ByteOffset;
use nvimx_diagnostics::{DiagnosticMessage, HighlightGroup};

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub struct SubCommandArgs<'a> {
    /// Starts at the first non-whitespace character after the subcommand name,
    /// and includes all characters up to the end of the command line,
    /// including any trailing whitespace.
    args: &'a str,
}

/// A group of adjacent non-whitespace characters in a [`SubCommandArgs`].
pub struct SubCommandArg<'a> {
    arg: &'a str,
    idx: SubCommandArgIdx,
}

/// The index of a [`SubCommandArg`] in a [`SubCommandArgs`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubCommandArgIdx {
    start: ByteOffset,
    end: ByteOffset,
}

/// An iterator over the [`SubCommandArg`]s of a [`SubCommandArgs`].
pub struct SubCommandArgsIter<'a> {
    args: &'a str,
    arg_offset: ByteOffset,
}

/// TODO: docs.
pub enum SubCommandCursor<'a> {
    /// TODO: docs.
    InArg {
        /// TODO: docs.
        argument: SubCommandArg<'a>,
        /// TODO: docs.
        cursor_offset: ByteOffset,
    },

    /// TODO: docs.
    BetweenArgs {
        /// TODO: docs.
        prev: Option<SubCommandArg<'a>>,

        /// TODO: docs.
        next: Option<SubCommandArg<'a>>,
    },
}

#[derive(Debug, Copy, Clone)]
pub enum SubCommandArgsIntoSliceError<'a, T> {
    Item(T),
    WrongNum(SubCommandArgsWrongNumError<'a>),
}

#[derive(Debug, Copy, Clone)]
pub struct SubCommandArgsWrongNumError<'a> {
    args: SubCommandArgs<'a>,
    actual_num: usize,
    expected_num: usize,
}

impl<'a> SubCommandArgs<'a> {
    /// TODO: docs.
    pub fn arg(&self, idx: SubCommandArgIdx) -> Option<SubCommandArg<'a>> {
        (self.args.len() <= idx.end.into()).then_some(SubCommandArg {
            idx,
            arg: &self.args[idx.start.into()..idx.end.into()],
        })
    }

    /// TODO: docs.
    pub fn is_empty(&self) -> bool {
        self.iter().next().is_none()
    }

    /// TODO: docs.
    pub fn iter(&self) -> SubCommandArgsIter<'a> {
        SubCommandArgsIter { args: self.args, arg_offset: 0usize.into() }
    }

    /// TODO: docs.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub(crate) fn new(args: &'a str) -> Self {
        Self { args }
    }

    /// TODO: docs.
    pub(crate) fn pop_front(&mut self) -> Option<SubCommandArg<'a>> {
        let mut iter = self.iter();
        let first = iter.next();
        *self = iter.remainder();
        first
    }
}

impl SubCommandArg<'_> {
    /// Returns the index of the argument in the [`SubCommandArgs`].
    pub fn idx(&self) -> SubCommandArgIdx {
        self.idx
    }
}

impl<'a> SubCommandArgsIter<'a> {
    /// Returns the [`ByteOffset`] of the last yielded argument in the original
    /// [`SubCommandArgs`], or:
    ///
    /// - 0 if [`next`](Self::next) has never been called;
    /// - the length of the original [`SubCommandArgs`] if [`next`](Self::next)
    ///   has returned `None`.
    pub(crate) fn last_offset(&self) -> ByteOffset {
        self.arg_offset
    }

    fn remainder(self) -> SubCommandArgs<'a> {
        SubCommandArgs { args: self.args }
    }
}

impl<'a> SubCommandCursor<'a> {
    pub(crate) fn new(args: &SubCommandArgs<'a>, offset: ByteOffset) -> Self {
        todo!();
    }
}

impl fmt::Debug for SubCommandArg<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("SubCommandArg").field(&self.arg).finish()
    }
}

impl AsRef<str> for SubCommandArg<'_> {
    fn as_ref(&self) -> &str {
        self
    }
}

impl Deref for SubCommandArg<'_> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.arg
    }
}

impl PartialEq<str> for SubCommandArg<'_> {
    fn eq(&self, s: &str) -> bool {
        &**self == s
    }
}

impl PartialEq<&str> for SubCommandArg<'_> {
    fn eq(&self, s: &&str) -> bool {
        self == *s
    }
}

impl PartialEq<SubCommandArg<'_>> for str {
    fn eq(&self, arg: &SubCommandArg<'_>) -> bool {
        arg == self
    }
}

impl PartialEq<SubCommandArg<'_>> for &str {
    fn eq(&self, arg: &SubCommandArg<'_>) -> bool {
        *self == arg
    }
}

impl<'a> Iterator for SubCommandArgsIter<'a> {
    type Item = SubCommandArg<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let args = self.args;
        if args.is_empty() {
            return None;
        }
        debug_assert!(args.starts_with(|c: char| !c.is_whitespace()));
        let arg_len = args.find(char::is_whitespace).unwrap_or(args.len());
        if arg_len == 0 {
            self.args = "";
            return None;
        }
        let idx_start = self.arg_offset;
        self.arg_offset += arg_len.into();
        let idx_end = self.arg_offset;
        let arg = &args[..arg_len];
        self.args = &args[arg_len..];
        Some(SubCommandArg {
            arg,
            idx: SubCommandArgIdx { start: idx_start, end: idx_end },
        })
    }
}

impl<'a> TryFrom<SubCommandArgs<'a>> for () {
    type Error = SubCommandArgsWrongNumError<'a>;

    fn try_from(args: SubCommandArgs<'a>) -> Result<Self, Self::Error> {
        args.is_empty().then_some(()).ok_or(SubCommandArgsWrongNumError {
            args,
            actual_num: args.len(),
            expected_num: 0,
        })
    }
}

impl<'a, const N: usize, T> TryFrom<SubCommandArgs<'a>> for [T; N]
where
    T: TryFrom<SubCommandArg<'a>>,
{
    type Error = SubCommandArgsIntoSliceError<'a, T::Error>;

    fn try_from(args: SubCommandArgs<'a>) -> Result<Self, Self::Error> {
        let mut array = maybe_uninit_uninit_array::<T, N>();
        let mut num_initialized = 0;
        let mut iter = args.iter();

        let maybe_err = loop {
            let arg = match iter.next() {
                Some(arg) if num_initialized < N => arg,
                Some(_) => {
                    break Some(Self::Error::WrongNum(
                        SubCommandArgsWrongNumError {
                            args,
                            actual_num: num_initialized + 1 + iter.count(),
                            expected_num: N,
                        },
                    ))
                },
                None if num_initialized < N => {
                    break Some(Self::Error::WrongNum(
                        SubCommandArgsWrongNumError {
                            args,
                            actual_num: num_initialized,
                            expected_num: N,
                        },
                    ))
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

/// Stable version of [`MaybeUninit::uninit_array`].
///
/// Remove this when std's implementation is stabilized.
fn maybe_uninit_uninit_array<T, const N: usize>() -> [MaybeUninit<T>; N] {
    unsafe { mem::MaybeUninit::uninit().assume_init() }
}

/// Stable version of [`MaybeUninit::array_assume_init`].
///
/// Remove this when std's implementation is stabilized.
unsafe fn maybe_uninit_array_assume_init<T, const N: usize>(
    array: [MaybeUninit<T>; N],
) -> [T; N] {
    (&array as *const [MaybeUninit<T>; N] as *const [T; N]).read()
}

impl<T: Into<DiagnosticMessage>> From<SubCommandArgsIntoSliceError<'_, T>>
    for DiagnosticMessage
{
    fn from(err: SubCommandArgsIntoSliceError<'_, T>) -> Self {
        match err {
            SubCommandArgsIntoSliceError::Item(item) => item.into(),
            SubCommandArgsIntoSliceError::WrongNum(err) => err.into(),
        }
    }
}

impl From<SubCommandArgsWrongNumError<'_>> for DiagnosticMessage {
    fn from(err: SubCommandArgsWrongNumError) -> Self {
        assert_ne!(err.args.len(), err.expected_num);

        let mut message = DiagnosticMessage::new();
        message
            .push_str("expected ")
            .push_str_highlighted(
                err.expected_num.to_string(),
                HighlightGroup::special(),
            )
            .push_str(" argument")
            .push_str(if err.expected_num == 1 { "" } else { "s" })
            .push_str(", but got ")
            .push_str_highlighted(
                err.actual_num.to_string(),
                HighlightGroup::warning(),
            );

        if !err.args.is_empty() {
            message.push_str(": ").push_comma_separated(
                err.args.iter(),
                HighlightGroup::warning(),
            );
        }

        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subcommand_args_iter() {
        let args = SubCommandArgs::new("  foo bar  baz   ");

        let mut iter = args.iter();
        assert_eq!(iter.last_offset(), 0usize);

        assert_eq!(iter.next().unwrap(), "foo");
        assert_eq!(iter.last_offset(), 2usize);

        assert_eq!(iter.next().unwrap(), "bar");
        assert_eq!(iter.last_offset(), 6usize);

        assert_eq!(iter.next().unwrap(), "baz");
        assert_eq!(iter.last_offset(), 11usize);

        assert!(iter.next().is_none());
        assert_eq!(iter.last_offset(), 17usize);
    }
}
