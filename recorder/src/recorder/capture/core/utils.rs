use windows::core::PCWSTR;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::FindWindowW;

pub fn extract_game_name(window_title: &str) -> String {
    window_title
        .split(&['-', '(', ')', '™', '®'][..])
        .next()
        .unwrap_or(window_title)
        .trim()
        .replace(' ', "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_game_name() {
        assert_eq!(extract_game_name("Minecraft - Singleplayer"), "Minecraft");
        assert_eq!(extract_game_name("Counter-Strike 2"), "Counter");
        assert_eq!(extract_game_name("Elden Ring™"), "Elden_Ring");
        assert_eq!(extract_game_name("Game (Paused)"), "Game");
    }
}

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