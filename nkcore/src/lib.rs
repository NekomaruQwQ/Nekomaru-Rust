//! Unified entry point for the platform-independent and platform-specific `nkcore` crates.

/// Common imports from the core crates and the active platform integration.
pub mod prelude {
    /// Platform-independent helpers and dependency re-exports.
    pub use nkcore_base::prelude::*;

    #[cfg(windows)]
        /// Windows-specific extension traits and utilities.
        pub use nkcore_windows::prelude::*;
}

/// Operating-system-specific APIs grouped by platform.
pub mod os {
    #[cfg(windows)]
        /// Windows-specific APIs.
        pub use nkcore_windows as windows;
}

/// Platform-independent helpers from `nkcore-base`.
pub use nkcore_base::*;

#[cfg(feature = "debug")]
    /// Diagnostics for attaching call-site context to API errors.
    pub mod debug {
        /// Constructs call-site context for an error message.
        pub use nkcore_debug::context;
        /// Evaluates an API call and attaches diagnostic context on failure.
        pub use nkcore_debug::api_call;
        /// Produces a readable name for a function or method call expression.
        pub use nkcore_debug::api_name_of;
    }
