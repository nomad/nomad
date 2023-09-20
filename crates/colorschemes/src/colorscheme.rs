#![allow(unused_variables)]

use crate::{HighlightGroup, Palette};

/// A [`Colorscheme`] is a collection of [`HighlightGroup`]s that are applied
/// to the UI elements of Neovim.
///
/// The [`Colorscheme`] trait is made up of several sub-traits that correspond
/// to the different types of highlight groups that can be applied to the UI.
///
/// Some sub-traits like [`BuiltinColorscheme`] or [`DiagnosticColorscheme`] refer
/// to highlight groups builtin to Neovim, while others like
/// [`NomadColorscheme`] or [`TelescopeColorscheme`] are specific to 3rd-party
/// plugins.
///
/// This trait system makes it impossible to create a new colorscheme that
/// doesn't address all of the different highlight group types, since omitting
/// any of them would result in a compile-time error.
///
/// The various `*Colorscheme` sub-traits contain methods returning
/// `Option<HighlightGroup>`. All these methods have a default implementation
/// that returns `None`, so you only need to implement the methods for the
/// highlight groups that you want to customize. For example, if you want to
/// create a colorscheme that doesn't set any Telescope highlight groups, you
/// can just:
///
/// ```rust
/// struct MyColorscheme;
///
/// impl TelescopeColorscheme for MyColorscheme {}
/// ```
pub trait Colorscheme:
    Default
    + BuiltinColorscheme
    + SyntaxColorscheme
    + DiagnosticColorscheme
    + LspColorscheme
    + TreeSitterColorscheme
    + NomadColorscheme
    + TelescopeColorscheme
{
    /// TODO: docs
    const NAME: &'static str;

    /// TODO: docs
    fn palette(&self) -> Palette;
}

/// This trait sets the highlight groups that are builtin to (Neo)Vim.
///
/// See [this page][builtin] for more infos.
///
/// [builtin]: https://neovim.io/doc/user/syntax.html#highlight-default
pub trait BuiltinColorscheme {
    /// The highlighting applied to the [`ColorColumn`][cc] highlight group.
    ///
    /// [cc]: https://neovim.io/doc/user/syntax.html#hl-ColorColumn
    fn color_column(palette: &Palette) -> HighlightGroup {
        HighlightGroup::default()
    }

    /// The highlighting applied to the [`Normal`][normal] highlight group.
    ///
    /// [normal]: https://neovim.io/doc/user/syntax.html#hl-Normal
    fn normal(palette: &Palette) -> HighlightGroup {
        HighlightGroup::new()
            .foreground(palette.foreground)
            .background(palette.background)
    }

    /// The highlighting applied to the [`NormalNC`][normal] highlight group.
    ///
    /// [normal]: https://neovim.io/doc/user/syntax.html#hl-NormalNC
    fn normal_nc(palette: &Palette) -> HighlightGroup {
        Self::normal(palette)
    }
}

/// This trait sets the highlight groups that are linked to syntax groups.
///
/// See [this page][syntax] for more infos.
///
/// [syntax]: https://neovim.io/doc/user/syntax.html#group-name
pub trait SyntaxColorscheme {
    /// The highlighting applied to the `String` highlight group.
    fn string(palette: &Palette) -> HighlightGroup {
        HighlightGroup::new().foreground(palette.string)
    }
}

/// TODO: docs
pub trait DiagnosticColorscheme {}

/// TODO: docs
pub trait LspColorscheme {}

/// TODO: docs
pub trait TreeSitterColorscheme {}

/// TODO: docs
pub trait NomadColorscheme {}

/// TODO: docs
pub trait TelescopeColorscheme {}
