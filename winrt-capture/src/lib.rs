
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
    Win32::Graphics::Dxgi::Common::*,
    Win32::Graphics::Dxgi::*,
    Win32::Graphics::Direct3D::*,
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
    d3d11_device: ID3D11Device,
    winrt_device: IDirect3DDevice,
    frame_pool: Direct3D11CaptureFramePool,
    frame_pool_size: Size2D<u32>,
    session: GraphicsCaptureSession,

    /// A staging texture for copying the captured frame so that it can be
    /// properly sampled in the shader.
    ///
    /// This is due to the limitation of the Windows graphics capture API
    /// that the frame pool can only be created as `B8G8R8A8_UNORM` while
    /// the captured frame contains gamma encoded data. This means that
    /// the captured frame should be viewed as `B8G8R8A8_UNORM_SRGB` to be
    /// properly gamma decoded, but such view cannot be created as that
    /// requires the frame texture to be created as `B8G8R8A8_UNORM_SRGB`
    /// or `B8G8R8A8_TYPELESS`.
    ///
    /// To work around this issue, we create a staging texture of format
    /// `B8G8R8A8_UNORM_SRGB` and copy the captured frame to the staging
    /// texture. Then the staging texture can be used as a shader resource
    /// without additional conversions.
    ///
    /// The staging texture is created with the same size as the frame pool
    /// and is recreated when the frame pool is resized.
    ///
    /// [`CaptureSession::get_next_frame`] copies the captured frame to the
    /// staging texture and returns the staging texture instead of the
    /// captured frame.
    staging_texture: ID3D11Texture2D,
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
        let winrt_device =
            Self::get_winrt_device(device)?;
        let frame_pool =
            api_call! {
                Direct3D11CaptureFramePool::CreateFreeThreaded(
                    &winrt_device,
                    DirectXPixelFormat::B8G8R8A8UIntNormalized,
                    2,
                    SizeInt32 { Width: 1, Height: 1 })
            }.context("failed to create frame pool")?;
        let staging_texture =
            Self::create_texture_2d(
                device,
                DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
                Size2D::new(1, 1))?;
        let session =
            api_call!(frame_pool.CreateCaptureSession(capture_item))
                .context("failed to create capture session from the given GraphicsCaptureItem")?;
        api_call!(session.StartCapture())
            .context("failed to start the capture session")?;
        Ok(Self {
            d3d11_device: device.clone(),
            winrt_device,
            frame_pool,
            frame_pool_size: Size2D::new(1, 1),
            staging_texture,
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
}

/// A captured frame backed by a Direct3D 11 texture.
pub struct CaptureFrame {
    /// The original texture of the captured frame obtained from the `WinRT`
    /// [`Direct3D11CaptureFrame`] object.
    ///
    /// The format of this texture is [`DXGI_FORMAT_B8G8R8A8_UNORM`] and the
    /// data is gamma encoded. This texture should not be used as a shader
    /// resource directly without manually applying gamma decoding in the shader.
    /// However, there are certain cases that a [`DXGI_FORMAT_B8G8R8A8_UNORM`]
    /// texture is still needed, such as when the captured frame needs to be
    /// encoded into a video stream using Windows Media Foundation (WMF).
    pub raw_texture: ID3D11Texture2D,

    /// The staging texture that contains the captured frame data.
    ///
    /// The format of this texture is [`DXGI_FORMAT_B8G8R8A8_UNORM_SRGB`] and the
    /// data is gamma encoded. The `texture_view` field provides a shader resource
    /// view of this texture that is sRGB-aware and can be sampled in the shader
    /// without additional conversions.
    pub texture: ID3D11Texture2D,

    /// The shader resource view of the staging texture that contains the captured
    /// frame data.
    ///
    /// The format of this texture view is [`DXGI_FORMAT_B8G8R8A8_UNORM_SRGB`] and
    /// it can be sampled in the shader without additional conversions.
    pub texture_view: ID3D11ShaderResourceView,

    /// The size of the captured frame, as well as the size of the staging texture,
    /// in pixels.
    pub size: Size2D<u32>,
}

impl CaptureSession {
    /// Acquires the next captured frame as a [`ID3D11Texture2D`]. Returns `None`
    /// if there is no new frame available since the last call.
    ///
    /// The returned texture is a staging texture that contains the copied frame
    /// data. The staging texture is used to work around the limitations of the
    /// Windows graphics capture API that prevents the captured frame from being
    /// used as a shader resource directly.
    pub fn get_next_frame(&mut self, ctx: &ID3D11DeviceContext)
     -> anyhow::Result<Option<CaptureFrame>> {
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
                // The frame size has not changed. We can just copy the new frame to
                // the staging texture and return it.
                let new_texture = Self::get_texture_from_capture_frame(&new_frame)?;
                unsafe {
                    ctx.CopyResource(&self.staging_texture, &new_texture);
                }

                Ok(Some(CaptureFrame {
                    raw_texture: new_texture,
                    texture:
                        self.staging_texture.clone(),
                    texture_view:
                        Self::create_srv_for_texture_2d(
                            &self.d3d11_device,
                            &self.staging_texture,
                            DXGI_FORMAT_B8G8R8A8_UNORM_SRGB)?,
                    size: new_size,
                }))
            } else {
                // The frame size has changed and the frame pool must be recreated.
                self.resize_frame_pool_and_staging_buffer(new_size)?;

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

    fn resize_frame_pool_and_staging_buffer(&mut self, new_size: Size2D<u32>)
     -> anyhow::Result<()> {
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

            // The staging texture must be resized accordingly to match the
            // new size of the frame pool.
            //
            // Note that if an error occurs here, `self.frame_pool_size` is
            // not updated and the next call to `get_next_frame` will try to
            // recreate the frame pool again. This is not ideal but it makes
            // the error handling simpler and more robust.
            self.staging_texture =
                Self::create_texture_2d(
                    &self.d3d11_device,
                    DXGI_FORMAT_B8G8R8A8_UNORM_SRGB,
                    new_size)?;

            // The `frame_pool_size` must be updated after the frame pool is
            // successfully recreated.
            self.frame_pool_size = new_size;

            // Done. Log the new size for debugging purposes.
            log::info!("frame pool and staging buffer resized to {new_size:?}");
        }.context("failed to resize frame pool")
    }
}

impl CaptureSession {
    fn create_texture_2d(device: &ID3D11Device, format: DXGI_FORMAT, size: Size2D<u32>)
     -> anyhow::Result<ID3D11Texture2D> {
        let desc = D3D11_TEXTURE2D_DESC {
            Width: size.width,
            Height: size.height,
            MipLevels: 1,
            ArraySize: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_SHADER_RESOURCE.0 as _,
            CPUAccessFlags: 0,
            MiscFlags: 0,
        };

        match nkcore::out_var_or_err(|out| api_call!(unsafe {
            device.CreateTexture2D(
                &raw const desc,
                None,
                Some(out))
        })) {
            Ok(Some(texture)) => Ok(texture),
            Ok(None) => Err(anyhow::anyhow!("CreateTexture2D returned None without an error code")),
            Err(err) => Err(err),
        }.context("failed to create texture")
    }

    fn create_srv_for_texture_2d(
        device: &ID3D11Device,
        texture: &ID3D11Texture2D,
        format: DXGI_FORMAT)
     -> anyhow::Result<ID3D11ShaderResourceView> {
        let desc = D3D11_SHADER_RESOURCE_VIEW_DESC {
            Format: format,
            ViewDimension: D3D_SRV_DIMENSION_TEXTURE2D,
            Anonymous: D3D11_SHADER_RESOURCE_VIEW_DESC_0 {
                Texture2D: D3D11_TEX2D_SRV {
                    MostDetailedMip: 0,
                    MipLevels: 1,
                },
            },
        };

        match nkcore::out_var_or_err(|out| api_call!(unsafe {
            device.CreateShaderResourceView(
                texture,
                Some(&raw const desc),
                Some(out))
        })) {
            Ok(Some(srv)) => Ok(srv),
            Ok(None) => Err(anyhow::anyhow!("CreateShaderResourceView returned None without an error code")),
            Err(err) => Err(err),
        }.context("failed to create shader resource view for the given texture")
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
