use core::any::Any;
use core::marker::PhantomData;
use core::ops::{Deref, DerefMut};
use core::panic;

use abs_path::AbsPath;
use executor::{
    BackgroundSpawner,
    BackgroundTask,
    Executor,
    LocalSpawner,
    LocalTask,
};
use futures_lite::future::{self, FutureExt};
use rand::rngs::StdRng;

use crate::context::{self, EventHandle, ResumeUnwinding, State};
use crate::editor::{AgentId, Editor};
use crate::module::{Module, Plugin, PluginId};
use crate::notify::Namespace;
use crate::{Access, AccessMut, Shared};

/// TODO: docs.
pub trait BorrowState {
    #[doc(hidden)]
    type Borrow<Ed: Editor>: Borrow<Ed>;
}

/// TODO: docs.
pub struct Context<Ed: Editor, Bs: BorrowState = NotBorrowed> {
    borrow: Bs::Borrow<Ed>,
}

/// TODO: docs.
pub struct NotBorrowed;

/// TODO: docs.
pub struct Borrowed<'a> {
    _lifetime: PhantomData<&'a ()>,
}

/// TODO: docs.
#[doc(hidden)]
pub trait Borrow<Ed: Editor>: AccessMut<State<Ed>> {
    /// TODO: docs.
    fn namespace(&self) -> &Namespace;

    /// TODO: docs.
    fn plugin_id(&self) -> PluginId;

    /// TODO: docs.
    fn state_handle(&self) -> Shared<State<Ed>>;
}

/// TODO: docs.
#[derive(cauchy::Clone)]
#[doc(hidden)]
pub struct NotBorrowedInner<Ed: Editor> {
    namespace: Namespace,
    plugin_id: PluginId,
    state_handle: Shared<State<Ed>>,
}

/// TODO: docs.
#[doc(hidden)]
pub struct BorrowedInner<'a, Ed: Editor> {
    pub(crate) namespace: &'a Namespace,
    pub(crate) plugin_id: PluginId,
    pub(crate) state_handle: &'a Shared<State<Ed>>,
    pub(crate) state: &'a mut State<Ed>,
}

impl<Ed: Editor, Bs: BorrowState> Context<Ed, Bs> {
    /// TODO: docs.
    #[inline]
    pub fn buffer_ids(
        &mut self,
    ) -> impl Iterator<Item = Ed::BufferId> + use<Ed, Bs> {
        self.with_editor(|ed| ed.buffer_ids())
    }

