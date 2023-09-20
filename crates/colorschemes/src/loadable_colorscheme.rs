use common::nvim;

use crate::{Colorscheme, HighlightGroup};

/// TODO: docs
pub(crate) trait LoadableColorscheme: Send + Sync {
    /// TODO: docs
    fn load(&self) -> nvim::Result<()>;
}

impl<C: Colorscheme + Send + Sync> LoadableColorscheme for C {
    fn load(&self) -> nvim::Result<()> {
        load_colorscheme(self)
    }
}

fn load_colorscheme<C>(colorscheme: &C) -> nvim::Result<()>
where
    C: Colorscheme,
{
    let palette = colorscheme.palette();

    set_hl("Normal", C::normal(&palette))?;
    set_hl("ColorColumn", C::color_column(&palette))?;

    Ok(())
}

fn set_hl(hl_name: &str, hl_group: HighlightGroup) -> nvim::Result<()> {
    nvim::api::set_hl(0, hl_name, &hl_group.into()).map_err(Into::into)
}
