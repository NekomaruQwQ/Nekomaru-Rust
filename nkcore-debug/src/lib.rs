//! Diagnostic macros that attach source locations and API names to errors.

#[doc(hidden)] pub use nkcore_debug_macros::api_name_of_internal as __api_name_of_internal;
#[doc(hidden)] pub use anyhow as __anyhow;
#[doc(hidden)] pub use pretty_name::type_name as __type_name;

use std::fmt;

/// Returns the name of the type inferred by `type_check` without invoking it.
///
/// The closure exists only to let generated code constrain `T` from an
/// expression while leaving that expression unevaluated.
#[doc(hidden)]
pub fn __api_receiver_type_name<T: ?Sized>(
    _type_check: impl FnOnce(&T)) -> &'static str {
    __type_name::<T>()
}

/// Constrains two references to have the same referent type.
#[doc(hidden)]
pub const fn __api_same_type<T: ?Sized>(_: &T, _: &T) {}

/// Source location and message attached to a failed API call.
#[expect(non_camel_case_types, reason = "internal type used by macros")]
#[doc(hidden)]
pub struct __api_call_context_t {
    /// Source file containing the instrumented call.
    pub file: &'static str,
    /// One-based source line containing the instrumented call.
    pub line: u32,
    /// Human-readable description of the failure.
    pub message: String,
}

impl fmt::Display for __api_call_context_t {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { file, line, ref message } = self;
        write!(f, "{message}\n    at {file}:{line}")
    }
}

/// Constructs a [`__api_call_context_t`] at the macro invocation's source location.
///
/// The arguments use the same format-string syntax as [`format!`]. Formatting
/// failures and panics from argument evaluation propagate to the caller.
#[doc(hidden)] #[macro_export] macro_rules! __api_call_context {
    ($message:expr $(, $message_arg:expr)*) => {
        $crate::__api_call_context_t {
            file: file!(),
            line: line!(),
            message: format!($message $(, $message_arg)*),
        }
    };
}

/// Evaluates a fallible API call and attaches call-site diagnostics on failure.
///
/// In debug builds, the context includes the parsed API name and source location.
/// The `unsafe { expression }` form keeps the unsafe operation visibly scoped at
/// the call site. The expression is evaluated exactly once, and any panic propagates.
#[macro_export] macro_rules! api_call {
    (unsafe { $expr:expr }) => { unsafe {
        $crate::__anyhow::Context::with_context(
            $expr, || $crate::__api_call_context!("{} failed", $crate::api_name_of!($expr)))
    }};
    ($expr:expr) => {{
        $crate::__anyhow::Context::with_context(
            $expr, || $crate::__api_call_context!("{} failed", $crate::api_name_of!($expr)))
    }};
}

/// Returns a readable name for a function or method call expression.
///
/// Debug builds preserve explicitly supplied type and lifetime generic arguments.
/// Unsupported expressions and const generic arguments produce a compile-time error.
#[macro_export] macro_rules! api_name_of(($expr:expr) => {
    $crate::__api_name_of_internal! {
        #[api_name_args(
            type_name = $crate::__type_name,
            receiver_type_name = $crate::__api_receiver_type_name,
            same_type = $crate::__api_same_type)]
        $expr
    }
});

#[cfg(test)]
#[expect(dead_code, reason = "test code")]
#[expect(clippy::unused_self, reason = "test code")]
mod test {
    #[test] fn api_call() {
        assert!(
            api_call!("test".parse::<u32>())
                .unwrap_err()
                .to_string()
                .starts_with("str::parse::<u32> failed"));
    }

    fn foo<P0, P1>(_: P0, _: P1) {}

    struct Foo;
    impl Foo {
        fn bar<P0, P1>(&self, _: P0, _: P1) {}
    }

    struct FallibleApi;
    impl FallibleApi {
        fn fail(&self) -> std::io::Result<()> {
            Err(std::io::Error::other("expected failure"))
        }
    }

    fn make_fallible_api(evaluations: &std::cell::Cell<usize>) -> FallibleApi {
        evaluations.set(evaluations.get() + 1);
        FallibleApi
    }

    #[test] fn api_call_evaluates_method_receiver_once_on_failure() {
        let evaluations = std::cell::Cell::new(0);

        let _error =
            api_call!(make_fallible_api(&evaluations).fail())
                .unwrap_err();

        assert_eq!(evaluations.get(), 1);
    }

    #[test] fn api_name_of() {
        let foo = Foo;
        assert_eq!(api_name_of!(foo.bar(0u32, 0u32)), "Foo::bar");
        assert_eq!(api_name_of!(foo.bar::<u32, u32>(0, 0)), "Foo::bar::<u32, u32>");

        assert_eq!(api_name_of!(foo(0u32, 0u32)), "foo");
        assert_eq!(api_name_of!(foo::<u32, u32>(0, 0)), "foo::<u32, u32>");

        assert_eq!(api_name_of!(super::test::foo(0u32, 0u32)), "foo");
        assert_eq!(api_name_of!(super::test::foo::<u32, u32>(0, 0)), "foo::<u32, u32>");

        assert_eq!(api_name_of!(Foo::bar(0u32, 0u32)), "Foo::bar");
        assert_eq!(api_name_of!(Foo::bar::<u32, u32>(0, 0)), "Foo::bar::<u32, u32>");

        assert_eq!(api_name_of!(crate::test::Foo::bar(0u32, 0u32)), "Foo::bar");
        assert_eq!(api_name_of!(crate::test::Foo::bar::<u32, u32>(0, 0)), "Foo::bar::<u32, u32>");
    }
}
