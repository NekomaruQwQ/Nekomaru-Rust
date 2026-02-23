pub mod prelude {
    pub use ::anyhow;
    pub use ::euclid::default as euclid;
    pub use ::log;

    pub use crate::default;
}

pub use crate::defer::defer;
pub use crate::env::executable_dir;

pub fn default<T: Default>() -> T { T::default() }

pub fn out_var<T: Default, F: FnOnce(&mut T)>(f: F) -> T {
    let mut out: T = Default::default();
    f(&mut out);
    out
}

pub fn out_var_or_err<T: Default, E, F: FnOnce(&mut T) -> Result<(), E>>(f: F) -> Result<T, E> {
    let mut out: T = Default::default();
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
/// This macro accepts an `FnOnce() -> T` closure, caches it using a thread-local
/// [`OnceCell`] and returns a `'static` reference to the cached value.
/// Each unique macro invocation has its own cache entry.
///
/// [`OnceCell`]: std::cell::OnceCell
#[macro_export] macro_rules! cache {
    (|| -> $ty:ty $init:block) => {{
        ::std::thread_local!(
            static CACHE:
                ::std::cell::OnceCell<&'static $ty> =
                ::std::cell::OnceCell::new());
        CACHE.with(|once| *once.get_or_init(|| Box::leak(Box::new($init))))
    }};
}
