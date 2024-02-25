#[neovim::plugin]
fn nomad() {}

// fn root_view(nvim: &mut Neovim) -> impl View<Neovim> {
//     let (line, mut line_set) = nvim.var(Line::default());
//
//     let (offset, mut offset_set) = nvim.var(Offset::default());
//
//     nvim.autocmd_builder()
//         .on_event(AutocmdEvent::CursorMoved)
//         .on_event(AutocmdEvent::CursorMovedI)
//         .exec(move |_args, ctx| {
//             if let Ok((line, offset)) = api::Window::current().get_cursor() {
//                 line_set.set(Line(line), ctx);
//                 offset_set.set(Offset(offset), ctx);
//             }
//         })
//         .build();
//
//     PrintCoordinates { line, offset }
// }
//
// #[derive(Debug, Clone, Copy, Default)]
// struct Line(usize);
//
// #[derive(Debug, Clone, Copy, Default)]
// struct Offset(usize);
//
// struct PrintCoordinates {
//     line: Get<Line>,
//     offset: Get<Offset>,
// }
//
// impl View<Neovim> for PrintCoordinates {
//     fn view(&self, ctx: &mut ViewCtx) -> impl Render<Neovim> {
//         let line: Line = *self.line.get(ctx);
//         let offset: Offset = *self.offset.get(ctx);
//         CoordinatesSnapshot { line, offset }
//     }
// }
//
// struct CoordinatesSnapshot {
//     line: Line,
//     offset: Offset,
// }
//
// impl Render<Neovim> for CoordinatesSnapshot {
//     fn render(&self, _: &mut ()) {
//         nvim_oxi::print!("({:?}, {:?})", self.line, self.offset);
//     }
// }
