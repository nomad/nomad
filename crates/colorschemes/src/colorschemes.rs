use std::convert::Infallible;

use common::*;
use fuzzy_modal::*;

use crate::*;

#[derive(Default)]
pub struct Colorschemes {
    is_disabled: bool,
    choose_modal: Option<FuzzyHandle>,
    sender: LateInit<Sender<Message>>,
}

pub enum Message {
    Close,
    Disable,
    Load(String),
    Choose,
}

impl Plugin for Colorschemes {
    const NAME: &'static str = "colorscheme";

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

        builder.function("choose").on_execute(|()| Message::Choose).build();
    }

    fn update_config(&mut self, config: Enable<Config>) {
        if !config.enable() {
            self.disable();
            return;
        }

        if let Some(colorscheme) = config.into_inner().enabled_colorscheme() {
            self.send(Message::Load(colorscheme));
        }
    }

    fn handle_message(
        &mut self,
        msg: Message,
        ctx: &Ctx<Self>,
    ) -> Result<(), Infallible> {
        if self.is_disabled {
            return Ok(());
        }

        match msg {
            Message::Close => self.close_choose_modal(),
            Message::Disable => self.disable(),
            Message::Load(colorscheme) => self.load(&colorscheme),
            Message::Choose => self.choose_colorscheme(ctx),
        };

        Ok(())
    }
}

impl Colorschemes {
    fn close_choose_modal(&mut self) {
        if let Some(modal) = self.choose_modal.take() {
            modal.close();
        }
    }

    fn choose_colorscheme(&mut self, ctx: &Ctx<Self>) {
        self.close_choose_modal();
        self.open_choose_modal(ctx);
    }

    fn disable(&mut self) {
        self.is_disabled = true;
        self.close_choose_modal();
    }

    fn load(&mut self, colorscheme: &str) {
        let Some(colorscheme) = schemes::colorschemes().get(colorscheme)
        else {
            todo!();
        };
        colorscheme.load().unwrap();
    }

    fn open_choose_modal(&mut self, ctx: &Ctx<Self>) {
        // TODO: get current colorscheme.
        let original_colorscheme = "Ayu Mirage".to_owned();

        let on_select_sender = self.sender.clone();

        let on_confirm_sender = self.sender.clone();

        let on_exit_sender = self.sender.clone();

        let modal = ctx
            .with_plugin::<FuzzyModal, _, _>(FuzzyModal::builder)
            .with_starting_text("Choose colorscheme...")
            .with_items(
                schemes::colorschemes().keys().copied().map(FuzzyItem::new),
            )
            .on_select(move |item| {
                let colorscheme = item.text.clone();
                on_select_sender.send(Message::Load(colorscheme));
            })
            .on_confirm(move |item| {
                let colorscheme = item.text;
                on_confirm_sender.send(Message::Load(colorscheme));
            })
            .on_cancel(move |_| {
                on_exit_sender.send(Message::Load(original_colorscheme));
            })
            .open();

        self.choose_modal = Some(modal);
    }

    fn send(&mut self, msg: Message) {
        self.sender.send(msg);
    }
}
