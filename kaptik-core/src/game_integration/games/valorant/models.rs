// models.rs

// models.rs – Valorant local & remote API response types.
//
// All field names follow the casing used by the Valorant API, using
// `#[serde(rename = "...")]` so our Rust types can stay idiomatic.

use serde::Deserialize;

// ─── Local API ────────────────────────────────────────────────────────────────

/// Response from GET /entitlements/v1/token
#[derive(Debug, Clone, Deserialize)]
pub struct EntitlementsTokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    /// The entitlement JWT – passed as `X-Riot-Entitlements-JWT` on remote calls.
    #[serde(rename = "token")]
    pub entitlement_token: String,
    /// The player's PUUID.
    #[serde(rename = "subject")]
    pub subject: String,
}

/// Response from GET /chat/v4/presences
#[derive(Debug, Clone, Deserialize)]
pub struct PresencesResponse {
    pub presences: Vec<Presence>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Presence {
    /// Player UUID – use this to find the local player's own entry.
    pub puuid: String,
    /// Product identifier; use `"valorant"` to filter out other Riot products.
    pub product: String,
    /// Base-64-encoded JSON with in-game state (see [`PresencePrivate`]).
    pub private: Option<String>,
}

/// Decoded content of [`Presence::private`].
///
/// The Valorant client encodes this object as base64 JSON and stores it in
/// the presence payload so other clients can display rich status info.
/// All fields are optional because the schema can vary between game states.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PresencePrivate {
    pub is_valid: Option<bool>,
    /// `"MENUS"` | `"PREGAME"` | `"INGAME"`
    pub session_loop_state: Option<String>,
    /// Current team's score (can be stringified integer on some client versions).
    pub party_owner_match_score_ally_team: Option<serde_json::Value>,
    /// Opponent's score (can be stringified integer).
    pub party_owner_match_score_enemy_team: Option<serde_json::Value>,
    /// `"Blue"` or `"Red"` – the current player's side.
    pub party_owner_match_current_team: Option<String>,
    /// Map path, e.g. `"/Game/Maps/Ascent/Ascent"`.
    pub match_map: Option<String>,
    /// Queue identifier, e.g. `"competitive"`, `"unrated"`, `"spikerush"`.
    pub queue_id: Option<String>,
    /// Game pod ID (server region/datacenter info).
    pub game_pod_id: Option<String>,
}

impl PresencePrivate {
    fn parse_score(v: &Option<serde_json::Value>) -> u32 {
        match v {
            Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0) as u32,
            Some(serde_json::Value::String(s)) => s.parse().unwrap_or(0),
            _ => 0,
        }
    }

    pub fn ally_score(&self) -> u32 {
        Self::parse_score(&self.party_owner_match_score_ally_team)
    }

    pub fn enemy_score(&self) -> u32 {
        Self::parse_score(&self.party_owner_match_score_enemy_team)
    }

    /// Total rounds played so far (ally + enemy won rounds).
    pub fn total_rounds(&self) -> u32 {
        self.ally_score() + self.enemy_score()
    }

    pub fn is_in_game(&self) -> bool {
        self.session_loop_state.as_deref() == Some("INGAME")
    }

    pub fn is_in_pregame(&self) -> bool {
        self.session_loop_state.as_deref() == Some("PREGAME")
    }
}

// ─── GLZ (Current-Game) API ───────────────────────────────────────────────────

/// Response from GET /core-game/v1/players/{puuid}
/// Used to retrieve the current match ID while a game is in progress.
#[derive(Debug, Clone, Deserialize)]
pub struct CoreGamePlayerResponse {
    #[serde(rename = "MatchID")]
    pub match_id: String,
    /// `"Blue"` or `"Red"`.
    #[serde(rename = "TeamID")]
    pub team_id: String,
    #[serde(rename = "Subject")]
    pub subject: String,
}

