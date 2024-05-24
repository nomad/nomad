/// TODO: docs
pub trait React {
    /// TODO: docs
    type Value;
}

/// TODO: docs
pub struct Reactive<T> {
    _value: T,
}

impl<T> React for Reactive<T> {
    type Value = T;
}
