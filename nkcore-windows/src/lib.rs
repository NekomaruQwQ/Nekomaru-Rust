#[cfg(feature = "winit")] pub mod winit;

pub mod prelude {
    pub use crate::RawWindowHandleExt as _;
}

use windows::Win32::Foundation::HWND;
use raw_window_handle::RawWindowHandle;

pub trait RawWindowHandleExt {
    fn as_hwnd(&self) -> HWND;
}

impl RawWindowHandleExt for RawWindowHandle {
    fn as_hwnd(&self) -> HWND {
        if let RawWindowHandle::Win32(handle) = self {
            HWND(handle.hwnd.get() as _)
        } else {
            panic!("The provided window handle is not a Win32 window handle.");
        }
    }
}
