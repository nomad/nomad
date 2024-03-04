use core::time::Duration;

use nomad::prelude::*;

use crate::CollabConfig;

/// TODO: docs.
pub struct Collab {
    config: Get<EnableConfig<Self>>,
    increment: Increment,
    print: Print,
}

impl DefaultEnable for Collab {
    const ENABLE: bool = false;
}

impl Module for Collab {
    const NAME: ModuleName = module_name!("collab");

    type Config = CollabConfig;

    #[inline]
    fn init(config: Get<EnableConfig<Self>>, ctx: &InitCtx) -> Self {
        let (counter, set_counter) = ctx.new_input(0u64);
        let increment = Increment { set_counter };
        let print = Print { counter };
        Self { config, increment, print }
    }

    #[inline]
    fn api(&self) -> Api {
        Api::new()
            .with_function(self.increment.clone())
            .with_function(self.print.clone())
    }

    #[inline]
    fn commands(&self) -> impl IntoIterator<Item = Command> {
        [self.increment.clone().into(), self.print.clone().into()]
    }

    #[inline]
    async fn load(
        &self,
        // _ctx: &mut SetCtx,
    ) -> impl MaybeResult<()> {
        let mut count = 0;

        loop {
            nvim::print!("{}'s count is {count}", Self::NAME);
            sleep(Duration::from_secs(1)).await;
            count += 1;
        }
    }
}

#[derive(Clone)]
struct Print {
    counter: Get<u64>,
}

impl Action for Print {
    const NAME: ActionName = action_name!("print");

    type Args = ();

    #[inline]
    fn execute(&self, _args: (), ctx: &mut SetCtx) {
        nvim::print!("Collab counter is now {:?}", self.counter.get(ctx))
    }
}

#[derive(Clone)]
struct Increment {
    set_counter: Set<u64>,
}

impl Action for Increment {
    const NAME: ActionName = action_name!("increment");

    type Args = ();

    #[inline]
    fn execute(&self, _args: (), ctx: &mut SetCtx) {
        self.set_counter.update(|counter| *counter += 1, ctx)
    }
}
