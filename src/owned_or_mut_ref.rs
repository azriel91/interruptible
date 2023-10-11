use std::ops::{Deref, DerefMut};

/// Holds an owned `T` or a mutable reference.
#[derive(Debug, PartialEq, Eq)]
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

#[cfg(test)]
mod tests {
    use std::ops::{Deref, DerefMut};

    use super::OwnedOrMutRef;

    #[test]
    fn debug() {
        let owned_or_mut_ref = OwnedOrMutRef::Owned(123);

        assert_eq!("Owned(123)", format!("{owned_or_mut_ref:?}"));
    }

    #[test]
    fn deref() {
        let owned = OwnedOrMutRef::Owned(123);
        let mut n = 123;
        let mut_ref = OwnedOrMutRef::MutRef(&mut n);

        assert_eq!(&123, Deref::deref(&owned));
        assert_eq!(&123, Deref::deref(&mut_ref));
    }

    #[test]
    fn deref_mut() {
        let mut owned = OwnedOrMutRef::Owned(123);
        let mut n = 123;
        let mut mut_ref = OwnedOrMutRef::MutRef(&mut n);

        assert_eq!(&mut 123, DerefMut::deref_mut(&mut owned));
        assert_eq!(&mut 123, DerefMut::deref_mut(&mut mut_ref));
    }

    #[test]
    fn from_t() {
        let owned = OwnedOrMutRef::from(123);

        assert_eq!(OwnedOrMutRef::Owned(123), owned);
    }

    #[test]
    fn from_mut_ref() {
        let mut n = 123;
        let mut_ref_n = OwnedOrMutRef::<u32>::from(&mut n);
        let mut m = 123;
        let mut_ref_m = OwnedOrMutRef::MutRef(&mut m);

        assert_eq!(mut_ref_n, mut_ref_m);
    }
}
