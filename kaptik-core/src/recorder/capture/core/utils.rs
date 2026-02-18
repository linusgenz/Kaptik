use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;
use windows::core::PCWSTR;


pub fn find_window_hwnd(window_title: &str) -> anyhow::Result<HWND> {
    unsafe {
        let mut title_wide: Vec<u16> = window_title.encode_utf16().collect();
        title_wide.push(0);

        let hwnd = FindWindowW(None, PCWSTR::from_raw(title_wide.as_ptr()))?;

        if hwnd.0.is_null() {
            return Err(anyhow::anyhow!("Window not found: {}", window_title));
        }

        Ok(hwnd)
    }
}
