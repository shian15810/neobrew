macro_rules! identity {
    ($($tt:tt)*) => {
        $($tt)*
    };
}

pub(crate) use identity;
