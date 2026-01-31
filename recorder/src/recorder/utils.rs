use windows::core::Interface;
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct3D11::ID3D11Device;
use windows::Win32::Graphics::Dxgi::{IDXGIDevice, IDXGIOutput6};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020, DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709};
use crate::log;
use anyhow::Result;

pub fn is_hdr_enabled(hwnd: HWND, d3d_device: &ID3D11Device) -> bool {
    unsafe {
        use windows::Win32::Graphics::Gdi::{MonitorFromWindow, MONITOR_DEFAULTTONEAREST};
        use windows::core::Interface;

        let hmonitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);

        let dxgi_device: IDXGIDevice = match d3d_device.cast() {
            Ok(dev) => dev,
            Err(e) => {
                log!("⚠️ Failed to cast D3D device: {}", e);
                return false;
            }
        };

        let adapter = match dxgi_device.GetAdapter() {
            Ok(a) => a,
            Err(e) => {
                log!("⚠️ Failed to get adapter: {}", e);
                return false;
            }
        };

        let mut output_index = 0;
        loop {
            match adapter.EnumOutputs(output_index) {
                Ok(output) => {
                    let desc = match output.GetDesc() {
                        Ok(d) => d,
                        Err(e) => {
                            log!("⚠️ Failed to get output desc: {}", e);
                            return false;
                        }
                    };

                    if desc.Monitor == hmonitor {
                        let output6: IDXGIOutput6 = match output.cast() {
                            Ok(o) => o,
                            Err(_) => return false,
                        };

                        let desc1 = match output6.GetDesc1() {
                            Ok(d1) => d1,
                            Err(e) => {
                                log!("⚠️ GetDesc1 failed: {}", e);
                                return false;
                            }
                        };

                        return matches!(
                            desc1.ColorSpace,
                            DXGI_COLOR_SPACE_RGB_FULL_G2084_NONE_P2020 |
                            DXGI_COLOR_SPACE_RGB_FULL_G10_NONE_P709
                        );
                    }

                    output_index += 1;
                }
                Err(_) => {
                    log!("⚠️ Could not find monitor for window");
                    return false;
                }
            }
        }
    }
}
