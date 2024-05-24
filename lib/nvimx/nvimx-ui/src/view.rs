use crate::{Bound, Cells, Render, RequestedBound, Scene, Surface};

/// TODO: docs.
pub(crate) struct View {
    /// TODO: docs.
    root: Box<dyn Render + 'static>,

    /// TODO: docs.
    scene: Scene,

    /// TODO: docs.
    surface: Surface,
}

impl View {
    /// Opens a new `View`.
    #[inline]
    pub(crate) fn open(
        root: Box<dyn Render + 'static>,
        available_size: Bound<Cells>,
    ) -> Self {
        let mut this = Self::new(root, Scene::new(), Surface::new_hidden());
        this.render(available_size);
        this
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        self.surface._lines().collect()
    }

    #[inline]
    fn new(
        root: Box<dyn Render + 'static>,
        scene: Scene,
        surface: Surface,
    ) -> Self {
        Self { root, scene, surface }
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn render(&mut self, available_size: Bound<Cells>) {
        let requested_size = self.root.layout();

        let size = match requested_size {
            RequestedBound::Explicit(size) => size.intersect(available_size),
            RequestedBound::Available => available_size,
        };

        self.scene.resize(size);

        self.root.paint(self.scene.as_fragment());

        let diff = self.scene.diff();

        nvim_oxi::print!("diff: {diff:#?}");

        diff.apply_to(&mut self.surface);

        match (self.surface.is_hidden(), size.is_empty()) {
            (true, false) => self.surface.show(),
            (false, true) => self.surface.hide(),
            _ => {},
        }
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn _size(&self) -> Bound<Cells> {
        self.scene.size()
    }
}
