//! Foundational helpers and commonly used re-exports for the `nkcore` crates.

/// Common standard-library and dependency imports for `nkcore` consumers.
pub mod prelude {
    pub use std::{
        borrow::Cow::{Borrowed, Owned},
        path::{Path, PathBuf},
        rc::Rc,
        sync::Arc,
    };

    pub use ::anyhow;
    pub use ::anyhow::Context;
    pub use ::anyhow_auto_context::auto_context;
    pub use ::euclid::default as euclid;
    pub use ::log;
    pub use ::tap::prelude::*;

    pub fn default<T: Default>() -> T { T::default() }
}

/// Builds a default value by allowing `f` to initialize it through a mutable reference.
///
/// Any panic raised by `f` is propagated to the caller.
pub fn out_var<T, F>(f: F) -> T
where
    T: Default,
    F: FnOnce(&mut T), {
    let mut out = T::default();
    f(&mut out);
    out
}

/// Builds a default value through a fallible initializer.
///
/// If `f` fails, its error is returned and the partially initialized value is dropped.
/// Any panic raised by `f` is propagated to the caller.
pub fn out_var_or_err<T, E, F>(f: F) -> Result<T, E>
where
    T: Default,
    F: FnOnce(&mut T) -> Result<(), E>, {
    let mut out = T::default();
    f(&mut out)?;
    Ok(out)
}

/// Evaluates an expression at most once successfully per thread and call
/// site, returning the cached Copy value on subsequent evaluations.
///
/// This macro accepts an `FnOnce() -> T` closure where T is [`Copy`].
/// It caches the result in a thread-local [`OnceCell`] and returns the
/// cached value on subsequent calls. Each unique invocation of the macro
/// has its own cache entry.
///
/// Panics from the initializer propagate to the caller, leaving the cache
/// uninitialized, allowing a later invocation to retry initialization.
///
/// [`OnceCell`]: std::cell::OnceCell
#[macro_export] macro_rules! once_per_thread {
    (|| -> $ty:ty { $($body:tt)* }) => {{
        const fn assert<T>()
        where T:
            'static +
            ?::std::marker::Sized +
            ::std::marker::Copy {}
        const _: () = assert::<$ty>();

        ::std::thread_local! {
            static CELL:
                ::std::cell::OnceCell<$ty> = const {
                    ::std::cell::OnceCell::new()
                };
        }

        CELL.with(|cell| *cell.get_or_init(|| -> $ty { $($body)* }))
    }};
}
