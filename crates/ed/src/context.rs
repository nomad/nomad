// State{Handle,Mut} is used in:
//
// - api_ctx;
// - command_builder;
// - plugin;
//
// Methods to re-implement:
//
// - block_on()
// - run()
// - spawn_and_block_on()
// - spawn_background()
// - spawn_local()
// - spawn_local_unprotected()

use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};

use abs_path::AbsPath;

use crate::Shared;
use crate::backend::{AgentId, Backend, Buffer};
use crate::module::Module;
use crate::notify::{self, Emitter, Namespace, NotificationId};
use crate::plugin::{Plugin, PluginId};
use crate::state::State;

/// TODO: docs.
pub trait BorrowState {
    #[doc(hidden)]
    type Borrow<Ed: Backend>: Borrow<Ed>;
}

/// TODO: docs.
pub struct Context<Ed: Backend, B: BorrowState = NotBorrowed> {
    borrow: B::Borrow<Ed>,
}

/// TODO: docs.
pub struct NotBorrowed;

/// TODO: docs.
pub struct Borrowed<'a> {
    _lifetime: PhantomData<&'a ()>,
}

/// TODO: docs.
#[doc(hidden)]
pub trait Borrow<Ed: Backend> {
    /// TODO: docs.
    fn namespace(&self) -> &Namespace;

    /// TODO: docs.
    fn plugin_id(&self) -> PluginId;

    /// TODO: docs.
    fn with_state<T>(&mut self, f: impl FnOnce(&mut State<Ed>) -> T) -> T;
}

/// TODO: docs.
#[doc(hidden)]
pub struct NotBorrowedInner<Ed: Backend> {
    namespace: Namespace,
    plugin_id: PluginId,
    state_handle: Shared<State<Ed>>,
}

/// TODO: docs.
#[doc(hidden)]
pub struct BorrowedInner<'a, Ed: Backend> {
    namespace: &'a Namespace,
    plugin_id: PluginId,
    state_handle: &'a Shared<State<Ed>>,
    state: &'a mut State<Ed>,
}

