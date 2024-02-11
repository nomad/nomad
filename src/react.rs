use alloc::rc::Rc;
use core::cell::RefCell;

pub struct Pond<R: Runtime> {
    rt_handle: Rc<RefCell<Option<R::Handle>>>,
}

impl<R: Runtime> Default for Pond<R> {
    #[inline(always)]
    fn default() -> Self {
        Self { rt_handle: Rc::new(RefCell::new(None)) }
    }
}

impl<R: Runtime> Clone for Pond<R> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self { rt_handle: Rc::clone(&self.rt_handle) }
    }
}

impl<R: Runtime> Pond<R> {
    #[inline]
    fn mark_as_dirty(&mut self) {
        self.with_rt_handle_mut(R::Handle::mark_as_dirty)
    }

    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    #[inline]
    pub fn pod<T>(&mut self, pod: T) -> (Out<T>, In<T>) {
        let inner = Rc::new(pod);
        let out = Out::new(Rc::clone(&inner));
        let in_ = In::new(inner);
        (out, in_)
    }

    #[inline(always)]
    fn with_rt_handle_mut<Ret, Fun>(&mut self, fun: Fun) -> Ret
    where
        Fun: FnOnce(&mut R::Handle) -> Ret,
    {
        let rt_handle = &mut *self.rt_handle.borrow_mut();
        let rt_handle = rt_handle.as_mut().unwrap();
        fun(rt_handle)
    }

    #[inline(always)]
    pub fn run(self, mut runtime: R) -> Result<R::RunOutput, R::InitError> {
        let handle = runtime.init(ReadCtx::new(self.clone()))?;
        let this_handle = &mut *self.rt_handle.borrow_mut();
        this_handle.replace(handle);
        Ok(runtime.run())
    }

    #[inline]
    pub fn with_write_ctx<'fun, Arg, Ret, Fun>(
        &mut self,
        mut fun: Fun,
    ) -> impl FnMut(Arg) -> Ret + 'fun
    where
        Fun: FnMut(Arg, &mut WriteCtx<Pond<R>>) -> Ret + 'fun,
    {
        let mut this = self.clone();
        move |args| fun(args, WriteCtx::from_mut_ref(&mut this))
    }
}

pub trait View {
    fn view<R: Runtime>(&self, pond: &mut ReadCtx<Pond<R>>) -> impl Render;
}

pub trait Render {
    fn render(&self);
}

pub struct Out<T> {
    inner: Rc<T>,
}

impl<T> Clone for Out<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self { inner: Rc::clone(&self.inner) }
    }
}

impl<T> Out<T> {
    #[inline(always)]
    pub fn get<'this, R: Runtime>(
        &'this self,
        pond: &mut ReadCtx<Pond<R>>,
    ) -> &'this T {
        pond.get(self)
    }

    #[inline(always)]
    fn new(value: Rc<T>) -> Self {
        Self { inner: value }
    }
}

pub struct In<T> {
    inner: Rc<T>,
}

impl<T> Clone for In<T> {
    #[inline(always)]
    fn clone(&self) -> Self {
        Self { inner: Rc::clone(&self.inner) }
    }
}

impl<T> In<T> {
    /// # Safety
    ///
    /// Exact same safety guarantees as `Rc::get_mut_unchecked` since this
    /// is just a wrapper around it. In particular, the caller must ensure
    /// that there are no other active references.
    #[inline(always)]
    unsafe fn get_mut_unchecked(&mut self) -> &mut T {
        &mut *(Rc::as_ptr(&self.inner) as *mut T)
    }

    #[inline(always)]
    fn new(value: Rc<T>) -> Self {
        Self { inner: value }
    }

    #[inline(always)]
    pub fn set<R: Runtime>(
        &mut self,
        new_value: T,
        pond: &mut WriteCtx<Pond<R>>,
    ) {
        *pond.get_mut(self) = new_value;
        pond.as_mut().mark_as_dirty();
    }
}

pub struct ReadCtx<T> {
    _inner: T,
}

impl<T> ReadCtx<T> {
    #[inline(always)]
    fn _as_mut(&mut self) -> &mut T {
        &mut self._inner
    }

    #[inline(always)]
    fn _from_mut_ref(inner_ref: &mut T) -> &mut Self {
        unsafe { core::mem::transmute(inner_ref) }
    }

    #[inline(always)]
    fn new(value: T) -> Self {
        Self { _inner: value }
    }
}

impl<R: Runtime> ReadCtx<Pond<R>> {
    #[inline(always)]
    pub fn get<'out, T>(&self, out: &'out Out<T>) -> &'out T {
        // TODO: track the view that's on the stack, and add `out` to its
        // dependencies.
        //
        &out.inner
    }
}

pub struct WriteCtx<T> {
    inner: T,
}

impl<T> WriteCtx<T> {
    #[inline(always)]
    fn as_mut(&mut self) -> &mut T {
        &mut self.inner
    }

    #[inline(always)]
    fn from_mut_ref(inner_ref: &mut T) -> &mut Self {
        unsafe { core::mem::transmute(inner_ref) }
    }
}

impl<R: Runtime> WriteCtx<Pond<R>> {
    #[inline]
    fn get_mut<'in_, T>(&mut self, in_: &'in_ mut In<T>) -> &'in_ mut T {
        // SAFETY: this method is the only way to access `In`'s inner value,
        // and the caller gave us a mutable reference to the `Pond`, so Rust's
        // aliasing rules guarantee that there are no other references to the
        // inner value.
        unsafe { in_.get_mut_unchecked() }
    }
}

pub trait Runtime: 'static + Sized {
    type Handle: Waker;

    type InitError;

    type RunOutput;

    fn init(
        &mut self,
        pond: ReadCtx<Pond<Self>>,
    ) -> Result<Self::Handle, Self::InitError>;

    fn run(self) -> Self::RunOutput;
}

pub trait Waker {
    fn mark_as_dirty(&mut self);
}
