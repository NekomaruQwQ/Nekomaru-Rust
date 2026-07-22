//! Diagnostic macros that attach source locations and API names to errors.

use nkcore_debug_macros as macros;

/// Implementation details referenced by the crate's exported macros.
///
/// This module is public because exported macros must resolve these names from
/// downstream crates; it is not intended as a direct consumer API.
#[doc(hidden)]
pub mod __ {
    /// Internal procedural macro used to parse API call expressions.
    pub use crate::macros::api_name_of_internal;
    /// Call-site context produced by [`crate::context!`].
    pub use crate::Context;
    /// Error type and context extension trait used by [`crate::api_call!`].
    pub use anyhow;
    /// Returns a readable name for a type.
    pub use pretty_name::type_name;
    /// Returns a readable name for a value's type.
    pub use pretty_name::type_name_of_val;
}

/// Source location and message attached to a failed API call.
pub struct Context {
    /// Source file containing the instrumented call.
    pub file: &'static str,
    /// One-based source line containing the instrumented call.
    pub line: u32,
    /// Human-readable description of the failure.
    pub message: String,
}

use std::fmt;

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { file, line, ref message } = self;
        write!(f, "{message}\n    at {file}:{line}")
    }
}

/// Constructs a [`Context`] at the macro invocation's source location.
///
/// The arguments use the same format-string syntax as [`format!`]. Formatting
/// failures and panics from argument evaluation propagate to the caller.
#[macro_export] macro_rules! context {
    ($message:expr $(, $message_arg:expr)*) => {
        $crate::__::Context {
            file: file!(),
            line: line!(),
            message: format!($message $(, $message_arg)*),
        }
    };
}

#[cfg(debug_assertions)]
/// Evaluates a fallible API call and attaches call-site diagnostics on failure.
///
/// In debug builds, the context includes the parsed API name and source location.
/// The `unsafe { expression }` form keeps the unsafe operation visibly scoped at
/// the call site. The expression is evaluated exactly once, and any panic propagates.
#[macro_export] macro_rules! api_call {
    (unsafe { $expr:expr }) => { unsafe {
        $crate::__::anyhow::Context::with_context(
            $expr, || $crate::context!("{} failed", $crate::api_name_of!($expr)))
    }};
    ($expr:expr) => {{
        $crate::__::anyhow::Context::with_context(
            $expr, || $crate::context!("{} failed", $crate::api_name_of!($expr)))
    }};
}

#[cfg(not(debug_assertions))]
/// Evaluates a fallible API call and attaches lightweight context on failure.
///
/// Release builds use a fixed message to avoid API-name and source-location
/// formatting overhead. The expression is evaluated exactly once, and any panic
/// propagates. The `unsafe { expression }` form keeps unsafe calls visibly scoped.
#[macro_export] macro_rules! api_call {
    (unsafe { $expr:expr }) => { unsafe {
        $crate::__::anyhow::Context::context($expr, "api call failed")
    }};
    ($expr:expr) => {{
        $crate::__::anyhow::Context::context($expr, "api call failed")
    }};
}

#[cfg(debug_assertions)]
/// Returns a readable name for a function or method call expression.
///
/// Debug builds preserve explicitly supplied type and lifetime generic arguments.
/// Unsupported expressions and const generic arguments produce a compile-time error.
#[macro_export] macro_rules! api_name_of(($expr:expr) => {
    $crate::__::api_name_of_internal! {
        #[api_name_args(
            type_name = $crate::__::type_name,
            type_name_of = $crate::__::type_name_of_val)]
        $expr
    }
});

#[cfg(not(debug_assertions))]
/// Returns a placeholder API name without runtime parsing overhead.
///
/// Release builds always expand to `"<unknown>"` and do not evaluate the expression.
#[macro_export] macro_rules! api_name_of(($expr:expr) => { "<unknown>"});

#[cfg(test)]
#[cfg(debug_assertions)]
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
