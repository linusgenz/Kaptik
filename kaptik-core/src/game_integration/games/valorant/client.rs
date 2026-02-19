// client.rs – Thin async wrapper around the Valorant local and remote APIs.
//
// Authentication flow:
//   1. Read the Riot Client lockfile to get port + password.
//   2. GET /entitlements/v1/token  →  access_token, entitlement JWT, PUUID
//   3. GET /riotclient/region-locale  →  region string  →  derive shard
//   4. Optionally GET /product-session/v1/external-sessions → client version
//
// All local endpoints use HTTPS with a self-signed certificate, so the client
// is built with `danger_accept_invalid_certs(true)`.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use reqwest::Client;

use crate::log;
use super::models::*;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// Standard base64-encoded platform header required by all remote Riot APIs.
const CLIENT_PLATFORM: &str =
    "ew0KCSJwbGF0Zm9ybVR5cGUiOiAiUEMiLA0KCSJwbGF0Zm9ybU9TIjogIldpbmRvd3MiLA0\
     KCSJwbGF0Zm9ybU9TVmVyc2lvbiI6ICIxMC4wLjE5MDQyLjEuMjU2LjY0Yml0IiwNCgkicGxh\
     dGZvcm1DaGlwc2V0IjogIlVua25vd24iDQp9";

/// Fallback client version used when auto-detection fails.
/// Update this string when a major new act is released.
const FALLBACK_CLIENT_VERSION: &str = "release-09.10-shipping-30-000000";

// ─────────────────────────────────────────────────────────────────────────────
// Session
// ─────────────────────────────────────────────────────────────────────────────

/// All auth & routing data needed to make Valorant API calls.
///
/// Cheap to clone – the strings are reference-counted internally by `Arc`
/// through the `reqwest::Client`, and all `String` fields are short.
#[derive(Debug, Clone)]
pub struct ClientSession {
    pub puuid: String,
    pub access_token: String,
    pub entitlement_token: String,
    /// Lowercase region code: `"na"`, `"eu"`, `"ap"`, `"kr"`, `"latam"`, `"br"`.
    pub region: String,
    /// Shard for PD/GLZ URL construction. Derived from [`Self::region`].
    pub shard: String,
    pub client_version: String,
    // Only used internally for local API calls.
    pub(super) port: u16,
    pub(super) basic_auth: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// Client
// ─────────────────────────────────────────────────────────────────────────────

/// HTTP client for the Valorant local and remote APIs.
///
/// Uses a single `reqwest::Client` instance (which is `Arc`-backed and
/// connection-pool-aware) so it is safe and efficient to clone.
#[derive(Clone)]
pub struct ValorantClient {
    http: Client,
}

impl ValorantClient {
    pub fn new() -> Result<Self> {
        let http = Client::builder()
            // The Riot Client local HTTPS server uses a self-signed cert.
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .context("Failed to build reqwest client")?;

        Ok(Self { http })
    }

    // ─── Session creation ─────────────────────────────────────────────────────

    /// Reads the lockfile, authenticates, and returns a fully populated
    /// [`ClientSession`].  Fails immediately if the Riot Client is not running.
    pub async fn create_session(&self) -> Result<ClientSession> {
        let (port, password) = Self::read_lockfile()?;
        let basic_auth = BASE64.encode(format!("riot:{}", password));

        // 1. Entitlements (auth)
        let tokens = self.fetch_entitlements(port, &basic_auth).await?;

        // 2. Region
        let (region, shard) = self
            .fetch_region(port, &basic_auth)
            .await
            .unwrap_or_else(|e| {
                log!("⚠️ Valorant: could not detect region ({}), defaulting to na", e);
                ("na".to_string(), "na".to_string())
            });

        // 3. Client version
        let client_version = self
            .fetch_client_version(port, &basic_auth)
            .await
            .unwrap_or_else(|e| {
                log!("⚠️ Valorant: could not detect client version ({}), using fallback", e);
                FALLBACK_CLIENT_VERSION.to_string()
            });

        log!(
            "✅ Valorant session – region={} shard={} version={}",
            region, shard, client_version
        );

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
            .join("Riot Games")
            .join("Riot Client")
            .join("Config")
            .join("lockfile")
    }

    fn read_lockfile() -> Result<(u16, String)> {
        let path = Self::lockfile_path();
        let raw = std::fs::read_to_string(&path)
            .with_context(|| format!("Lockfile not found at {:?} – is Valorant running?", path))?;

        // Format: name:pid:port:password:protocol
        let parts: Vec<&str> = raw.trim().split(':').collect();
        if parts.len() < 5 {
            return Err(anyhow!("Malformed lockfile: expected 5 colon-separated fields"));
        }

        let port = parts[2]
            .parse::<u16>()
            .context("Invalid port number in lockfile")?;
        let password = parts[3].to_string();

        Ok((port, password))
    }

