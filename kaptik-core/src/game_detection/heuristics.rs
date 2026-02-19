// heuristics.rs

//! Heuristics to decide whether a visible window belongs to a game.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Dwm::DwmIsCompositionEnabled;
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTOPRIMARY, MonitorFromWindow,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

// Exclusion lists

const EXCLUDED_EXES: &[&str] = &[
    "rustrover64.exe", "code.exe", "devenv.exe", "rider64.exe", "qtcreator.exe",
    "clion64.exe", "idea64.exe", "pycharm64.exe", "sublime_text.exe", "notepad++.exe",
    "atom.exe", "chrome.exe", "firefox.exe", "msedge.exe", "brave.exe", "opera.exe",
    "explorer.exe", "applicationframehost.exe", "textinputhost.exe",
    "shellexperiencehost.exe", "searchhost.exe", "startmenuexperiencehost.exe",
    "discord.exe", "slack.exe", "teams.exe", "zoom.exe", "skype.exe",
    "spotify.exe", "vlc.exe", "obs64.exe", "streamlabs obs.exe", "overwolf.exe",
    "nvidia overlay.exe", "nvidia share.exe", "steam.exe", "epicgameslauncher.exe",
    "origin.exe", "uplay.exe", "battle.net.exe", "ea desktop.exe",
    "gamebar.exe", "xboxgamebarwidgets.exe",
    "winword.exe", "excel.exe", "powerpnt.exe", "outlook.exe",
    "taskmgr.exe", "mmc.exe", "cmd.exe", "powershell.exe", "SnippingTool.exe",
];

const LAUNCHER_EXES: &[&str] = &[
    "leagueclientux.exe", "leagueclient.exe", "riotclientservices.exe",
    "riot client.exe", "riotclientservices.exe",
];

const EXCLUDED_TITLE_PATTERNS: &[&str] = &[
    "qt creator", "visual studio", "rustrover", "intellij", "pycharm",
    "chrome", "firefox", "edge", "discord", "spotify", "obs",
    "windows input", "nvidia overlay", "task manager", "settings", "control panel",
];

const EXCLUDED_PATHS: &[&str] = &[
    "program files\\jetbrains", "program files\\microsoft",
    "\\nvidia corporation\\", "\\overwolf\\",
    "appdata\\local\\programs", "windows\\system32",
    "windows\\syswow64", "windows\\winsxs",
    "program files (x86)\\microsoft", "program files\\windowsapps",
    "program files (x86)\\windowsapps",
];

const GAME_PATHS: &[&str] = &[
    "\\steamapps\\common\\", "\\epic games\\", "\\riot games\\",
    "\\ubisoft\\", "\\ea games\\", "\\gog galaxy\\games\\",
    "\\xbox games\\", "\\battle.net\\",
];

const ENGINE_INDICATORS: &[&str] = &["unreal", "unity", "cryengine", "frostbite", "source2"];

const SYSTEM_TITLE_PATTERNS: &[&str] = &["windows", "microsoft", "nvidia", "amd", "intel"];

// Public API

/// Returns `true` when the window / process combination looks like a game.
///
/// Uses a simple scoring heuristic:
/// - Instant-reject: known non-game executables, launchers, title patterns, paths
/// - Scoring: game install paths (+2), engine name in exe (+2), fullscreen (+1),
///   D3D composition (+1), neutral window title (+1)
/// - Threshold: score ≥ 3
pub fn is_likely_game(hwnd: HWND, exe_name: &str, title: &str, path: &str) -> bool {
    let exe_lower = exe_name.to_lowercase();
    let path_lower = path.to_lowercase();
    let title_lower = title.to_lowercase();

    if EXCLUDED_EXES.iter().any(|e| exe_lower.contains(e)) { return false; }
    if LAUNCHER_EXES.iter().any(|e| exe_lower.contains(e)) { return false; }
    if EXCLUDED_TITLE_PATTERNS.iter().any(|p| title_lower.contains(p)) { return false; }
    if EXCLUDED_PATHS.iter().any(|p| path_lower.contains(p)) { return false; }

    let mut score = 0i32;

    if GAME_PATHS.iter().any(|p| path_lower.contains(p)) { score += 2; }
    if exe_lower.contains("game") && !exe_lower.contains("launcher") { score += 1; }
    if ENGINE_INDICATORS.iter().any(|e| exe_lower.contains(e)) { score += 2; }
    if is_fullscreen(hwnd) { score += 1; }
    if has_d3d_composition() { score += 1; }
    if !SYSTEM_TITLE_PATTERNS.iter().any(|p| title_lower.contains(p)) { score += 1; }

    score >= 3
}

/// Returns `true` when the window covers the entire primary monitor.
pub fn is_fullscreen(hwnd: HWND) -> bool {
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(hwnd, &mut rect).is_err() { return false; }

        let width  = rect.right  - rect.left;
        let height = rect.bottom - rect.top;

        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
        let mut info = MONITORINFO {
            cbSize: size_of::<MONITORINFO>() as u32,
            ..Default::default()
        };

        if GetMonitorInfoW(monitor, &mut info).as_bool() {
            let mw = info.rcMonitor.right  - info.rcMonitor.left;
            let mh = info.rcMonitor.bottom - info.rcMonitor.top;
            return width >= mw && height >= mh;
        }

        false
    }
}

/// Checks whether DWM composition (a proxy for D3D/graphics activity) is enabled.
fn has_d3d_composition() -> bool {
    unsafe {
        DwmIsCompositionEnabled()
            .map(|b| b.as_bool())
            .unwrap_or(false)
    }
}