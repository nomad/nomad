use futures_util::stream::{select, Select};
use futures_util::{Stream, StreamExt};
use nomad::neovim::{
    command,
    function,
    module_api,
    CommandEvent,
    ConfigEvent,
    FunctionEvent,
    ModuleApi,
    Neovim,
};
use nomad::{module_name, Context, Module, ModuleName, Subscription};

use crate::collab_editor::CollabEditor;
use crate::events::{
    Cursor,
    CursorEvent,
    Edit,
    EditEvent,
    JoinSession,
    Selection,
    SelectionEvent,
    StartSession,
};
use crate::{Collab, Config};

/// TODO: docs.
pub struct NeovimCollab(Collab<Neovim>);

impl Module<Neovim> for NeovimCollab {
    const NAME: ModuleName = module_name!("collab");

    type Config = Config;

    fn init(ctx: &Context<Neovim>) -> (Self, ModuleApi) {
        let (api, config_stream) = module_api::<Self>(ctx);

        let (join_cmd, join_cmd_sub) = command::<JoinSession>(ctx);
        let (start_cmd, start_cmd_sub) = command::<StartSession>(ctx);

        let (join_fn, join_fn_sub) = function::<JoinSession>(ctx);
        let (start_fn, start_fn_sub) = function::<StartSession>(ctx);

        let collab = Self(Collab {
            ctx: ctx.clone(),
            config: Config::default(),
            config_stream,
            join_stream: select(join_cmd_sub, join_fn_sub),
            start_stream: select(start_cmd_sub, start_fn_sub),
        });

        let api = api
            .with_command(join_cmd)
            .with_command(start_cmd)
            .with_function(join_fn)
            .with_function(start_fn);

        (collab, api)
    }

    async fn run(&mut self, _: &Context<Neovim>) {
        self.0.run().await;
    }
}

impl CollabEditor for Neovim {
    type ConfigStream = Subscription<ConfigEvent<NeovimCollab>, Neovim>;
    type JoinStream = Select<
        Subscription<CommandEvent<JoinSession>, Neovim>,
        Subscription<FunctionEvent<JoinSession>, Neovim>,
    >;
    type StartStream = Select<
        Subscription<CommandEvent<StartSession>, Neovim>,
        Subscription<FunctionEvent<StartSession>, Neovim>,
    >;
    type EditStream = Subscription<EditEvent, Neovim>;
    type CursorStream = Subscription<CursorEvent, Neovim>;
    type SelectionStream = Subscription<SelectionEvent, Neovim>;
}
