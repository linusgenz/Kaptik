use crate::log;
use crate::recorder::capture::core::d3d;
use anyhow::Result;
use std::ffi::CString;
use windows::Win32::Graphics::Direct3D::*;
use windows::Win32::Graphics::Direct3D11::*;
use windows::core::PCSTR;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum TonemapAlgorithm {
    Reinhard,
    AcesSimple,
    AcesFitted,
    Uncharted2,
    HejlDawson,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum HdrNitsMode {
    HdrNitsAuto,
    HdrNits1000,
    HdrNits2000,
    HdrNits4000,
    HdrNits10000,
}

impl Default for HdrNitsMode {
    fn default() -> Self {
        Self::HdrNitsAuto
    }
}

#[repr(C)]
struct ToneMapCB {
    input_max_nits: f32,
    output_max_nits: f32,
    _pad: [f32; 2],
}

/// HDR to SDR tone mapping renderer
pub struct ToneMapRenderer {
    vs: ID3D11VertexShader,
    ps: ID3D11PixelShader,
    sampler: ID3D11SamplerState,
    constant_buffer: ID3D11Buffer,
    rtv: ID3D11RenderTargetView,
    sdr_texture: ID3D11Texture2D,
    width: u32,
    height: u32,
}

impl TonemapAlgorithm {
    fn shader_source(&self) -> &'static str {
        match self {
            Self::Reinhard => include_str!("../shader/tonemap_ps_reinhard.hlsl"),
            Self::AcesSimple => include_str!("../shader/tonemap_ps_aces_simple.hlsl"),
            Self::AcesFitted => include_str!("../shader/tonemap_ps_aces_fitted.hlsl"),
            Self::Uncharted2 => include_str!("../shader/tonemap_ps_uncharted2.hlsl"),
            Self::HejlDawson => include_str!("../shader/tonemap_ps_hejl.hlsl"),
        }
    }

    /// Returns a human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Reinhard => "Reinhard",
            Self::AcesSimple => "ACES Simple (Fast)",
            Self::AcesFitted => "ACES Fitted (Best Quality)",
            Self::Uncharted2 => "Uncharted 2 (Filmic)",
            Self::HejlDawson => "Hejl-Dawson (Fast Filmic)",
        }
    }
}

impl Default for TonemapAlgorithm {
    fn default() -> Self {
        Self::AcesFitted
    }
}

impl ToneMapRenderer {
    pub fn new(
        device: &ID3D11Device,
        width: u32,
        height: u32,
        algorithm: TonemapAlgorithm,
    ) -> Result<Self> {
        let (sdr_texture, rtv) = d3d::create_sdr_target(device, width, height)?;

        let vs_blob = compile_shader(include_str!("../shader/tonemap_vs.hlsl"), "main", "vs_5_0")?;

        log!("Using tonemap algorithm: {}", algorithm.name());

        let ps_blob = compile_shader(algorithm.shader_source(), "main", "ps_5_0")?;

        let vs = create_vertex_shader(device, &vs_blob)?;
        let ps = create_pixel_shader(device, &ps_blob)?;
        let sampler = create_sampler_state(device)?;
        let constant_buffer = create_constant_buffer(device)?;

        Ok(Self {
            vs,
            ps,
            sampler,
            constant_buffer,
            rtv,
            sdr_texture,
            width,
            height,
        })
    }

