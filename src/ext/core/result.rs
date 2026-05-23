use crate::macros::identity;

#[cfg(debug_assertions)]
identity! {
    pub(crate) const trait ResultExt<T, E> {
        fn transpose_err(self) -> Option<Result<T, E>>;
    }
}

#[cfg(debug_assertions)]
identity! {
    impl<T, E> const ResultExt<T, E> for Result<T, Option<E>> {
        #[inline]
        fn transpose_err(self) -> Option<Result<T, E>> {
            match self {
                Ok(x) => Some(Ok(x)),
                Err(Some(e)) => Some(Err(e)),
                Err(None) => None,
            }
        }
    }
}

#[cfg(not(debug_assertions))]
identity! {
    pub(crate) trait ResultExt<T, E> {
        fn transpose_err(self) -> Option<Result<T, E>>;
    }
}

#[cfg(not(debug_assertions))]
identity! {
    impl<T, E> ResultExt<T, E> for Result<T, Option<E>> {
        #[inline]
        fn transpose_err(self) -> Option<Result<T, E>> {
            match self {
                Ok(x) => Some(Ok(x)),
                Err(Some(e)) => Some(Err(e)),
                Err(None) => None,
            }
        }
    }
}
