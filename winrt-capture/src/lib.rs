
#![feature(try_blocks)]

use nkcore::prelude::*;
use nkcore::debug::*;

use euclid::*;

use windows::core::Interface as _;
use windows::{
    Graphics::*,
    Graphics::Capture::*,
    Graphics::DirectX::*,
    Graphics::DirectX::Direct3D11::*,
    UI::*,
    Win32::Foundation::*,
    Win32::Graphics::Dxgi::*,
    Win32::Graphics::Direct3D11::*,
    Win32::System::WinRT::Direct3D11::*,
};

/// A capture session for a window or a display. It provides the captured
/// frame as a [`ID3D11Texture2D`] that can be rendered with Direct3D 11.
///
/// This is a RAII wrapper around a pair of [`GraphicsCaptureSession::StartCapture`]
/// and [`GraphicsCaptureSession::Close`] calls.
/// The capture is automatically stopped when the [`CaptureSession`] is dropped.
pub struct CaptureSession {
    winrt_device: IDirect3DDevice,
    frame_pool: Direct3D11CaptureFramePool,
    frame_pool_size: Size2D<u32>,
    session: GraphicsCaptureSession,
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        // The capture session must be stopped before the frame pool is dropped.
        // Errors are logged but ignored as we are in the destructor.
        if let Err(err) = self.session.Close() {
            log::error!("failed to stop the capture session: {err}");
        }
    }
}

impl CaptureSession {
    /// Creates a new capture session for the given [`GraphicsCaptureItem`] and
    /// starts the capture immediately.
    pub fn new(device: &ID3D11Device, capture_item: &GraphicsCaptureItem)
     -> anyhow::Result<Self> {
        let winrt_device = Self::get_winrt_device(device)?;
        let frame_pool =
            api_call! {
                Direct3D11CaptureFramePool::CreateFreeThreaded(
                    &winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    SizeInt32 { Width: 1, Height: 1 })
            }.context("failed to create frame pool")?;
        let session =
            api_call!(frame_pool.CreateCaptureSession(capture_item))
                .context("failed to create capture session from the given GraphicsCaptureItem")?;
        api_call!(session.StartCapture())
            .context("failed to start the capture session")?;
        Ok(Self {
            winrt_device,
            frame_pool,
            frame_pool_size: Size2D::new(1, 1),
            session,
        })
    }

    /// Creates a new capture session for the given window and starts the capture
    /// immediately.
    pub fn from_hwnd(device: &ID3D11Device, hwnd: HWND)
     -> anyhow::Result<Self> {
        let window_id = WindowId { Value: hwnd.0 as _ };
        let capture_item =
            api_call!(GraphicsCaptureItem::TryCreateFromWindowId(window_id))
                .context("failed to create a GraphicsCaptureItem from the given HWND")?;
        Self::new(device, &capture_item)
    }

    /// Acquires the next captured frame as a [`ID3D11Texture2D`]. Returns `None`
    /// if there is no new frame available since the last call.
    pub fn get_next_frame(&mut self)
     -> anyhow::Result<Option<ID3D11Texture2D>> {
        // Here we obtain all the frames in the pool while keeping only the
        // last one. Previous frames are discarded as they are outdated and
        // will never be rendered.
        let mut last_frame = None;
        while let Ok(frame) = self.frame_pool.TryGetNextFrame() {
            last_frame = Some(frame);
        }

        if let Some(new_frame) = last_frame {
            let new_size =
                new_frame.ContentSize()
                    .context("failed to get the expected size of the frame pool")?;
            let new_size =
                Size2D::new(
                    new_size.Width as _,
                    new_size.Height as _);
            if new_size == self.frame_pool_size {
                // The frame size has not changed.
                Ok(Some(Self::get_texture_from_capture_frame(&new_frame)?))
            } else {
                // The frame size has changed and the frame pool must be recreated.
                self.resize_frame_pool(new_size)?;

                // The new frame is not likely valid as the frame was not captured
                // with the correct size.
                // So we just return None without trying to get the texture from it.
                Ok(None)
            }
        } else {
            // There is no new frame available since the last call.
            Ok(None)
        }
    }

    fn resize_frame_pool(&mut self, new_size: Size2D<u32>) -> anyhow::Result<()> {
        try {
            // The frame size has changed and the frame pool must be recreated.
            api_call! {
                self.frame_pool.Recreate(
                    &self.winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    SizeInt32 {
                        Width: new_size.width as _,
                        Height: new_size.height as _,
                    })
            }?;

            // The `frame_pool_size` must be updated after the frame pool is
            // successfully recreated.
            self.frame_pool_size = new_size;

            log::info!("frame pool resized to {new_size:?}");
        }.context("failed to resize frame pool")
    }

    fn get_winrt_device(device: &ID3D11Device)
     -> anyhow::Result<IDirect3DDevice> {
        try {
            let dxgi_device =
                api_call!(device.cast::<IDXGIDevice>())?;
            let winrt_device =
                api_call!(unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device) })?;
            api_call!(winrt_device.cast::<IDirect3DDevice>())?
        }.context("failed to get IDirect3DDevice from ID3D11Device")
    }

    fn get_texture_from_capture_frame(frame: &Direct3D11CaptureFrame)
     -> anyhow::Result<ID3D11Texture2D> {
        try {
            let surface = api_call!(unsafe { frame.Surface() })?;
            let surface = api_call!(unsafe { surface.cast::<IDirect3DDxgiInterfaceAccess>() })?;
            api_call!(unsafe { surface.GetInterface::<ID3D11Texture2D>() })?
        }.context("failed to get ID3D11Texture from Direct3D11CaptureFrame")
    }
}
