use core::fmt;
use core::mem::{self, MaybeUninit};
use core::ops::Deref;

use nvimx_common::ByteOffset;
use nvimx_diagnostics::{DiagnosticMessage, HighlightGroup};

/// TODO: docs.
#[derive(Copy, Clone)]
pub struct SubCommandArgs<'a> {
    /// Starts at the first non-whitespace character after the subcommand name,
    /// and includes all characters up to the end of the command line,
    /// including any trailing whitespace.
    args: &'a str,
}

/// A group of adjacent non-whitespace characters in a [`SubCommandArgs`].
#[derive(Copy, Clone)]
pub struct SubCommandArg<'a> {
    arg: &'a str,
    idx: SubCommandArgIdx,
}

/// The index of a [`SubCommandArg`] in a [`SubCommandArgs`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SubCommandArgIdx {
    pub(crate) start: ByteOffset,
    pub(crate) end: ByteOffset,
}

/// An iterator over the [`SubCommandArg`]s of a [`SubCommandArgs`].
#[derive(Clone)]
pub struct SubCommandArgsIter<'a> {
    args: &'a str,
    last_idx_end: ByteOffset,
}

/// TODO: docs.
#[derive(Debug, Copy, Clone)]
pub enum SubCommandCursor<'a> {
    /// TODO: docs.
    InArg {
        /// TODO: docs.
        arg: SubCommandArg<'a>,
        /// TODO: docs.
        offset: ByteOffset,
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
        (self.args.len() <= idx.end).then_some(SubCommandArg {
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
        SubCommandArgsIter { args: self.args, last_idx_end: 0 }
    }

    /// TODO: docs.
    pub fn len(&self) -> usize {
        self.iter().count()
    }

    pub(crate) fn as_str(&self) -> &'a str {
        self.args
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
    pub(crate) fn remainder(self) -> SubCommandArgs<'a> {
        SubCommandArgs { args: self.args }
    }
}

impl<'a> SubCommandCursor<'a> {
    pub(crate) fn new(args: &SubCommandArgs<'a>, offset: ByteOffset) -> Self {
        debug_assert!(offset <= args.args.len());

        let mut prev = None;
        for arg in args.iter() {
            let idx = arg.idx();
            if offset < idx.start {
                return Self::BetweenArgs { prev, next: Some(arg) };
            }
            if offset <= idx.end {
                return Self::InArg { arg, offset: offset - idx.start };
            }
            prev = Some(arg);
        }
        Self::BetweenArgs { prev, next: None }
    }
}

struct ArgsList<'a>(SubCommandArgsIter<'a>);

impl fmt::Debug for ArgsList<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        struct DebugAsStr<'a>(SubCommandArg<'a>);
        impl fmt::Debug for DebugAsStr<'_> {
            fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
                fmt::Debug::fmt(self.0.as_ref(), f)
            }
        }

        f.debug_list().entries(self.0.clone().map(DebugAsStr)).finish()
    }
}

impl fmt::Debug for SubCommandArgs<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("SubCommandArgs").field(&ArgsList(self.iter())).finish()
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

impl fmt::Debug for SubCommandArgsIter<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("SubCommandArgsIter")
            .field(&ArgsList(self.clone()))
            .finish()
    }
}

impl<'a> Iterator for SubCommandArgsIter<'a> {
    type Item = SubCommandArg<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let args = self.args;
        if args.is_empty() {
            return None;
        }
        let len_whitespace = args.len() - args.trim_start().len();
        let trimmed = &args[len_whitespace..];
        let len_arg = trimmed.find(' ').unwrap_or(trimmed.len());
        let (arg, rest) = trimmed.split_at(len_arg);
        self.args = rest;
        let idx_start = self.last_idx_end + len_whitespace;
        let idx_end = idx_start + len_arg;
        self.last_idx_end = idx_end;
        (len_arg > 0).then_some(SubCommandArg {
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
                    ));
                },
                None if num_initialized < N => {
                    break Some(Self::Error::WrongNum(
                        SubCommandArgsWrongNumError {
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
        assert_eq!(iter.next().unwrap(), "foo");
        assert_eq!(iter.next().unwrap(), "bar");
        assert_eq!(iter.next().unwrap(), "baz");
        assert!(iter.next().is_none());
    }
}
