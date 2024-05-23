use nomad::prelude::*;

use crate::{Activity, Config, Join, Start};

/// TODO: docs.
pub struct Collab {
    pub(crate) activity: Shared<Activity>,
    pub(crate) config: Get<Config>,
}

impl Collab {
    fn new(config: Get<Config>) -> Self {
        Self { activity: Shared::new(Activity::default()), config }
    }
}

impl Module for Collab {
    const NAME: ModuleName = module_name!("collab");

    type Config = Config;

    fn init(config: Get<Self::Config>) -> Api<Self> {
        let collab = Self::new(config);

        let join = Join::new(&collab);

        let start = Start::new(&collab);

        Api::new(collab)
            .with_command(start.clone())
            .with_command(join.clone())
            .with_function(start)
            .with_function(join)
    }

    async fn run(&self) {}
}