    /// TODO: docs.
    #[inline]
    pub fn for_each_buffer(
        &mut self,
        mut fun: impl FnMut(context::Buffer<'_, Ed>),
    ) {
        let state = self.state_handle();
        self.with_editor(move |ed| {
            ed.for_each_buffer(|inner| {
                fun(context::Buffer::new(inner, state.clone()))
            })
        });
    }

    /// TODO: docs.
    #[inline]
    pub fn fs(&mut self) -> Ed::Fs {
        self.with_editor(|ed| ed.fs())
    }

    /// TODO: docs.
    #[inline]
    pub fn on_buffer_created(
        &mut self,
        mut fun: impl FnMut(context::Buffer<'_, Ed>, AgentId) + 'static,
    ) -> EventHandle<Ed> {
        let editor = self.editor();
        let state = self.state_handle();
        let handle = self.with_editor(move |ed| {
            ed.on_buffer_created(
                move |buffer, agent_id| {
                    fun(context::Buffer::new(buffer, state.clone()), agent_id)
                },
                editor,
            )
        });
        EventHandle::new(handle, self.state_handle())
    }

    /// TODO: docs.
    #[inline]
    pub fn on_cursor_created(
        &mut self,
        mut fun: impl FnMut(context::Cursor<'_, Ed>, AgentId) + 'static,
    ) -> EventHandle<Ed> {
        let editor = self.editor();
        let state = self.state_handle();
        let handle = self.with_editor(move |ed| {
            ed.on_cursor_created(
                move |cursor, agent_id| {
                    fun(context::Cursor::new(cursor, state.clone()), agent_id)
                },
                editor,
            )
        });
        EventHandle::new(handle, self.state_handle())
    }

    /// TODO: docs.
    #[inline]
    pub fn on_selection_created(
        &mut self,
        mut fun: impl FnMut(context::Selection<'_, Ed>, AgentId) + 'static,
    ) -> EventHandle<Ed> {
        let editor = self.editor();
        let state = self.state_handle();
        let handle = self.with_editor(move |ed| {
            ed.on_selection_created(
                move |selection, agent_id| {
                    fun(
                        context::Selection::new(selection, state.clone()),
                        agent_id,
                    )
                },
                editor,
            )
        });
        EventHandle::new(handle, self.state_handle())
    }

    /// TODO: docs.
    #[inline]
    pub fn namespace(&self) -> &Namespace {
        self.borrow.namespace()
    }

    /// TODO: docs.
    #[inline]
    pub fn new_agent_id(&mut self) -> AgentId {
        self.borrow.with_mut(|state| state.next_agent_id())
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_background<Fut>(
        &mut self,
        fut: Fut,
    ) -> BackgroundTask<Fut::Output, Ed::Executor>
    where
        Fut: Future + Send + 'static,
        Fut::Output: Send + 'static,
    {
        BackgroundTask::new(self.with_editor(move |ed| {
            ed.executor().background_spawner().spawn(fut)
        }))
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_editor<T>(&mut self, fun: impl FnOnce(&mut Ed) -> T) -> T {
        self.borrow.with_mut(move |state| fun(state))
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn with_rng<T>(&mut self, fun: impl FnOnce(&mut StdRng) -> T) -> T {
        self.borrow.with_mut(move |state| fun(state.rng()))
    }

    #[inline]
    pub(crate) fn new(borrow: Bs::Borrow<Ed>) -> Self {
        Self { borrow }
    }

    #[inline]
    pub(crate) fn plugin_id(&self) -> PluginId {
        self.borrow.plugin_id()
    }

    #[inline]
    pub(crate) fn state_handle(&self) -> Shared<State<Ed>> {
        self.borrow.state_handle()
    }

    #[inline]
    fn editor(&self) -> impl AccessMut<Ed> + Clone + 'static {
        self.state_handle().map_mut(Deref::deref, DerefMut::deref_mut)
    }

    #[inline]
    fn spawn_local_inner<T: 'static>(
        &mut self,
        fun: impl AsyncFnOnce(&mut Context<Ed>) -> T + 'static,
    ) -> LocalTask<Option<T>, Ed::Executor> {
        self.spawn_local_unprotected_inner(async move |ctx| {
            match panic::AssertUnwindSafe(fun(ctx)).catch_unwind().await {
                Ok(ret) => Some(ret),
                Err(payload) => {
                    ctx.with_borrowed(|ctx| ctx.handle_panic(payload));
                    None
                },
            }
        })
    }

    #[inline]
    fn spawn_local_unprotected_inner<T: 'static>(
        &mut self,
        fun: impl AsyncFnOnce(&mut Context<Ed>) -> T + 'static,
    ) -> LocalTask<T, Ed::Executor> {
        let mut ctx = Context::new(NotBorrowedInner {
            namespace: self.namespace().clone(),
            plugin_id: self.plugin_id(),
            state_handle: self.state_handle(),
        });
        LocalTask::new(self.with_editor(move |ed| {
            ed.executor().local_spawner().spawn(async move {
                // Yielding prevents a panic that would occur when:
                //
                // - the local executor immediately polls the future when a new
                //   task is spawned, and
                // - Context::with_borrowed() is called before the first .await
                //   point is reached
                //
                // In that case, with_borrowed() would panic because the state
                // is already mutably borrowed by Self.
                //
                // Yielding guarantees that by the time with_borrowed() is
                // called, the synchronous code containing Self will have
                // already finished running.
                future::yield_now().await;

                fun(&mut ctx).await
            })
        }))
    }
}

impl<Ed: Editor, Bs: BorrowState> Context<Ed, Bs>
where
    Self: AccessMut<Ed>,
{
    /// TODO: docs.
    #[inline]
    pub async fn create_buffer(
        &mut self,
        file_path: &AbsPath,
        agent_id: AgentId,
    ) -> Result<Ed::BufferId, Ed::CreateBufferError> {
        Ed::create_buffer(self, file_path, agent_id).await
    }
}

impl<Ed: Editor, Bs: BorrowState> Context<Ed, Bs>
where
    Bs::Borrow<Ed>: DerefMut<Target = State<Ed>>,
{
    /// TODO: docs.
    #[inline]
    pub fn buffer(
        &mut self,
        buffer_id: Ed::BufferId,
    ) -> Option<context::Buffer<'_, Ed>> {
        let state_handle = self.state_handle();
        Ed::buffer(self, buffer_id)
            .map(|buffer| context::Buffer::new(buffer, state_handle))
    }

    /// TODO: docs.
    #[inline]
    pub fn buffer_at_path(
        &mut self,
        path: &AbsPath,
    ) -> Option<context::Buffer<'_, Ed>> {
        let state_handle = self.state_handle();
        Ed::buffer_at_path(self, path)
            .map(|buffer| context::Buffer::new(buffer, state_handle))
    }

    /// TODO: docs.
    #[inline]
    pub fn current_buffer(&mut self) -> Option<context::Buffer<'_, Ed>> {
        let state_handle = self.state_handle();
        Ed::current_buffer(self)
            .map(|buffer| context::Buffer::new(buffer, state_handle))
    }

    /// TODO: docs.
    #[inline]
    pub fn cursor(
        &mut self,
        cursor_id: Ed::CursorId,
    ) -> Option<context::Cursor<'_, Ed>> {
        let state_handle = self.state_handle();
        Ed::cursor(self, cursor_id)
            .map(|cursor| context::Cursor::new(cursor, state_handle))
    }

