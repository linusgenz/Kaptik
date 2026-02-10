use anyhow::Result;
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Win32::Foundation::HMODULE;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::Win32::Graphics::Dxgi::*;
use windows::Win32::System::WinRT::Direct3D11::CreateDirect3D11DeviceFromDXGIDevice;
use windows::core::Interface;

/// Creates a Direct3D11 device and context
pub fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext)> {
    unsafe {
        let mut device = None;
        let mut context = None;

        D3D11CreateDevice(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE(std::ptr::null_mut()),
            D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            None,
            D3D11_SDK_VERSION,
            Some(&mut device),
            None,
            Some(&mut context),
        )?;

        Ok((
            device.ok_or_else(|| anyhow::anyhow!("Device creation failed"))?,
            context.ok_or_else(|| anyhow::anyhow!("Context creation failed"))?,
        ))
    }
}

/// Creates a WinRT Direct3D device from a D3D11 device
pub fn create_direct3d_device(d3d_device: &ID3D11Device) -> Result<IDirect3DDevice> {
    unsafe {
        let dxgi_device: IDXGIDevice = d3d_device.cast()?;
        let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
        Ok(inspectable.cast()?)
    }
}

/// Checks if HDR is currently enabled on the primary display
pub fn check_hdr_enabled() -> Result<bool> {
    use windows::Win32::Graphics::Dxgi::Common::{
        DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709, DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020,
    };

    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1()?;
        let adapter: IDXGIAdapter1 = factory.EnumAdapters1(0)?;
        let output: IDXGIOutput = adapter.EnumOutputs(0)?;
        let output6: IDXGIOutput6 = output.cast()?;
        let desc = output6.GetDesc1()?;

        crate::log!("🔍 Display ColorSpace: {:?}", desc.ColorSpace);

        let is_hdr = desc.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020 // HDR10
            || desc.ColorSpace == DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709; // scRGB

        Ok(is_hdr)
    }
}

/// Creates an SDR render target texture
pub fn create_sdr_target(
    device: &ID3D11Device,
    width: u32,
    height: u32,
) -> Result<(ID3D11Texture2D, ID3D11RenderTargetView)> {
    use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};

    let desc = D3D11_TEXTURE2D_DESC {
        Width: width,
        Height: height,
        MipLevels: 1,
        ArraySize: 1,
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        Usage: D3D11_USAGE_DEFAULT,
        BindFlags: (D3D11_BIND_RENDER_TARGET | D3D11_BIND_SHADER_RESOURCE).0 as u32,
        CPUAccessFlags: 0,
        MiscFlags: 0,
    };

    unsafe {
        let mut tex: Option<ID3D11Texture2D> = None;
        device.CreateTexture2D(&desc, None, Some(&mut tex))?;
        let tex = tex.expect("CreateTexture2D returned null");

        let mut rtv: Option<ID3D11RenderTargetView> = None;
        device.CreateRenderTargetView(&tex, None, Some(&mut rtv))?;
        let rtv = rtv.expect("CreateRenderTargetView returned null");

        Ok((tex, rtv))
    }
}
