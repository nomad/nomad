use std::cmp::Ordering;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use common::nvim::{self, api::opts::*};
use common::*;

use crate::*;

pub type ViewId = nvim::api::Window;

/// TODO: docs.
pub(crate) struct View {
    /// TODO: docs.
    files: Files,

    /// TODO: docs.
    previewer: Option<Previewer>,

    /// TODO: docs.
    prompt: Prompt,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("the path has no parent directory")]
    NoParentDir,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

/// TODO: docs.
struct File {
    /// TODO: docs.
    path: PathBuf,

    /// TODO: docs.
    file_name: OsString,
}

impl View {
    /// TODO: docs.
    pub fn close(self) {
        let force_close = true;
        let buf_delete_opts = BufDeleteOpts::builder().force(true).build();
        self.prompt.close(force_close, &buf_delete_opts);
        self.files.close(force_close, &buf_delete_opts);
    }

    fn focus(&mut self, file_idx: usize) {
        self.files.focus(file_idx);
        let file = self.files.file(file_idx);
        self.prompt.focus(file);
        if let Some(previewer) = &mut self.previewer {
            previewer.focus(file);
        }
    }

    /// TODO: docs.
    pub fn id(&self) -> ViewId {
        (*self.files.window).clone()
    }

    /// TODO: docs.
    pub fn new(
        at_path: PathBuf,
        with_config: &WindowConfig,
    ) -> Result<Self, Error> {
        let is_file_path = at_path.is_file();

        let dir = if is_file_path {
            at_path.parent().ok_or(Error::NoParentDir)?
        } else {
            &at_path
        };

        let siblings = children(dir).map(|mut siblings| {
            siblings.sort_by(sort_first_by_directory_then_by_name);
            siblings
        })?;

        let focused_file_idx = if is_file_path {
            siblings.iter().position(|file| file.path == at_path)
        } else {
            None
        }
        .unwrap_or(0);

        let files = Files::new(siblings);

        let prompt = Prompt::new(dir.to_owned(), files.file(focused_file_idx));

        let mut this = Self {
            files,
            prompt,
            previewer: Some(Previewer::new(preview_file_name)),
        };

        this.open(with_config);

        this.focus(focused_file_idx);

        Ok(this)
    }

    fn open(&mut self, config: &WindowConfig) {
        let (prompt_config, files_config) =
            config.bisect(Axis::Vertical, ScreenUnit::Cells(1));
        self.prompt.open(prompt_config);
        self.files.open(files_config);
    }
}

/// TODO: docs
struct Prompt {
    dir: PathBuf,
    path_start: usize,
    path_end: usize,
    buffer: nvim::api::Buffer,
    window: LateInit<nvim::api::Window>,
    window_config: LateInit<WindowConfig>,
}

impl Prompt {
    fn close(self, force_close: bool, opts: &BufDeleteOpts) {
        self.window.into_inner().close(force_close).unwrap();
        self.buffer.delete(opts).unwrap();
    }

    fn focus(&mut self, file: &File) {
        let new_path = file.file_name.to_string_lossy();

        // self.buffer
        //     .set_text(
        //         0..0,
        //         self.path_start,
        //         self.path_end,
        //         std::iter::once(new_path.as_ref()),
        //     )
        //     .unwrap();
        //
        // self.path_end = self.path_start + new_path.len();
    }

    /// TODO: docs
    fn new(dir: PathBuf, file: &File) -> Self {
        let mut buffer = nvim::api::create_buf(false, true).unwrap();

        let dir_len;
        let path_len;

        let prompt_line = {
            let mut s = String::new();
            s.push(' ');
            s.push_str(dir.to_string_lossy().as_ref());
            s.push(std::path::MAIN_SEPARATOR);
            dir_len = s.len();
            let file_name = file.file_name.to_string_lossy();
            path_len = file_name.len();
            s.push_str(file_name.as_ref());
            s
        };

        buffer
            .set_lines(0..0, true, std::iter::once(prompt_line.as_str()))
            .unwrap();

        Self {
            dir,
            path_start: dir_len,
            path_end: dir_len + path_len,
            buffer,
            window: LateInit::default(),
            window_config: LateInit::default(),
        }
    }

    /// TODO: docs
    fn open(&mut self, config: WindowConfig) {
        let window_config = (&config).into();

        let window = nvim::api::open_win(&self.buffer, true, &window_config)
            .expect("the config is valid");

        self.window.init(window);
        self.window_config.init(config);
    }
}

/// TODO: docs
struct Files {
    files: Vec<File>,
    buffer: nvim::api::Buffer,
    window: LateInit<nvim::api::Window>,
    window_config: LateInit<WindowConfig>,
}

impl Files {
    fn close(self, force_close: bool, opts: &BufDeleteOpts) {
        self.window.into_inner().close(force_close).unwrap();
        self.buffer.delete(opts).unwrap();
    }

    fn file(&self, idx: usize) -> &File {
        &self.files[idx]
    }

    fn focus(&mut self, idx: usize) {
        self.window.set_cursor(idx + 1, 0).unwrap();
    }

    /// TODO: docs
    fn new(files: Vec<File>) -> Self {
        let mut buffer = nvim::api::create_buf(false, true).unwrap();

        buffer
            .set_lines(
                0..0,
                true,
                files
                    .iter()
                    .map(|file| file.file_name.to_string_lossy().into_owned())
                    .map(|path| nvim::String::from(path.as_str())),
            )
            .unwrap();

        Self {
            buffer,
            files,
            window: LateInit::default(),
            window_config: LateInit::default(),
        }
    }

    /// TODO: docs
    fn open(&mut self, config: WindowConfig) {
        let window_config = (&config).into();

        let mut window =
            nvim::api::open_win(&self.buffer, true, &window_config)
                .expect("the config is valid");

        window.set_option("cursorline", true).unwrap();

        self.window.init(window);
        self.window_config.init(config);
    }
}

/// TODO: docs
struct Previewer {
    previewer: Box<dyn Fn(&File) -> String + 'static>,
}

impl Previewer {
    fn focus(&mut self, _file: &File) {}

    fn new<P>(previewer: P) -> Self
    where
        P: Fn(&File) -> String + 'static,
    {
        Self { previewer: Box::new(previewer) }
    }
}

/// TODO: docs
fn children(parent: &Path) -> Result<Vec<File>, Error> {
    let siblings = std::fs::read_dir(parent)?;
    let mut files = Vec::new();
    for file in siblings {
        let file = file?;
        let path = file.path();
        let file_name = file.file_name();
        files.push(File { file_name, path });
    }
    Ok(files)
}

/// TODO: docs
fn preview_file_name(file: &File) -> String {
    file.file_name.to_string_lossy().into_owned()
}

/// TODO: docs
fn sort_first_by_directory_then_by_name(a: &File, b: &File) -> Ordering {
    match (a.path.is_dir(), b.path.is_dir()) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.path.cmp(&b.path),
    }
}