    /// Converts an HDR texture to SDR
    pub fn tonemap(
        &self,
        device: &ID3D11Device,
        context: &ID3D11DeviceContext,
        hdr_texture: &ID3D11Texture2D,
    ) -> Result<ID3D11Texture2D> {
        unsafe {
            // Create shader resource view for input
            let mut srv: Option<ID3D11ShaderResourceView> = None;
            device.CreateShaderResourceView(hdr_texture, None, Some(&mut srv))?;
            let srv = srv.ok_or_else(|| anyhow::anyhow!("Failed to create SRV"))?;

            let cb = ToneMapCB {
                input_max_nits: 5000.0,
                output_max_nits: 100.0,
                _pad: [0.0; 2],
            };
            context.UpdateSubresource(&self.constant_buffer, 0, None, &cb as *const _ as _, 0, 0);

            // Set render state
            context.OMSetRenderTargets(Some(&[Some(self.rtv.clone())]), None);
            context.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
            context.PSSetSamplers(0, Some(&[Some(self.sampler.clone())]));
            context.VSSetShader(&self.vs, None);
            context.PSSetShader(&self.ps, None);
            context.PSSetConstantBuffers(0, Some(&[Some(self.constant_buffer.clone())]));
            context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLELIST);

            // Set viewport
            let viewport = D3D11_VIEWPORT {
                TopLeftX: 0.0,
                TopLeftY: 0.0,
                Width: self.width as f32,
                Height: self.height as f32,
                MinDepth: 0.0,
                MaxDepth: 1.0,
            };
            context.RSSetViewports(Some(&[viewport]));

            // Draw fullscreen triangle
            context.Draw(3, 0);

            // Cleanup
            context.PSSetShaderResources(0, Some(&[None]));
            context.OMSetRenderTargets(None, None);
            context.Flush();

            Ok(self.sdr_texture.clone())
        }
    }
}

fn compile_shader(source: &str, entry: &str, profile: &str) -> Result<ID3DBlob> {
    unsafe {
        use windows::Win32::Graphics::Direct3D::Fxc::D3DCompile;

        let mut blob = None;
        let mut error_blob = None;

        let entry_cstr = CString::new(entry)?;
        let profile_cstr = CString::new(profile)?;
        let pc_entry = PCSTR::from_raw(entry_cstr.as_ptr() as *const u8);
        let pc_profile = PCSTR::from_raw(profile_cstr.as_ptr() as *const u8);

        D3DCompile(
            source.as_ptr() as _,
            source.len(),
            None,
            None,
            None,
            pc_entry,
            pc_profile,
            0,
            0,
            &mut blob,
            Some(&mut error_blob),
        )
        .map_err(|e| {
            if let Some(err) = error_blob {
                let msg = std::slice::from_raw_parts(
                    err.GetBufferPointer() as *const u8,
                    err.GetBufferSize(),
                );
                anyhow::anyhow!("Shader compile error: {}", String::from_utf8_lossy(msg))
            } else {
                anyhow::anyhow!("Shader compile failed: {:?}", e)
            }
        })?;

        Ok(blob.unwrap())
    }
}

fn create_vertex_shader(device: &ID3D11Device, blob: &ID3DBlob) -> Result<ID3D11VertexShader> {
    unsafe {
        let bytes =
            std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize());

        let mut shader: Option<ID3D11VertexShader> = None;
        device.CreateVertexShader(bytes, None, Some(&mut shader))?;
        Ok(shader.unwrap())
    }
}

fn create_pixel_shader(device: &ID3D11Device, blob: &ID3DBlob) -> Result<ID3D11PixelShader> {
    unsafe {
        let bytes =
            std::slice::from_raw_parts(blob.GetBufferPointer() as *const u8, blob.GetBufferSize());

        let mut shader: Option<ID3D11PixelShader> = None;
        device.CreatePixelShader(bytes, None, Some(&mut shader))?;
        Ok(shader.unwrap())
    }
}

fn create_sampler_state(device: &ID3D11Device) -> Result<ID3D11SamplerState> {
    unsafe {
        let desc = D3D11_SAMPLER_DESC {
            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
            ..Default::default()
        };

        let mut sampler: Option<ID3D11SamplerState> = None;
        device.CreateSamplerState(&desc, Some(&mut sampler))?;
        Ok(sampler.unwrap())
    }
}

fn create_constant_buffer(device: &ID3D11Device) -> Result<ID3D11Buffer> {
    unsafe {
        let desc = D3D11_BUFFER_DESC {
            ByteWidth: size_of::<ToneMapCB>() as u32,
            Usage: D3D11_USAGE_DEFAULT,
            BindFlags: D3D11_BIND_CONSTANT_BUFFER.0 as u32,
            CPUAccessFlags: 0,
            MiscFlags: 0,
            StructureByteStride: 0,
        };

        let mut buffer: Option<ID3D11Buffer> = None;
        device.CreateBuffer(&desc, None, Some(&mut buffer))?;
        Ok(buffer.unwrap())
    }
}
