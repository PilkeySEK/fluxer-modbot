/// Panic only if in debug mode.
macro_rules! debug_panic {
    ($($reason:tt)*) => {
        if cfg!(debug_assertions) {
            panic!($($reason)*);
        }
    };
}

pub(crate) use debug_panic;
