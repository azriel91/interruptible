use std::ops::Deref;

/// Holds an owned `T` or an immutable reference.
#[derive(Debug, PartialEq, Eq)]
pub enum OwnedOrRef<'r, T> {
    /// Holds an owned `T`.
    Owned(T),
    /// Holds an immutable reference to `T`.
    Ref(&'r T),
}

impl<'r, T> OwnedOrRef<'r, T> {
    /// Reborrows this `OwnedOrRef` with a shorter lifetime.
    ///
    /// For an `Owned` variant, this borrows it as a `Ref`.
    pub fn reborrow(&self) -> OwnedOrRef<'_, T> {
        match self {
            OwnedOrRef::Owned(ref t) => OwnedOrRef::Ref(t),
            OwnedOrRef::Ref(t) => OwnedOrRef::Ref(t),
        }
    }
}

impl<'r, T> Deref for OwnedOrRef<'r, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            OwnedOrRef::Owned(t) => t,
            OwnedOrRef::Ref(t) => t,
        }
    }
}

impl<'r, T> From<T> for OwnedOrRef<'r, T> {
    fn from(v: T) -> Self {
        Self::Owned(v)
    }
}

impl<'r, T> From<&'r T> for OwnedOrRef<'r, T> {
    fn from(v: &'r T) -> Self {
        Self::Ref(v)
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Deref;

    use super::OwnedOrRef;

    #[test]
    fn debug() {
        let owned_or_ref = OwnedOrRef::Owned(123);

        assert_eq!("Owned(123)", format!("{owned_or_ref:?}"));
    }

    #[test]
    fn reborrow() {
        let owned = OwnedOrRef::Owned(123);
        let n = 123;
        let r#ref = OwnedOrRef::Ref(&n);

        assert_eq!(OwnedOrRef::Ref(&123), owned.reborrow());
        assert_eq!(OwnedOrRef::Ref(&123), r#ref.reborrow());
    }

    #[test]
    fn deref() {
        let owned = OwnedOrRef::Owned(123);
        let n = 123;
        let r#ref = OwnedOrRef::Ref(&n);

        assert_eq!(&123, Deref::deref(&owned));
        assert_eq!(&123, Deref::deref(&r#ref));
    }

    #[test]
    fn from_t() {
        let owned = OwnedOrRef::from(123);

        assert_eq!(OwnedOrRef::Owned(123), owned);
    }

    #[test]
    fn from_ref() {
        let n = 123;
        let ref_n = OwnedOrRef::<u32>::from(&n);
        let m = 123;
        let ref_m = OwnedOrRef::Ref(&m);

        assert_eq!(ref_n, r#ref_m);
    }
}
