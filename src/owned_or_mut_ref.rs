use std::ops::{Deref, DerefMut};

/// Holds an owned `T` or a mutable reference.
#[derive(Debug)]
pub(crate) enum OwnedOrMutRef<'r, T> {
    /// Holds an owned `T`.
    Owned(T),
    /// Holds a mutable reference to `T`.
    MutRef(&'r mut T),
}

impl<'r, T> Deref for OwnedOrMutRef<'r, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            OwnedOrMutRef::Owned(t) => t,
            OwnedOrMutRef::MutRef(t) => t,
        }
    }
}

impl<'r, T> DerefMut for OwnedOrMutRef<'r, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            OwnedOrMutRef::Owned(t) => t,
            OwnedOrMutRef::MutRef(t) => t,
        }
    }
}

impl<'r, T> From<T> for OwnedOrMutRef<'r, T> {
    fn from(v: T) -> Self {
        Self::Owned(v)
    }
}

impl<'r, T> From<&'r mut T> for OwnedOrMutRef<'r, T> {
    fn from(v: &'r mut T) -> Self {
        Self::MutRef(v)
    }
}
