use std::time::Instant;

use windows::Win32::Graphics::Direct3D11::ID3D11Texture2D;

pub(crate) struct CapturedFrame {
    pub(crate) texture: ID3D11Texture2D,
    pub(crate) timestamp: Instant,
}
