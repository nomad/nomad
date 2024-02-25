use nomad::*;

/// TODO: docs.
pub struct Collab {}

impl Module for Collab {
    const NAME: ModuleName = module_name!("collab");
}
