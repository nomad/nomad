use nomad::{nvim_oxi, Nomad};

#[nvim_oxi::plugin(nvim_oxi = nvim_oxi)]
fn nomad() -> Nomad {
    Nomad::new().with_module::<auth::Auth>().with_module::<collab::Collab>()
    // .with_module::<version::Version>()
}
