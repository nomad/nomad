use alloc::rc::Rc;
use core::cell::RefCell;

use neovim::nvim::Dictionary;
use neovim::Ctx;

use crate::Module;

/// TODO: docs
pub(crate) trait ObjectSafeModule {
    /// TODO: docs
    fn api(this: &Rc<Self>, ctx: &Rc<RefCell<Ctx>>) -> Dictionary;
}

impl<M: Module> ObjectSafeModule for M {
    #[inline]
    fn api(this: &Rc<Self>, ctx: &Rc<RefCell<Ctx>>) -> Dictionary {
        let mut dict = Dictionary::new();

        for (action_name, action) in this.api().into_iter() {
            let ctx = Rc::clone(ctx);

            let function = move |object| {
                let ctx = &mut *ctx.borrow_mut();
                action(object, ctx.as_set());
            };

            dict.insert(action_name.as_str(), function);
        }

        dict
    }
}