    /// TODO: docs.
    #[track_caller]
    #[inline]
    pub fn module<M>(&self) -> &M
    where
        M: Module<Ed>,
    {
        match self.try_module::<M>() {
            Some(module) => module,
            None => panic!("module {:?} not found", M::NAME),
        }
    }

    /// TODO: docs.
    #[inline]
    pub fn selection(
        &mut self,
        selection_id: Ed::SelectionId,
    ) -> Option<context::Selection<'_, Ed>> {
        let state_handle = self.state_handle();
        Ed::selection(self, selection_id)
            .map(|selection| context::Selection::new(selection, state_handle))
    }

    /// TODO: docs.
    #[inline]
    pub fn try_module<M>(&self) -> Option<&M>
    where
        M: Module<Ed>,
    {
        self.borrow.get_module::<M>()
    }
}

impl<Ed: Editor> Context<Ed, NotBorrowed> {
    /// TODO: docs.
    #[inline]
    pub fn block_on<T>(&mut self, fun: impl AsyncFnOnce(&mut Self) -> T) -> T {
        let mut this = Self { borrow: self.borrow.clone() };
        let future = async move { fun(&mut this).await };
        future::block_on(self.with_editor(|ed| ed.executor().run(future)))
    }

    /// TODO: docs.
    #[inline]
    pub async fn run<T>(
        &mut self,
        fun: impl AsyncFnOnce(&mut Self) -> T,
    ) -> T {
        let mut this = Self { borrow: self.borrow.clone() };
        let future = async move { fun(&mut this).await };
        self.with_editor(|ed| ed.executor().run(future)).await
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_local<T: 'static>(
        &mut self,
        fun: impl AsyncFnOnce(&mut Self) -> T + 'static,
    ) -> LocalTask<Option<T>, Ed::Executor> {
        self.spawn_local_inner(fun)
    }

    /// TODO: docs.
    #[inline]
    pub fn spawn_local_unprotected<T: 'static>(
        &mut self,
        fun: impl AsyncFnOnce(&mut Self) -> T + 'static,
    ) -> LocalTask<T, Ed::Executor> {
        self.spawn_local_unprotected_inner(fun)
    }

    /// TODO: docs.
    #[inline]
    pub fn with_borrowed<T>(
        &self,
        fun: impl FnOnce(&mut Context<Ed, Borrowed<'_>>) -> T,
    ) -> T {
        self.borrow.state_handle.with_mut(|state| {
            let mut ctx = Context::new(BorrowedInner {
                namespace: self.namespace(),
                plugin_id: self.plugin_id(),
                state_handle: &self.borrow.state_handle,
                state,
            });
            fun(&mut ctx)
        })
    }

    /// TODO: docs.
    #[inline]
    pub(crate) fn from_editor(editor: Ed) -> Self {
        Self::new(NotBorrowedInner {
            namespace: Namespace::default(),
            plugin_id: <ResumeUnwinding as Plugin<Ed>>::id(),
            state_handle: Shared::new(State::new(editor)),
        })
    }
}