/// Response from GET /core-game/v1/matches/{matchId}
#[derive(Debug, Clone, Deserialize)]
pub struct CoreGameMatchResponse {
    #[serde(rename = "MatchID")]
    pub match_id: String,
    /// `"IN_PROGRESS"` while the game is live.
    #[serde(rename = "State")]
    pub state: String,
    /// Full map asset path, e.g. `"/Game/Maps/Ascent/Ascent"`.
    #[serde(rename = "MapID")]
    pub map_id: String,
    /// Full mode asset path.
    #[serde(rename = "ModeID")]
    pub mode_id: String,
    #[serde(rename = "ProvisioningFlow")]
    pub provisioning_flow: String,
    #[serde(rename = "Players")]
    pub players: Vec<CoreGameMatchPlayer>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreGameMatchPlayer {
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "TeamID")]
    pub team_id: String,
    /// Agent UUID – convert with [`super::maps::agent_id_to_name`].
    #[serde(rename = "CharacterID")]
    pub character_id: Option<String>,
    #[serde(rename = "PlayerIdentity")]
    pub player_identity: Option<CoreGamePlayerIdentity>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CoreGamePlayerIdentity {
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "GameName")]
    pub game_name: Option<String>,
    #[serde(rename = "TagLine")]
    pub tag_line: Option<String>,
}

// ─── PD (Player Data) API ─────────────────────────────────────────────────────

/// Response from GET /match-history/v1/history/{puuid}
#[derive(Debug, Clone, Deserialize)]
pub struct MatchHistoryResponse {
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "BeginIndex")]
    pub begin_index: u32,
    #[serde(rename = "EndIndex")]
    pub end_index: u32,
    #[serde(rename = "Total")]
    pub total: Option<u32>,
    #[serde(rename = "History")]
    pub history: Vec<MatchHistoryItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchHistoryItem {
    #[serde(rename = "MatchID")]
    pub match_id: String,
    #[serde(rename = "GameStartTime")]
    pub game_start_time: Option<u64>,
    #[serde(rename = "QueueID")]
    pub queue_id: Option<String>,
}

/// Response from GET /match-details/v1/matches/{matchId}
///
/// Only the fields we actually use are modelled; the real response is much
/// larger.
#[derive(Debug, Clone, Deserialize)]
pub struct MatchDetailsResponse {
    #[serde(rename = "matchInfo")]
    pub match_info: MatchInfo,
    #[serde(rename = "players")]
    pub players: Vec<MatchPlayer>,
    #[serde(rename = "teams")]
    pub teams: Vec<MatchTeam>,
    /// Per-round breakdown; present only on completed matches.
    #[serde(rename = "roundResults")]
    pub round_results: Option<Vec<RoundResult>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchInfo {
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "mapId")]
    pub map_id: String,
    #[serde(rename = "queueID")]
    pub queue_id: String,
    #[serde(rename = "gameMode")]
    pub game_mode: String,
    #[serde(rename = "isCompleted")]
    pub is_completed: bool,
    #[serde(rename = "completionState")]
    pub completion_state: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchPlayer {
    #[serde(rename = "subject")]
    pub subject: String,
    #[serde(rename = "gameName")]
    pub game_name: String,
    #[serde(rename = "tagLine")]
    pub tag_line: String,
    #[serde(rename = "teamId")]
    pub team_id: String,
    /// Agent UUID.
    #[serde(rename = "characterId")]
    pub character_id: String,
    #[serde(rename = "stats")]
    pub stats: Option<MatchPlayerStats>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchPlayerStats {
    pub score: u32,
    #[serde(rename = "roundsPlayed")]
    pub rounds_played: u32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchTeam {
    #[serde(rename = "teamId")]
    pub team_id: String,
    pub won: bool,
    #[serde(rename = "roundsPlayed")]
    pub rounds_played: u32,
    #[serde(rename = "roundsWon")]
    pub rounds_won: u32,
}

/// Per-round data within a completed match.
#[derive(Debug, Clone, Deserialize)]
pub struct RoundResult {
    #[serde(rename = "roundNum")]
    pub round_num: u32,
    #[serde(rename = "winningTeam")]
    pub winning_team: String,
    #[serde(rename = "roundResultCode")]
    pub round_result_code: String,
    #[serde(rename = "playerStats")]
    pub player_stats: Vec<RoundPlayerStats>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoundPlayerStats {
    pub subject: String,
    pub kills: Vec<KillEvent>,
    pub score: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KillEvent {
    /// Milliseconds since game start.
    #[serde(rename = "timeSinceGameStartMillis")]
    pub game_time: Option<u64>,
    /// Milliseconds since round start.
    #[serde(rename = "timeSinceRoundStartMillis")]
    pub round_time: Option<u64>,
    pub killer: String,
    pub victim: String,
    pub assistants: Vec<String>,
}