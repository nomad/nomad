use core::ops::AddAssign;

use nvim_oxi::lua::ffi::State as LuaState;
use nvim_oxi::{
    lua,
    Dictionary as NvimDictionary,
    Function as NvimFunction,
    Object as NvimObject,
};

use super::{ModuleApi, Neovim};
use crate::Nomad;

const SETUP_FN_NAME: &str = "setup";

/// TODO: docs.
#[derive(Default, Debug)]
pub struct Api {
    dict: NvimDictionary,
}

impl AddAssign<ModuleApi> for Api {
    #[track_caller]
    fn add_assign(&mut self, module_api: ModuleApi) {
        if self.dict.get(&module_api.name).is_some() {
            panic!(
                "a module with the name '{}' already exists in {self:#?}",
                module_api.name
            );
        }

        if module_api.name == SETUP_FN_NAME {
            panic!(
                "got a module with the name '{}', which is reserved for the \
                 setup function",
                module_api.name
            );
        }

        self.dict.insert(module_api.name, module_api.inner);
    }
}

impl lua::Pushable for Api {
    unsafe fn push(mut self, state: *mut LuaState) -> Result<i32, lua::Error> {
        let setup = NvimFunction::from_fn(|obj| setup(obj));
        self.dict.insert(SETUP_FN_NAME, setup);
        self.dict.push(state)
    }
}

fn setup(_obj: NvimObject) {}

impl lua::Pushable for Nomad<Neovim> {
    unsafe fn push(mut self, state: *mut LuaState) -> Result<i32, lua::Error> {
        self.start_modules();
        self.into_api().push(state)
    }
}
