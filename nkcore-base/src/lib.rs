pub mod prelude {
    pub use std::{
        borrow::Cow::{Borrowed, Owned},
        rc::Rc,
        sync::Arc,
    };

    pub use ::anyhow;
    pub use ::anyhow::Context as _;
    pub use ::euclid::default as euclid;
    pub use ::log;
    pub use ::tap::prelude::*;

    pub use crate::default;
}

pub use crate::defer::defer;
pub use crate::env::executable_dir;

pub fn default<T: Default>() -> T { T::default() }

pub fn out_var<T, F>(f: F) -> T
where
    T: Default,
    F: FnOnce(&mut T), {
    let mut out = T::default();
    f(&mut out);
    out
}

pub fn out_var_or_err<T, E, F>(f: F) -> Result<T, E>
where
    T: Default,
    F: FnOnce(&mut T) -> Result<(), E>, {
    let mut out = T::default();
    f(&mut out)?;
    Ok(out)
}

mod defer {
    pub fn defer<F: FnOnce()>(f: F) -> impl Drop { Defer(Some(f)) }
    struct Defer<F: FnOnce()>(Option<F>);
    impl<F: FnOnce()> Drop for Defer<F> {
        fn drop(&mut self) {
            // SAFETY(unwrap): no way to set to None except here.
            self.0.take().unwrap()();
        }
    }
}

mod env {
    use std::env;
    use std::io;
    use std::path::PathBuf;

    pub fn executable_dir() -> io::Result<PathBuf> {
        Ok({
            env::current_exe()?
                .parent()
                .unwrap()
                .to_owned()
        })
    }
}

/// Helper macro for caching calculation results in thread-local storage.
///
/// This macro accepts an `FnOnce() -> T` closure where T is [`Copy`].
/// It caches the result of the closure on the first invocation, and
/// on subsequent invocations, it returns the cached value instead of
/// re-evaluating the closure.
///
/// The cached value is stored in a thread-local [`OnceCell`]. Each unique
/// invocation of the macro has its own cache entry, so different calls to
/// the macro with different closures will not interfere with each other's
/// cached values.
///
/// [`OnceCell`]: std::cell::OnceCell
#[macro_export] macro_rules! once {
    (|| -> $ty:ty { $($body:tt)* }) => {{
        fn assert_static<T: 'static>() {}
        fn assert_copy<T: Copy>() {}
        assert_static::<$ty>();
        assert_copy::<$ty>();

        ::std::thread_local! {
            static CELL:
                ::std::cell::OnceCell<$ty> =
                ::std::cell::OnceCell::new();
        }

        CELL.with(|cell| *cell.get_or_init(|| -> $ty { $($body)* }))
    }};
}
