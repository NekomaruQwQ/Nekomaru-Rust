//! Windows-specific extensions and event-loop helpers for `nkcore`.

#[cfg(feature = "winit")]
    /// Helpers for driving a `winit` event loop with a closure.
    pub mod winit;

/// Common Windows-specific imports for `nkcore` consumers.
pub mod prelude {
    /// Logger initialization compatible with environment-based filtering.
    pub use pretty_env_logger;

    /// Converts raw Win32 window handles into Windows API handles.
    pub use super::rwh_ext::RawWindowHandleExt;
}

mod rwh_ext {
    use windows::Win32::Foundation::HWND;
    use raw_window_handle::RawWindowHandle;

    /// Converts a [`RawWindowHandle`] into platform-native Windows handles.
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
                panic!("The provided window handle is not a Win32 window handle.");
            }
        }
    }
}
