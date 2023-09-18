use std::collections::HashMap;
use std::convert::Infallible;

use common::nvim;
use common::*;

use crate::*;

/// TODO: docs
#[derive(Default)]
pub struct Seph {
    /// TODO: docs
    config: Config,

    /// TODO: docs
    is_disabled: bool,

    /// TODO: docs
    sender: LateInit<Sender<Message>>,

    /// TODO: docs
    views: HashMap<ViewId, View>,
}

impl Plugin for Seph {
    const NAME: &'static str = "seph";

    type Message = Message;

    type Config = Config;

    type InitError = Infallible;

    type HandleMessageError = Infallible;

    fn init(
        &mut self,
        sender: &Sender<Self::Message>,
    ) -> Result<(), Infallible> {
        self.sender.init(sender.clone());
        Ok(())
    }

    fn init_api(builder: &mut ApiBuilder<'_, Self>) {
        builder
            .function("open")
            .on_execute(|config: Option<WindowConfig>| Message::Open(config))
            .build();

        builder.function("close").on_execute(|()| Message::Close).build();
    }

    // fn init_keymaps(builder: &mut KeymapBuilder<'_, Self>) {
    //     builder
    //         .in_mode(Mode::Normal)
    //         .map("ll")
    //         .to(Message::Open(None))
    //         .build();
    //
    //     builder
    //         .in_mode(Mode::Normal)
    //         .map("ll")
    //         .to(Message::Open(None))
    //         .build();
    // }

    fn init_commands(builder: &mut CommandBuilder<'_, Self>) {
        builder
            .command("Seph")
            .on_execute(|_opts| Message::Open(None))
            .with_desc("Open a new seph window")
            .build();

        builder
            .command("SephClose")
            .on_execute(|_opts| Message::Close)
            .with_desc("Close the current seph window")
            .build();
    }

    fn update_config(&mut self, config: Enable<Self::Config>) {
        if !config.enable() {
            self.disable();
        }
        self.config = config.into_inner();
    }

    fn handle_message(&mut self, msg: Message) -> Result<(), Infallible> {
        if self.is_disabled {
            return Ok(());
        }

        match msg {
            Message::Close => self.close(),
            Message::Disable => self.disable(),
            Message::Open(config) => self.open(config),
        };

        Ok(())
    }
}

impl Seph {
    fn close(&mut self) {
        let focused_win = nvim::api::Window::current();
        if let Some(view) = self.views.remove(&focused_win) {
            view.close();
        }
    }

    fn disable(&mut self) {
        for (_, view) in self.views.drain() {
            view.close();
        }
        self.is_disabled = true;
    }

    fn open(&mut self, config: Option<WindowConfig>) {
        let config = config.as_ref().unwrap_or(&self.config.window);
        let path = nvim::api::Buffer::current().get_name().unwrap();
        if let Ok(view) = View::new(path, config) {
            self.views.insert(view.id(), view);
        }
    }

    #[allow(dead_code)]
    fn send(&mut self, msg: Message) {
        self.sender.send(msg);
    }
}
