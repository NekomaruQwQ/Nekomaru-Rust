use nkcore_debug_macros as macros;

pub mod __ {
    pub use crate::macros::api_name_of_internal;
    pub use crate::Context;
    pub use anyhow;
    pub use pretty_name::type_name;
    pub use pretty_name::type_name_of_val;
}

pub struct Context {
    pub file: &'static str,
    pub line: u32,
    pub message: String,
}

use std::fmt;

impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let &Self { file, line, ref message } = self;
        write!(f, "{message}\n    at {file}:{line}")
    }
}

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
#[macro_export] macro_rules! api_call {
    (unsafe { $expr:expr }) => { unsafe {
        $crate::__::anyhow::Context::context($expr, "api call failed")
    }};
    ($expr:expr) => {{
        $crate::__::anyhow::Context::context($expr, "api call failed")
    }};
}

#[cfg(debug_assertions)]
#[macro_export] macro_rules! api_name_of(($expr:expr) => {
    $crate::__::api_name_of_internal! {
        #[api_name_args(
            type_name = $crate::__::type_name,
            type_name_of = $crate::__::type_name_of_val)]
        $expr
    }
});

#[cfg(not(debug_assertions))]
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
