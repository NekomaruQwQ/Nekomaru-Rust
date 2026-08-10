//! Nekomaru's Core Library for Windows.
/// Common Windows-specific imports for `nkcore` consumers.
pub mod prelude {
    pub use pretty_env_logger;

    pub use crate::rwh_ext::RawWindowHandleExt;
}

#[cfg(feature = "winit")] pub mod winit;
#[cfg(feature = "d3d11")] pub mod d3d11;
#[cfg(feature = "process-tree")] mod process_tree;
#[cfg(feature = "process-tree")] pub use process_tree::kill_children_on_exit;

pub use rwh_ext::RawWindowHandleExt;
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
