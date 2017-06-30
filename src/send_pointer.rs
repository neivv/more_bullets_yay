use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// A pointer wrapper which implements `Send`.
pub struct SendPtr<T>(pub *mut T);

impl<T> Clone for SendPtr<T> {
    fn clone(&self) -> SendPtr<T> {
        SendPtr(self.0)
    }
}

impl<T> Copy for SendPtr<T> {}

impl<T> PartialEq for SendPtr<T> {
    fn eq(&self, other: &SendPtr<T>) -> bool {
        self.0 == other.0
    }
}
impl<T> Eq for SendPtr<T> {}

impl<T> PartialEq<*mut T> for SendPtr<T> {
    fn eq(&self, other: &*mut T) -> bool {
        self.0 == *other
    }
}

impl<T> Hash for SendPtr<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

unsafe impl<T> Send for SendPtr<T> {}

impl<T> Deref for SendPtr<T> {
    type Target = *mut T;
    fn deref(&self) -> &*mut T {
        &self.0
    }
}

impl<T> From<*mut T> for SendPtr<T> {
    fn from(val: *mut T) -> SendPtr<T> {
        SendPtr(val)
    }
}
