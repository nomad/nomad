extern crate alloc;

use core::convert::Infallible;

use api::opts::*;
use nvim_oxi::api;

pub mod react;

use react::{Out, Pond, ReadCtx, Render, Runtime, View, Waker};

#[nvim_oxi::module]
fn nomad() -> nvim_oxi::Result<()> {
    let mut pond = Pond::new();

    let (line, mut line_in) = pond.pod(Line::default());

    let (offset, mut offset_in) = pond.pod(Offset::default());

    let on_cursor_moved = pond.with_write_ctx(move |_args, pond| {
        let (line, offset) = api::Window::current().get_cursor()?;
        line_in.set(Line(line), pond);
        offset_in.set(Offset(offset), pond);
        Ok::<_, nvim_oxi::Error>(false)
    });

    api::create_autocmd(
        ["CursorMoved", "CursorMovedI"],
        &CreateAutocmdOpts::builder().callback(on_cursor_moved).build(),
    )?;

    let coordinates = PrintCoordinates { line, offset };

    let runtime = Neovim::new(coordinates);

    pond.run(runtime)?;

    Ok(())
}

struct Neovim<V> {
    root_view: Option<V>,
}

impl<V> Neovim<V> {
    #[inline(always)]
    fn new(root_view: V) -> Self {
        Self { root_view: Some(root_view) }
    }
}

impl<V> Runtime for Neovim<V>
where
    V: View + 'static,
{
    type Handle = NeovimHandle;

    type InitError = nvim_oxi::Error;

    type RunOutput = ();

    fn init(
        &mut self,
        mut pond: ReadCtx<Pond<Self>>,
    ) -> Result<Self::Handle, Self::InitError> {
        use nvim_oxi::libuv::AsyncHandle;

        let root_view = self.root_view.take().unwrap();

        let async_handle = AsyncHandle::new(move || {
            let render = root_view.view(&mut pond);
            nvim_oxi::schedule(move |_| {
                render.render();
                Ok(())
            });
            Ok::<_, Infallible>(())
        })?;

        let handle = NeovimHandle { async_handle };

        Ok(handle)
    }

    fn run(self) -> Self::RunOutput {}
}

#[derive(Clone)]
struct NeovimHandle {
    async_handle: nvim_oxi::libuv::AsyncHandle,
}

impl Waker for NeovimHandle {
    #[inline(always)]
    fn mark_as_dirty(&mut self) {
        self.async_handle.send().unwrap();
    }
}

#[derive(Clone, Copy, Default)]
struct Line(usize);

impl core::fmt::Display for Line {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

#[derive(Clone, Copy, Default)]
struct Offset(usize);

impl core::fmt::Display for Offset {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(f)
    }
}

struct PrintCoordinates {
    line: Out<Line>,
    offset: Out<Offset>,
}

impl View for PrintCoordinates {
    fn view<R: Runtime>(
        &self,
        pond: &mut ReadCtx<Pond<R>>,
    ) -> impl Render + 'static {
        let line: Line = *self.line.get(pond);
        let offset: Offset = *self.offset.get(pond);
        CoordinatesSnapshot::new(line, offset)
    }
}

struct CoordinatesSnapshot {
    line: Line,
    offset: Offset,
}

impl CoordinatesSnapshot {
    fn new(line: Line, offset: Offset) -> Self {
        Self { line, offset }
    }
}

impl Render for CoordinatesSnapshot {
    fn render(&self) {
        nvim_oxi::print!("({}, {})", self.line, self.offset);
    }
}