impl<Ed: Backend, B: BorrowState> Context<Ed, B> {
    /// TODO: docs.
    #[inline]
    pub async fn create_and_focus(
        &mut self,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> Result<Ed::BufferId, Ed::CreateBufferError> {
        let buffer_id = self.create_buffer(file_path, agent_id).await?;
        self.with_editor(|ed| {
            if let Some(mut buffer) = ed.buffer(buffer_id.clone()) {
                buffer.focus()
            }
        });
        Ok(buffer_id)
    }

    /// TODO: docs.
    #[inline]
    pub async fn create_buffer(
        &mut self,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> Result<Ed::BufferId, Ed::CreateBufferError> {
        Ed::create_buffer(file_path, agent_id, self).await
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_error(&mut self, message: notify::Message) -> NotificationId {
        self.emit_message(notify::Level::Error, message)
    }

    /// TODO: docs.
    #[inline]
    pub fn emit_info(&mut self, message: notify::Message) -> NotificationId {
        self.emit_message(notify::Level::Info, message)
    }

    /// TODO: docs.
    #[inline]
    pub fn for_each_buffer(&mut self, fun: impl FnMut(Ed::Buffer<'_>)) {
        self.with_editor(move |ed| ed.for_each_buffer(fun));
    }

    /// TODO: docs.
    #[inline]
    pub fn fs(&mut self) -> Ed::Fs {
        self.with_editor(|ed| ed.fs())
    }

    /// TODO: docs.
    #[inline]
    pub fn on_buffer_created<Fun>(&mut self, fun: Fun) -> Ed::EventHandle
    where
        Fun: FnMut(&Ed::Buffer<'_>, AgentId) + 'static,
    {
        self.with_editor(move |ed| ed.on_buffer_created(fun))
    }

    /// TODO: docs.
    #[inline]
    pub fn on_cursor_created<Fun>(&mut self, fun: Fun) -> Ed::EventHandle
    where
        Fun: FnMut(&Ed::Cursor<'_>, AgentId) + 'static,
    {
        self.with_editor(move |ed| ed.on_cursor_created(fun))
    }

    /// TODO: docs.
    #[inline]
    pub fn on_selection_created<Fun>(&mut self, fun: Fun) -> Ed::EventHandle
    where
        Fun: FnMut(&Ed::Selection<'_>, AgentId) + 'static,
    {
        self.with_editor(move |ed| ed.on_selection_created(fun))
    }

    /// TODO: docs.
    #[inline]
    pub fn new_agent_id(&mut self) -> AgentId {
        self.borrow.with_state(|state| state.next_agent_id())
    }

    /// TODO: docs.
    #[inline]
    pub fn with_editor<T>(&mut self, fun: impl FnOnce(&mut Ed) -> T) -> T {
        self.borrow.with_state(move |state| fun(state))
    }

    #[inline]
    pub(crate) fn emit_err<Err>(&mut self, err: Err) -> NotificationId
    where
        Err: notify::Error,
    {
        let namespace = self.namespace().clone();
        self.with_editor(move |ed| ed.emit_err(&namespace, err))
    }

    #[inline]
    pub(crate) fn emit_message(
        &mut self,
        level: notify::Level,
        message: notify::Message,
    ) -> NotificationId {
        let namespace = self.namespace().clone();
        self.with_editor(move |ed| {
            ed.emitter().emit(notify::Notification {
                level,
                namespace: &namespace,
                message,
                updates_prev: None,
            })
        })
    }

    #[inline]
    fn namespace(&self) -> &Namespace {
        self.borrow.namespace()
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.borrow.plugin_id()
    }
}

impl<Ed: Backend, B: BorrowState> Context<Ed, B>
where
    B::Borrow<Ed>: DerefMut<Target = State<Ed>>,
{
    /// TODO: docs.
    #[inline]
    pub fn buffer(
        &mut self,
        buffer_id: Ed::BufferId,
    ) -> Option<Ed::Buffer<'_>> {
        Ed::buffer(self, buffer_id)
    }

    /// TODO: docs.
    #[inline]
    pub fn buffer_at_path(
        &mut self,
        path: &AbsPath,
    ) -> Option<Ed::Buffer<'_>> {
        Ed::buffer_at_path(self, path)
    }

    /// TODO: docs.
    #[inline]
    pub fn current_buffer(&mut self) -> Option<Ed::Buffer<'_>> {
        Ed::current_buffer(self)
    }

    /// TODO: docs.
    #[inline]
    pub fn cursor(
        &mut self,
        cursor_id: Ed::CursorId,
    ) -> Option<Ed::Cursor<'_>> {
        Ed::cursor(self, cursor_id)
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn get_module<M>(&self) -> &M
    where
        M: Module<Ed>,
    {
        match self.try_get_module::<M>() {
            Some(module) => module,
            None => panic!("module {:?} not found", M::NAME),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn selection(
        &mut self,
        selection_id: Ed::SelectionId,
    ) -> Option<Ed::Selection<'_>> {
        Ed::selection(self, selection_id)
    }

    /// TODO: docs.
    #[inline]
    pub fn try_get_module<M>(&self) -> Option<&M>
    where
        M: Module<Ed>,
    {
        self.borrow.get_module::<M>()
    }
}

impl<Ed: Backend> Context<Ed, NotBorrowed> {
    /// TODO: docs.
    #[inline]
    pub fn with_mut<T>(
        &self,
        fun: impl FnOnce(&mut Context<Ed, Borrowed<'_>>) -> T,
    ) -> T {
        self.borrow.state_handle.with_mut(|state| {
            let mut ctx = Context {
                borrow: BorrowedInner {
                    namespace: self.namespace(),
                    plugin_id: self.plugin_id(),
                    state_handle: &self.borrow.state_handle,
                    state,
                },
            };
            fun(&mut ctx)
        })
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn from_editor(editor: Ed) -> Self {
        Self {
            borrow: NotBorrowedInner {
                namespace: Namespace::default(),
                plugin_id: <crate::state::ResumeUnwinding as Plugin<Ed>>::id(),
                state_handle: Shared::new(State::new(editor)),
            },
        }
    }
}

impl<Ed: Backend, B: BorrowState> Deref for Context<Ed, B>
where
    B::Borrow<Ed>: Deref<Target = State<Ed>>,
{
    type Target = Ed;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.borrow.deref()
    }
}

impl<Ed: Backend, B: BorrowState> DerefMut for Context<Ed, B>
where
    B::Borrow<Ed>: DerefMut<Target = State<Ed>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow.deref_mut()
    }
}

impl BorrowState for NotBorrowed {
    type Borrow<Ed: Backend> = NotBorrowedInner<Ed>;
}

impl<'a> BorrowState for Borrowed<'a> {
    type Borrow<Ed: Backend> = BorrowedInner<'a, Ed>;
}

impl<Ed: Backend> Borrow<Ed> for NotBorrowedInner<Ed> {
    #[inline]
    fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }

    #[inline]
    fn with_state<T>(&mut self, f: impl FnOnce(&mut State<Ed>) -> T) -> T {
        self.state_handle.with_mut(f)
    }
}

impl<Ed: Backend> Borrow<Ed> for BorrowedInner<'_, Ed> {
    #[inline]
    fn namespace(&self) -> &Namespace {
        self.namespace
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }

    #[inline]
    fn with_state<T>(&mut self, f: impl FnOnce(&mut State<Ed>) -> T) -> T {
        f(self.state)
    }
}

impl<Ed: Backend> Deref for BorrowedInner<'_, Ed> {
    type Target = State<Ed>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<Ed: Backend> DerefMut for BorrowedInner<'_, Ed> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}
