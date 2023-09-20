use crate::*;

#[derive(Default)]
pub(crate) struct AyuMirage;

impl Colorscheme for AyuMirage {
    const NAME: &'static str = "Ayu Mirage";

    fn palette(&self) -> Palette {
        Palette {
            foreground: hex!("#cccac2"),
            background: hex!("#242936"),
            string: hex!("#d6ff80"),
        }
    }
}

impl BuiltinColorscheme for AyuMirage {}

impl SyntaxColorscheme for AyuMirage {}

impl DiagnosticColorscheme for AyuMirage {}

impl LspColorscheme for AyuMirage {}

impl TreeSitterColorscheme for AyuMirage {}

impl NomadColorscheme for AyuMirage {}

impl TelescopeColorscheme for AyuMirage {}
