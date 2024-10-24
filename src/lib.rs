use nomad::neovim::{nvim_oxi, Neovim};
use nomad::Nomad;

#[nvim_oxi::plugin]
fn nomad() -> Nomad {
    Nomad::new()
        .with_module::<auth::Auth>()
        .with_module::<collab::Collab>()
        .with_module::<status::Status>()
        .with_module::<version::Version>()
}
