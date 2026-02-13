use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Dwm::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::Threading::*;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::core::{BOOL, PWSTR};

#[derive(Debug, Clone)]
pub struct GameProcess {
    pub name: String,
    pub exe_path: String,
    pub pid: u32,
    pub window_title: String,
    pub is_fullscreen: bool,
}

pub struct GameDetector {
    known_games: Arc<RwLock<HashMap<u32, GameProcess>>>,
    callback: Option<Arc<dyn Fn(GameEvent) + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub enum GameEvent {
    GameStarted(GameProcess),
    GameStopped(String), // exe name
    GameFocused(String),
    GameUnfocused(String),
}

impl GameDetector {
    pub fn new() -> Self {
        Self {
            known_games: Arc::new(RwLock::new(HashMap::new())),
            callback: None,
        }
    }

    pub fn set_callback<F>(&mut self, callback: F)
    where
        F: Fn(GameEvent) + Send + Sync + 'static,
    {
        self.callback = Some(Arc::new(callback));
    }

    pub async fn start_monitoring(&self) {
        let known_games = self.known_games.clone();
        let callback = self.callback.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(2));

            loop {
                interval.tick().await;

                if let Ok(processes) = Self::detect_games() {
                    let mut games = known_games.write().await;

                    for process in &processes {
                        if !games.contains_key(&process.pid) {
                            games.insert(process.pid, process.clone());

                            if let Some(ref cb) = callback {
                                cb(GameEvent::GameStarted(process.clone()));
                            }
                        }
                    }

                    // Gestoppte Games finden
                    let current_pids: Vec<u32> = processes.iter().map(|p| p.pid).collect();

                    let stopped_pids: Vec<u32> = games
                        .keys()
                        .filter(|pid| !current_pids.contains(pid))
                        .cloned()
                        .collect();

                    for pid in stopped_pids {
                        if let Some(game) = games.remove(&pid) {
                            if let Some(ref cb) = callback {
                                cb(GameEvent::GameStopped(game.name.clone()));
                            }
                        }
                    }
                }
            }
        });
    }

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

    /// Prüft ob ein Prozess ein Game ist
    fn is_game_process(hwnd: HWND) -> Option<GameProcess> {
        unsafe {
            // Nur sichtbare Fenster
            if !IsWindowVisible(hwnd).as_bool() {
                return None;
            }

            // Window-Titel holen
            let mut title = vec![0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title);
            if len == 0 {
                return None;
            }
            let window_title = String::from_utf16_lossy(&title[..len as usize]);

            // Prozess-ID holen
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));

            if pid == 0 {
                return None;
            }

            // Prozess-Handle öffnen
            let process =
                OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_VM_READ, false, pid).ok()?;

            // Exe-Pfad holen
            let mut exe_path = vec![0u16; 1024];
            let mut size = exe_path.len() as u32;

            let pwstr = PWSTR::from_raw(exe_path.as_mut_ptr());

            if QueryFullProcessImageNameW(process, PROCESS_NAME_WIN32, pwstr, &mut size).is_ok() {
                let path = String::from_utf16_lossy(&exe_path[..size as usize]);
                let exe_name = std::path::Path::new(&path)
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown.exe")
                    .to_string();

                let _ = CloseHandle(process);

                // Game-Heuristiken
                if Self::is_likely_game(hwnd, &exe_name, &window_title, &path) {
                    let is_fullscreen = Self::is_fullscreen(hwnd);

                    return Some(GameProcess {
                        name: exe_name,
                        exe_path: path,
                        pid,
                        window_title,
                        is_fullscreen,
                    });
                }
            }

            let _ = CloseHandle(process);
        }

        None
    }

    fn is_likely_game(hwnd: HWND, exe_name: &str, title: &str, path: &str) -> bool {
        let exe_lower = exe_name.to_lowercase();
        let path_lower = path.to_lowercase();
        let title_lower = title.to_lowercase();

        // Ausschlussliste
        let excluded_processes = [
            "rustrover64.exe",
            "code.exe",
            "devenv.exe",
            "rider64.exe",
            "qtcreator.exe",
            "clion64.exe",
            "idea64.exe",
            "pycharm64.exe",
            "sublime_text.exe",
            "notepad++.exe",
            "atom.exe",
            "chrome.exe",
            "firefox.exe",
            "msedge.exe",
            "brave.exe",
            "opera.exe",
            "explorer.exe",
            "applicationframehost.exe",
            "textinputhost.exe",
            "shellexperiencehost.exe",
            "searchhost.exe",
            "startmenuexperiencehost.exe",
            "discord.exe",
            "slack.exe",
            "teams.exe",
            "zoom.exe",
            "skype.exe",
            "spotify.exe",
            "vlc.exe",
            "obs64.exe",
            "streamlabs obs.exe",
            "overwolf.exe",
            "nvidia overlay.exe",
            "nvidia share.exe",
            "steam.exe",
            "epicgameslauncher.exe",
            "origin.exe",
            "uplay.exe",
            "battle.net.exe",
            "ea desktop.exe",
            "gamebar.exe",
            "xboxgamebarwidgets.exe",
            "winword.exe",
            "excel.exe",
            "powerpnt.exe",
            "outlook.exe",
            "taskmgr.exe",
            "mmc.exe",
            "cmd.exe",
            "powershell.exe",
            "SnippingTool.exe",
        ];

        if excluded_processes.iter().any(|e| exe_lower.contains(e)) {
            return false;
        }

        let launcher_exes = [
            "leagueclientux.exe",
            "leagueclient.exe",
            "riotclientservices.exe",
            "Riot Client.exe",
            "RiotClientServices.exe"
        ];
        if launcher_exes.iter().any(|e| exe_lower.contains(e)) {
            return false;
        }

        let excluded_title_patterns = [
            "qt creator",
            "visual studio",
            "rustrover",
            "intellij",
            "pycharm",
            "chrome",
            "firefox",
            "edge",
            "discord",
            "spotify",
            "obs",
            "windows input",
            "nvidia overlay",
            "task manager",
            "settings",
            "control panel",
        ];

        if excluded_title_patterns
            .iter()
            .any(|p| title_lower.contains(p))
        {
            return false;
        }

        // Pfad-basierte Ausschlüsse
        let excluded_paths = [
            "program files\\jetbrains",
            "program files\\microsoft",
            "\\nvidia corporation\\",
            "\\overwolf\\",
            "appdata\\local\\programs",
            "windows\\system32",
            "windows\\syswow64",
            "windows\\winsxs",
            "program files\\microsoft",
            "program files (x86)\\microsoft",
            "program files\\windowsapps",
            "program files (x86)\\windowsapps",
        ];

        if excluded_paths.iter().any(|p| path_lower.contains(p)) {
            return false;
        }

        // Punkte-Logik für echte Spiele
        let mut score = 0;

        let game_paths = [
            "\\steamapps\\common\\",
            "\\epic games\\",
            "\\riot games\\",
            "\\ubisoft\\",
            "\\ea games\\",
            "\\gog galaxy\\games\\",
            "\\xbox games\\",
            "\\battle.net\\",
        ];

        if game_paths.iter().any(|p| path_lower.contains(p)) {
            score += 2;
        }

        if exe_lower.contains("game") && !exe_lower.contains("launcher") {
            score += 1;
        }

        let engine_indicators = ["unreal", "unity", "cryengine", "frostbite", "source2"];
        if engine_indicators.iter().any(|e| exe_lower.contains(e)) {
            score += 2;
        }

        if Self::is_fullscreen(hwnd) {
            score += 1;
        }

        if { Self::has_d3d11_or_vulkan() } {
            score += 1;
        }

        let system_title_patterns = ["windows", "microsoft", "nvidia", "amd", "intel"];
        if !system_title_patterns
            .iter()
            .any(|p| title_lower.contains(p))
        {
            score += 1;
        }

        score >= 3
    }

    fn is_fullscreen(hwnd: HWND) -> bool {
        unsafe {
            let mut rect = RECT::default();
            if GetWindowRect(hwnd, &mut rect).is_err() {
                return false;
            }

            let width = rect.right - rect.left;
            let height = rect.bottom - rect.top;

            let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTOPRIMARY);
            let mut monitor_info = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            if GetMonitorInfoW(monitor, &mut monitor_info).as_bool() {
                let monitor_width = monitor_info.rcMonitor.right - monitor_info.rcMonitor.left;
                let monitor_height = monitor_info.rcMonitor.bottom - monitor_info.rcMonitor.top;

                return width >= monitor_width && height >= monitor_height;
            }

            false
        }
    }

    fn has_d3d11_or_vulkan() -> bool {
        unsafe {
            match DwmIsCompositionEnabled() {
                Ok(enabled) => enabled.as_bool(),
                Err(_) => false,
            }
        }
    }

    pub async fn get_active_games(&self) -> Vec<GameProcess> {
        self.known_games.read().await.values().cloned().collect()
    }
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
