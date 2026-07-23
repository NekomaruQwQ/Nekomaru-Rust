//! Nekomaru's Core Library for Windows.

pub use nkcore_base::*;

/// Common Windows-specific imports for `nkcore` consumers.
pub mod prelude {
    pub use nkcore_base::prelude::*;

    pub use pretty_env_logger;

    pub use super::rwh_ext::RawWindowHandleExt;
}

/// Helpers for driving a `winit` event loop with a closure.
#[cfg(feature = "winit")] pub mod winit;

/// Direct3D 11 helpers.
#[cfg(feature = "d3d11")] pub mod d3d11;

/// Diagnostics for attaching call-site context to API errors.
#[cfg(feature = "debug")] pub use nkcore_debug as debug;

mod rwh_ext {
    use windows::Win32::Foundation::HWND;
    use raw_window_handle::RawWindowHandle;

    /// Converts a [`RawWindowHandle`] into platform-native handles.
    pub trait RawWindowHandleExt {
        /// Returns this handle as an [`HWND`].
        ///
        /// # Panics
        ///
        /// Panics if this is not a [`RawWindowHandle::Win32`] handle.
        fn as_hwnd(&self) -> HWND;
    }

    impl RawWindowHandleExt for RawWindowHandle {
        fn as_hwnd(&self) -> HWND {
            if let &Self::Win32(handle) = self {
                HWND(handle.hwnd.get() as _)
            } else {
                panic!("Not a Win32 window handle.");
            }
        }
    }
}
