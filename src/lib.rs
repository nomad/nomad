use nomad::neovim::{nvim_oxi, Neovim};
use nomad::Nomad;

#[nvim_oxi::plugin]
fn nomad() -> Nomad<Neovim> {
    Nomad::new(Neovim)
        .with_module::<auth::NeovimAuth>()
        .with_module::<collab::NeovimCollab>()
        .with_module::<status::NeovimStatus>()
}
