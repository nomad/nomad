use core::time::Duration;

use nomad::prelude::*;

use crate::Config;

/// TODO: docs.
pub struct Collab {
    _config: Get<Config>,
}

impl Collab {
    fn new(config: Get<Config>) -> Self {
        Self { _config: config }
    }
}

impl Module for Collab {
    const NAME: ModuleName = module_name!("collab");

    type Config = Config;

    fn init(config: Get<Self::Config>) -> Api<Self> {
        let (counter, set_counter) = new_input(0u64);

        let increment = Increment { set_counter };

        let print = Print { counter };

        Api::new(Self::new(config))
            .with_command(increment.clone())
            .with_command(print.clone())
            .with_function(increment)
            .with_function(print)
    }

    async fn run(&self) -> impl MaybeResult<()> {
        let count = 0;
        nvim::print!("{}'s count is {count}", Self::NAME);
        sleep(Duration::from_secs(1)).await;
    }
}

#[derive(Clone)]
struct Print {
    counter: Get<u64>,
}

impl Action<Collab> for Print {
    const NAME: ActionName = action_name!("print");

    type Args = ();

    type Return = ();

    fn execute(&self, _args: ()) {
        nvim::print!("Collab counter is now {:?}", self.counter.get())
    }
}

#[derive(Clone)]
struct Increment {
    set_counter: Set<u64>,
}

impl Action<Collab> for Increment {
    const NAME: ActionName = action_name!("increment");

    type Args = ();

    type Return = ();

    fn execute(&self, _args: ()) {
        self.set_counter.update(|counter| *counter += 1)
    }
}
