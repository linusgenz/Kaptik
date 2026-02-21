// client.rs – Thin async wrapper around the Valorant local and remote APIs.
//
// Authentication flow:
//   1. Read the Riot Client lockfile to get port + password.
//   2. GET /entitlements/v1/token  →  access_token, entitlement JWT, PUUID
//   3. GET /riotclient/region-locale  →  region string  →  derive shard
//   4. GET /product-session/v1/external-sessions  →  client version

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use reqwest::Client;

use crate::log;
use super::models::*;

use std::fs::OpenOptions;
use std::io::Write;

fn log_file_path() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("valorant_debug.log")
}

fn flog(msg: &str) {
    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
    let line = format!("[{}] {}\n", timestamp, msg);
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_file_path()) {
        let _ = file.write_all(line.as_bytes());
        let _ = file.flush();
    }
}

macro_rules! flog {
    ($($arg:tt)*) => { flog(&format!($($arg)*)); };
}

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

const CLIENT_PLATFORM: &str =
    "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0\
     KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxh\
     dGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";

const FALLBACK_CLIENT_VERSION: &str = "release-12.02-shipping-9-4226954";

// ─────────────────────────────────────────────────────────────────────────────
// Session
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct ClientSession {
    pub puuid: String,
    pub access_token: String,
    pub entitlement_token: String,
    pub region: String,
    pub shard: String,
    pub client_version: String,
    pub(super) port: u16,
    pub(super) basic_auth: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Client
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct ValorantClient {
    http: Client,
}

impl ValorantClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .context("Failed to build reqwest client")?;
        Ok(Self { http })
    }

    // ─── Session creation ─────────────────────────────────────────────────────

    pub async fn create_session(&self) -> Result<ClientSession> {
        let (port, password) = Self::read_lockfile()?;
        let basic_auth = BASE64.encode(format!("riot:{}", password));

        let tokens = self.fetch_entitlements(port, &basic_auth).await?;

        let (region, shard) = self.fetch_region(port, &basic_auth).await.unwrap_or_else(|e| {
            flog!("[SESSION] ⚠️ Region fetch failed: {} – defaulting to na/na", e);
            log!("⚠️ Valorant: could not detect region ({}), defaulting to na", e);
            ("na".to_string(), "na".to_string())
        });

        let client_version = self.fetch_client_version(port, &basic_auth).await.unwrap_or_else(|e| {
            flog!("[SESSION] ⚠️ Client version fetch failed: {} – using fallback", e);
            log!("⚠️ Valorant: could not detect client version ({}), using fallback", e);
            FALLBACK_CLIENT_VERSION.to_string()
        });

        flog!("[SESSION] ✅ PUUID={} region={} shard={} version={}", tokens.subject, region, shard, client_version);
        log!("✅ Valorant session – region={} shard={} version={}", region, shard, client_version);

        Ok(ClientSession {
            puuid: tokens.subject,
            access_token: tokens.access_token,
            entitlement_token: tokens.entitlement_token,
            region,
            shard,
            client_version,
            port,
            basic_auth,
        })
    }

    // ─── Private helpers ──────────────────────────────────────────────────────

    fn lockfile_path() -> PathBuf {
        let local_app_data = std::env::var("LOCALAPPDATA")
            .unwrap_or_else(|_| r"C:\Users\User\AppData\Local".to_string());
        PathBuf::from(local_app_data)
            .join("Riot Games").join("Riot Client").join("Config").join("lockfile")
    }

    fn read_lockfile() -> Result<(u16, String)> {
        let path = Self::lockfile_path();
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Lockfile not found at {:?} – is Valorant running?", path))?;
        let parts: Vec<&str> = raw.trim().split(':').collect();
        if parts.len() < 5 {
            return Err(anyhow!("Malformed lockfile: expected 5 colon-separated fields"));
        }
        let port = parts[2].parse::<u16>().context("Invalid port in lockfile")?;
        Ok((port, parts[3].to_string()))
    }

    async fn fetch_entitlements(&self, port: u16, basic_auth: &str) -> Result<EntitlementsTokenResponse> {
        let url = format!("https://127.0.0.1:{}/entitlements/v1/token", port);
        let resp = self.http.get(&url)
            .header("Authorization", format!("Basic {}", basic_auth))
            .send().await?;
        if !resp.status().is_success() {
            return Err(anyhow!("Entitlements returned HTTP {}", resp.status()));
        }
        resp.json::<EntitlementsTokenResponse>().await.context("Failed to parse entitlements response")
    }

    async fn fetch_region(&self, port: u16, basic_auth: &str) -> Result<(String, String)> {
        let url = format!("https://127.0.0.1:{}/riotclient/region-locale", port);
        let text = self.http.get(&url)
            .header("Authorization", format!("Basic {}", basic_auth))
            .send().await?.text().await?;
        let val: serde_json::Value = serde_json::from_str(&text)?;
        let raw = val.get("region").and_then(|v| v.as_str()).unwrap_or("na").to_lowercase();
        let region = normalize_region(&raw);
        let shard = region_to_shard(&region).to_string();
        Ok((region, shard))
    }

    async fn fetch_client_version(&self, port: u16, basic_auth: &str) -> Result<String> {
        let url = format!("https://127.0.0.1:{}/product-session/v1/external-sessions", port);
        let text = self.http.get(&url)
            .header("Authorization", format!("Basic {}", basic_auth))
            .send().await?.text().await?;
        let val: serde_json::Value = serde_json::from_str(&text)?;
        if let Some(obj) = val.as_object() {
            for (_key, session) in obj {
                if let Some(args) = session
                    .get("launchConfiguration").and_then(|lc| lc.get("arguments")).and_then(|a| a.as_array())
                {
                    for arg in args {
                        if let Some(s) = arg.as_str() {
                            if let Some(version) = s.strip_prefix("-ShooterGameVersion=") {
                                return Ok(version.to_string());
                            }
                        }
                    }
                }
            }
        }
        Err(anyhow!("ShooterGameVersion not found in session"))
    }

    // ─── GLZ (Core-Game) calls ────────────────────────────────────────────────

    pub async fn get_current_game_player(&self, session: &ClientSession) -> Result<CurrentGamePlayerResponse> {
        let url = format!(
            "https://glz-{}-1.{}.a.pvp.net/core-game/v1/players/{}",
            session.region, session.shard, session.puuid
        );
        let resp = self.http.get(&url).headers(self.remote_headers(session)).send().await?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            return Err(anyhow!("core-game/v1/players returned {}", status));
        }
        serde_json::from_str(&text).context("Failed to parse CurrentGamePlayerResponse")
    }


    pub async fn get_current_game_match(&self, session: &ClientSession, match_id: &str) -> Result<CurrentGameMatchResponse> {
        let url = format!(
            "https://glz-{}-1.{}.a.pvp.net/core-game/v1/matches/{}",
            session.region, session.shard, match_id
        );
        let resp = self.http.get(&url).headers(self.remote_headers(session)).send().await?;
        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            flog!("[CORE_GAME] ❌ core-game/v1/matches/{} returned {} – {}", match_id, status, &body[..body.len().min(100)]);
            return Err(anyhow!("core-game/v1/matches returned {}", status));
        }

        let struct1 = resp.json::<CurrentGameMatchResponse>().await.context("Failed to parse CurrentGameMatchResponse");
        flog!("GAMEMATCH RESPONSE BODY: {:?}", &struct1);
        struct1
    }

    // ─── PD (Player-Data) calls ───────────────────────────────────────────────

    pub async fn get_latest_match_id(&self, session: &ClientSession) -> Result<Option<String>> {
        let url = format!(
            "https://pd.{}.a.pvp.net/match-history/v1/history/{}?startIndex=0&endIndex=1",
            session.shard, session.puuid
        );
        let resp = self.http.get(&url).headers(self.remote_headers(session)).send().await?;
        let history = resp.json::<MatchHistoryResponse>().await.context("Failed to parse match history")?;
        Ok(history.history.into_iter().next().map(|h| h.match_id))
    }

    pub async fn get_match_details(&self, session: &ClientSession, match_id: &str) -> Result<MatchDetailsResponse> {
        let url = format!(
            "https://pd.{}.a.pvp.net/match-details/v1/matches/{}",
            session.shard, match_id
        );
        let resp = self.http.get(&url).headers(self.remote_headers(session)).send().await?;
        resp.json::<MatchDetailsResponse>().await.context("Failed to parse match details")
    }

    // ─── Header builder ───────────────────────────────────────────────────────

    fn remote_headers(&self, session: &ClientSession) -> reqwest::header::HeaderMap {
        let mut map = reqwest::header::HeaderMap::new();
        map.insert("Authorization", format!("Bearer {}", session.access_token).parse().unwrap());
        map.insert("X-Riot-Entitlements-JWT", session.entitlement_token.parse().unwrap());
        map.insert("X-Riot-ClientVersion", session.client_version.parse().unwrap());
        map.insert("X-Riot-ClientPlatform", CLIENT_PLATFORM.parse().unwrap());
        map
    }
}

// ─── Region helpers ───────────────────────────────────────────────────────────

fn normalize_region(raw: &str) -> String {
    let stripped = raw.trim_end_matches(|c: char| c.is_ascii_digit());
    match stripped {
        "euw" | "eune" | "tr" | "ru" => "eu".to_string(),
        "na" => "na".to_string(),
        "br" => "br".to_string(),
        "latam" => "latam".to_string(),
        "oce" | "jp" => "ap".to_string(),
        other => other.to_string(),
    }
}

fn region_to_shard(region: &str) -> &'static str {
    match region {
        "na" | "latam" | "br" => "na",
        "eu" => "eu",
        "ap" => "ap",
        "kr" => "kr",
        _ => "na",
    }
}