impl<Ed: Editor> Context<Ed, Borrowed<'_>> {
    /// TODO: docs.
    #[inline]
    pub fn spawn_and_detach(
        &mut self,
        fun: impl AsyncFnOnce(&mut Context<Ed>) + 'static,
    ) {
        self.spawn_local_inner(async move |ctx| fun(ctx).await).detach();
    }

    #[inline]
    pub(crate) fn state_mut(&mut self) -> &mut State<Ed> {
        self.borrow.state
    }

    #[inline]
    fn handle_panic(&mut self, panic_payload: Box<dyn Any + Send>) {
        State::handle_panic(panic_payload, self);
    }
}

impl<Ed: Editor, Bs: BorrowState> Deref for Context<Ed, Bs>
where
    Bs::Borrow<Ed>: Deref<Target = State<Ed>>,
{
    type Target = Ed;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.borrow.deref()
    }
}

impl<Ed: Editor, Bs: BorrowState> DerefMut for Context<Ed, Bs>
where
    Bs::Borrow<Ed>: DerefMut<Target = State<Ed>>,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.borrow.deref_mut()
    }
}

impl<Ed: Editor> Access<Ed> for Context<Ed, NotBorrowed> {
    #[track_caller]
    #[inline]
    fn with<T>(&self, f: impl FnOnce(&Ed) -> T) -> T {
        self.borrow.with(move |state| f(state))
    }
}

impl<Ed: Editor> AccessMut<Ed> for Context<Ed, NotBorrowed> {
    #[track_caller]
    #[inline]
    fn with_mut<T>(&mut self, f: impl FnOnce(&mut Ed) -> T) -> T {
        self.borrow.with_mut(move |state| f(state))
    }
}

impl<Ed: Editor, Bs: BorrowState> Access<Ed> for &Context<Ed, Bs>
where
    Context<Ed, Bs>: Access<Ed>,
{
    #[track_caller]
    #[inline]
    fn with<T>(&self, f: impl FnOnce(&Ed) -> T) -> T {
        (**self).with(f)
    }
}

impl<Ed: Editor, Bs: BorrowState> Access<Ed> for &mut Context<Ed, Bs>
where
    Context<Ed, Bs>: Access<Ed>,
{
    #[track_caller]
    #[inline]
    fn with<T>(&self, f: impl FnOnce(&Ed) -> T) -> T {
        (**self).with(f)
    }
}

impl<Ed: Editor, Bs: BorrowState> AccessMut<Ed> for &mut Context<Ed, Bs>
where
    Context<Ed, Bs>: AccessMut<Ed>,
{
    #[track_caller]
    #[inline]
    fn with_mut<T>(&mut self, f: impl FnOnce(&mut Ed) -> T) -> T {
        (**self).with_mut(f)
    }
}

impl BorrowState for NotBorrowed {
    type Borrow<Ed: Editor> = NotBorrowedInner<Ed>;
}

impl<'a> BorrowState for Borrowed<'a> {
    type Borrow<Ed: Editor> = BorrowedInner<'a, Ed>;
}

impl<Ed: Editor> Access<State<Ed>> for NotBorrowedInner<Ed> {
    #[track_caller]
    #[inline]
    fn with<T>(&self, f: impl FnOnce(&State<Ed>) -> T) -> T {
        self.state_handle.with(f)
    }
}

impl<Ed: Editor> AccessMut<State<Ed>> for NotBorrowedInner<Ed> {
    #[track_caller]
    #[inline]
    fn with_mut<T>(&mut self, f: impl FnOnce(&mut State<Ed>) -> T) -> T {
        self.state_handle.with_mut(f)
    }
}

impl<Ed: Editor> Borrow<Ed> for NotBorrowedInner<Ed> {
    #[inline]
    fn namespace(&self) -> &Namespace {
        &self.namespace
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }

    #[inline]
    fn state_handle(&self) -> Shared<State<Ed>> {
        self.state_handle.clone()
    }
}

impl<Ed: Editor> Borrow<Ed> for BorrowedInner<'_, Ed> {
    #[inline]
    fn namespace(&self) -> &Namespace {
        self.namespace
    }

    #[inline]
    fn plugin_id(&self) -> PluginId {
        self.plugin_id
    }

    #[inline]
    fn state_handle(&self) -> Shared<State<Ed>> {
        self.state_handle.clone()
    }
}

impl<Ed: Editor> Deref for BorrowedInner<'_, Ed> {
    type Target = State<Ed>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.state
    }
}

impl<Ed: Editor> DerefMut for BorrowedInner<'_, Ed> {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.state
    }
}
