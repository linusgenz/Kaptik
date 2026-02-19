// detector.rs

//! The main detection loop and window enumeration.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, TRUE};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_INFORMATION, PROCESS_VM_READ,
};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible,
};
use windows::core::{BOOL, PWSTR};
use crate::game_detection::DetectionEvent;
use super::heuristics::{is_fullscreen, is_likely_game};
use super::process::GameProcess;

type Callback = Arc<dyn Fn(DetectionEvent) + Send + Sync>;

pub struct GameDetector {
    known_games: Arc<RwLock<HashMap<u32, GameProcess>>>,
    callback: Option<Callback>,
}

impl GameDetector {
    pub fn new() -> Self {
        Self {
            known_games: Arc::new(RwLock::new(HashMap::new())),
            callback: None,
        }
    }

    /// Register the callback invoked on every [`DetectionEvent`].
    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(DetectionEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));
    }

    /// Start a background Tokio task that polls running processes every 2 s.
    pub async fn start_monitoring(&self) {
        let known_games = self.known_games.clone();
        let callback = self.callback.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));

            loop {
                interval.tick().await;

                let Ok(processes) = Self::detect_games() else { continue };

                let mut games = known_games.write().await;

                // Newly appeared processes.
                for process in &processes {
                    if !games.contains_key(&process.pid) {
                        games.insert(process.pid, process.clone());
                        if let Some(cb) = &callback {
                            cb(DetectionEvent::GameStarted(process.clone()));
                        }
                    }
                }

                // Processes that have gone away.
                let current_pids: Vec<u32> = processes.iter().map(|p| p.pid).collect();
                let gone: Vec<u32> = games
                    .keys()
                    .filter(|pid| !current_pids.contains(pid))
                    .cloned()
                    .collect();

                for pid in gone {
                    if let Some(game) = games.remove(&pid) {
                        if let Some(cb) = &callback {
                            cb(DetectionEvent::GameStopped(game.name.clone()));
                        }
                    }
                }
            }
        });
    }

    /// Enumerate all visible top-level windows and return those that pass
    /// the game heuristics.
    fn detect_games() -> anyhow::Result<Vec<GameProcess>> {
        let mut games = Vec::new();
        unsafe {
            EnumWindows(
                Some(enum_windows_callback),
                LPARAM(&mut games as *mut _ as isize),
            )?;
        }
        Ok(games)
    }

    /// Try to classify a single window as a game process.
    pub(super) fn is_game_process(hwnd: HWND) -> Option<GameProcess> {
        unsafe {
            if !IsWindowVisible(hwnd).as_bool() {
                return None;
            }

            let mut title = vec![0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len == 0 { return None; }
            let window_title = String::from_utf16_lossy(&title[..len as usize]);

            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if pid == 0 { return None; }

            let process = OpenProcess(
                PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                false,
                pid,
            ).ok()?;

            let mut exe_path = vec![0u16; 1024];
            let mut size = exe_path.len() as u32;
            let pwstr = PWSTR::from_raw(exe_path.as_mut_ptr());

            let result = QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, pwstr, &mut size);
            let _ = CloseHandle(process);

            result.ok()?;

            let path = String::from_utf16_lossy(&exe_path[..size as usize]);
            let exe_name = std::path::Path::new(&path)
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown.exe")
                .to_string();

            if is_likely_game(hwnd, &exe_name, &window_title, &path) {
                Some(GameProcess {
                    name: exe_name,
                    exe_path: path,
                    pid,
                    window_title,
                    is_fullscreen: is_fullscreen(hwnd),
                })
            } else {
                None
            }
        }
    }

    /// Returns all currently tracked games (snapshot).
    pub async fn get_active_games(&self) -> Vec<GameProcess> {
        self.known_games.read().await.values().cloned().collect()
    }
}

impl Default for GameDetector {
    fn default() -> Self { Self::new() }
}

extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    unsafe {
        let games = &mut *(lparam.0 as *mut Vec<GameProcess>);
        if let Some(game) = GameDetector::is_game_process(hwnd) {
            games.push(game);
        }
        TRUE
    }
}