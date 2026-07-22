//! Foundational helpers and commonly used re-exports for the `nkcore` crates.

/// Common standard-library and dependency imports for `nkcore` consumers.
pub mod prelude {
    /// Frequently used owned and shared pointer types.
    pub use std::{
        borrow::Cow::{Borrowed, Owned},
        rc::Rc,
        sync::Arc,
    };

    /// Error handling utilities from `anyhow`.
    pub use ::anyhow;
    /// Adds contextual information to errors without importing the trait by name.
    pub use ::anyhow::Context as _;
    /// Euclid types using its default, untyped coordinate-space aliases.
    pub use ::euclid::default as euclid;
    /// Logging facade used throughout the workspace.
    pub use ::log;
    /// Method-chaining helpers from `tap`.
    pub use ::tap::prelude::*;

    /// Creates a value using its [`Default`] implementation.
    pub use crate::default;
}

/// Creates a scope guard that invokes its closure when dropped.
pub use crate::defer::defer;
/// Returns the directory containing the running executable.
pub use crate::env::executable_dir;

/// Creates a value using its [`Default`] implementation.
pub fn default<T: Default>() -> T { T::default() }

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

mod defer {
    /// Creates a guard that invokes `f` exactly once when the guard is dropped.
    ///
    /// Any panic raised by `f` is propagated from the guard's destructor.
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

    /// Returns the parent directory of the current executable path.
    ///
    /// Errors from resolving the current executable are returned to the caller.
    ///
    /// # Panics
    ///
    /// Panics on platforms that return an executable path without a parent.
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
/// Panics from the initializer propagate to the caller and leave the cache
/// uninitialized, allowing a later invocation to retry initialization.
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