    async fn fetch_entitlements(
        &self,
        port: u16,
        basic_auth: &str,
    ) -> Result<EntitlementsTokenResponse> {
        let url = format!("https://127.0.0.1:{}/entitlements/v1/token", port);
        self.http
            .get(&url)
            .header("Authorization", format!("Basic {}", basic_auth))
            .send()
            .await?
            .json::<EntitlementsTokenResponse>()
            .await
            .context("Failed to parse entitlements token response")
    }

    async fn fetch_region(&self, port: u16, basic_auth: &str) -> Result<(String, String)> {
        // The /riotclient/region-locale endpoint returns {"region":"na","locale":"en_US"}
        // NOTE: The Riot Client may return LoL-style region codes (e.g. "euw", "eune", "na1")
        // rather than Valorant-style codes ("eu", "na"). We normalise them here.
        let url = format!("https://127.0.0.1:{}/riotclient/region-locale", port);
        let resp: serde_json::Value = self.http
            .get(&url)
            .header("Authorization", format!("Basic {}", basic_auth))
            .send()
            .await?
            .json()
            .await?;

        let raw = resp
            .get("region")
            .and_then(|v| v.as_str())
            .unwrap_or("na")
            .to_lowercase();

        // Normalise to Valorant region codes:
        // Strip trailing digits (na1 → na, br1 → br, tr1 → tr)
        // Map LoL sub-regions to Valorant regions
        let region = normalize_region(&raw);
        let shard = region_to_shard(&region).to_string();

        log!("🌍 Valorant region: raw='{}' → normalized='{}' shard='{}'", raw, region, shard);
        Ok((region, shard))
    }

