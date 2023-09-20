use std::convert::Infallible;

use common::*;

use crate::*;

#[derive(Default)]
pub struct Colorschemes {
    is_disabled: bool,

    sender: LateInit<Sender<Message>>,
}

pub enum Message {
    Close,
    Disable,
    Load(String),
    Open,
}

impl Plugin for Colorschemes {
    const NAME: &'static str = "colorschemes";

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
            .function("load")
            .on_execute(|colorscheme: String| Message::Load(colorscheme))
            .build();

        builder.function("open").on_execute(|()| Message::Open).build();
    }

    fn update_config(&mut self, config: Enable<Config>) {
        if !config.enable() {
            self.disable();
            return;
        }

        if let Some(colorscheme) = config.into_inner().enabled_colorscheme() {
            self.sender.send(Message::Load(colorscheme));
        }
    }

    fn handle_message(&mut self, msg: Message) -> Result<(), Infallible> {
        if self.is_disabled {
            return Ok(());
        }

        match msg {
            Message::Close => self.close(),
            Message::Disable => self.disable(),
            Message::Load(colorscheme) => self.load(&colorscheme),
            Message::Open => self.open(),
        };

        Ok(())
    }
}

impl Colorschemes {
    fn close(&mut self) {}

    fn load(&mut self, colorscheme: &str) {
        let Some(colorscheme) = schemes::colorschemes().get(colorscheme)
        else {
            todo!();
        };
        colorscheme.load().unwrap();
    }

    fn disable(&mut self) {
        self.is_disabled = true;
    }

    fn open(&mut self) {}

    #[allow(dead_code)]
    fn send(&mut self, msg: Message) {
        self.sender.send(msg);
    }
}