    async fn fetch_client_version(&self, port: u16, basic_auth: &str) -> Result<String> {
        // /product-session/v1/external-sessions returns a map of product → session.
        // Each session has launchConfiguration.arguments, one of which looks like
        // "-ShooterGameVersion=release-09.10-shipping-30-600000".
        let url = format!(
            "https://127.0.0.1:{}/product-session/v1/external-sessions",
            port
        );
        /*let resp: serde_json::Value = self.http
            .get(&url)
            .header("Authorization", format!("Basic {}", basic_auth))
            .send()
            .await?
            .json()
            .await?;*/
//
        let resp = self.http
            .get(&url)
            .header("Authorization", format!("Basic {}", basic_auth))
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        log!("CLIENT_VERSION {} -> {}\n{}", url, status, text);
        let resp: serde_json::Value = serde_json::from_str(&text)?;
//

        if let Some(obj) = resp.as_object() {
            for (_, session) in obj {
                if let Some(args) = session
                    .get("launchConfiguration")
                    .and_then(|lc| lc.get("arguments"))
                    .and_then(|a| a.as_array())
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

        Err(anyhow!("ShooterGameVersion argument not found in session"))
    }

    // ─── Local API calls ──────────────────────────────────────────────────────

    /// Fetches the local presence list and decodes the current player's own
    /// Valorant presence private blob.
    pub async fn get_own_presence(&self, session: &ClientSession) -> Result<PresencePrivate> {
        let url = format!("https://127.0.0.1:{}/chat/v4/presences", session.port);
        let resp = self.http
            .get(&url)
            .header("Authorization", format!("Basic {}", session.basic_auth))
            .send()
            .await?
            .json::<PresencesResponse>()
            .await?;

        let own = resp
            .presences
            .iter()
            .find(|p| p.puuid == session.puuid && p.product == "valorant")
            .ok_or_else(|| anyhow!("Own Valorant presence not found in list"))?;

        let private_b64 = own
            .private
            .as_deref()
            .ok_or_else(|| anyhow!("Presence private is null (not in game?)"))?;

        let decoded = BASE64
            .decode(private_b64.as_bytes())
            .context("Failed to base64-decode presence private")?;

        serde_json::from_slice::<PresencePrivate>(&decoded)
            .context("Failed to parse presence private JSON")
    }

    // ─── GLZ (Current-Game) calls ─────────────────────────────────────────────

    /// Returns the match the player is currently in, or an error if not in-game.
    pub async fn get_current_game_player(
        &self,
        session: &ClientSession,
    ) -> Result<CoreGamePlayerResponse> {
        let url = format!(
            "https://glz-{}-1.{}.a.pvp.net/core-game/v1/players/{}",
            session.region, session.shard, session.puuid
        );
      /*  let resp = self
            .http
            .get(&url)
            .headers(self.remote_headers(session))
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            return Err(anyhow!("CoreGame_FetchPlayer returned {}", status));
        }*/
//
        let resp = self
            .http
            .get(&url)
            .headers(self.remote_headers(session))
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        log!("CORE_GAME_PLAYER {} -> {}\n{}", url, status, text);

        if !status.is_success() {
            return Err(anyhow!("CoreGame_FetchPlayer returned {}", status));
        }

        serde_json::from_str(&text)
            .context("Failed to parse CoreGame player response")
//

       /* resp.json::<CoreGamePlayerResponse>()
            .await
            .context("Failed to parse CoreGame player response")*/
    }

    /// Returns details for an in-progress match (map, mode, player list, agents).
    pub async fn get_current_game_match(
        &self,
        session: &ClientSession,
        match_id: &str,
    ) -> Result<CoreGameMatchResponse> {
        let url = format!(
            "https://glz-{}-1.{}.a.pvp.net/core-game/v1/matches/{}",
            session.region, session.shard, match_id
        );
        self.http
            .get(&url)
            .headers(self.remote_headers(session))
            .send()
            .await?
            .json::<CoreGameMatchResponse>()
            .await
            .context("Failed to parse CoreGame match response")
    }

    // ─── PD (Player-Data) calls ───────────────────────────────────────────────

    /// Returns the most recent match in the player's history.
    pub async fn get_latest_match_id(
        &self,
        session: &ClientSession,
    ) -> Result<Option<String>> {
        let url = format!(
            "https://pd.{}.a.pvp.net/match-history/v1/history/{}?startIndex=0&endIndex=1",
            session.shard, session.puuid
        );
        let resp = self.http
            .get(&url)
            .headers(self.remote_headers(session))
            .send()
            .await?
            .json::<MatchHistoryResponse>()
            .await
            .context("Failed to parse match history response")?;

        Ok(resp.history.into_iter().next().map(|h| h.match_id))
    }

    /// Returns the full details for a completed match.
    pub async fn get_match_details(
        &self,
        session: &ClientSession,
        match_id: &str,
    ) -> Result<MatchDetailsResponse> {
        let url = format!(
            "https://pd.{}.a.pvp.net/match-details/v1/matches/{}",
            session.shard, match_id
        );
        self.http
            .get(&url)
            .headers(self.remote_headers(session))
            .send()
            .await?
            .json::<MatchDetailsResponse>()
            .await
            .context("Failed to parse match details response")
    }

    // ─── Header builder ───────────────────────────────────────────────────────

    fn remote_headers(&self, session: &ClientSession) -> reqwest::header::HeaderMap {
        let mut map = reqwest::header::HeaderMap::new();
        map.insert(
            "Authorization",
            format!("Bearer {}", session.access_token)
                .parse()
                .expect("valid header value"),
        );
        map.insert(
            "X-Riot-Entitlements-JWT",
            session.entitlement_token.parse().expect("valid header value"),
        );
        map.insert(
            "X-Riot-ClientVersion",
            session.client_version.parse().expect("valid header value"),
        );
        map.insert(
            "X-Riot-ClientPlatform",
            CLIENT_PLATFORM.parse().expect("valid header value"),
        );
        map
    }
}

// Region helpers

/// The `/riotclient/region-locale` endpoint returns LoL sub-region codes for
/// users whose primary game is League of Legends (e.g. `"euw"`, `"eune"`,
/// `"na1"`, `"tr1"`, `"br1"`). This function maps them to the Valorant
/// equivalents that GLZ and PD URLs expect.
fn normalize_region(raw: &str) -> String {
    // Strip trailing digits: "na1" → "na", "br1" → "br", "tr1" → "tr"
    let stripped = raw.trim_end_matches(|c: char| c.is_ascii_digit());

    match stripped {
        // EU sub-regions → "eu"
        "euw" | "eune" | "tr" | "ru" => "eu".to_string(),
        // NA sub-region → "na"
        "na" => "na".to_string(),
        // BR & LATAM share the NA shard but keep their region identity in the URL
        "br" => "br".to_string(),
        "latam" => "latam".to_string(),
        // OCE and JP fold into AP
        "oce" | "jp" => "ap".to_string(),
        // Pass through known Valorant codes unchanged
        other => other.to_string(),
    }
}

/// Maps a normalised Valorant region code to the shard used in PD / GLZ URLs.
///
/// | Region        | Shard |
/// |---------------|-------|
/// | na            | na    |
/// | latam         | na    |
/// | br            | na    |
/// | eu            | eu    |
/// | ap            | ap    |
/// | kr            | kr    |
fn region_to_shard(region: &str) -> &'static str {
    match region {
        "na" | "latam" | "br" => "na",
        "eu" => "eu",
        "ap" => "ap",
        "kr" => "kr",
        _ => "na",
    }
}