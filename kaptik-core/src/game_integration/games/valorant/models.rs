// models.rs – Valorant API response types.

use serde::Deserialize;

// ─── Local API ────────────────────────────────────────────────────────────────

/// Response from GET /entitlements/v1/token
#[derive(Debug, Clone, Deserialize)]
pub struct EntitlementsTokenResponse {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "token")]
    pub entitlement_token: String,
    #[serde(rename = "subject")]
    pub subject: String,
}

// ─── GLZ (Core-Game) API ─────────────────────────────────────────────────────

/// Response from GET /core-game/v1/players/{puuid}
#[derive(Debug, Clone, Deserialize)]
pub struct CurrentGamePlayerResponse {
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "MatchID")]
    pub match_id: String,
    #[serde(rename = "Version")]
    pub version: u64,
}

/// Response from GET /core-game/v1/matches/{matchId}
#[derive(Debug, Clone, Deserialize)]
pub struct CurrentGameMatchResponse {
    #[serde(rename = "MatchID")]
    pub match_id: String,
    #[serde(rename = "State")]
    pub state: String,
    #[serde(rename = "MapID")]
    pub map_id: String,
    #[serde(rename = "ModeID")]
    pub mode_id: String,
    #[serde(rename = "ProvisioningFlow")]
    pub provisioning_flow: String,
    #[serde(rename = "GamePodID")]
    pub game_pod_id: String,
    #[serde(rename = "AllMUCName")]
    pub all_muc_name: String,
    #[serde(rename = "TeamMUCName")]
    pub team_muc_name: String,
    #[serde(rename = "TeamVoiceID")]
    pub team_voice_id: String,
    #[serde(rename = "TeamMatchToken")]
    pub team_match_token: String,
    #[serde(rename = "IsReconnectable")]
    pub is_reconnectable: bool,
    #[serde(rename = "ConnectionDetails")]
    pub connection_details: ConnectionDetails,
    #[serde(rename = "PostGameDetails")]
    pub post_game_details: Option<serde_json::Value>,
    #[serde(rename = "Players")]
    pub players: Vec<Player>,
    #[serde(rename = "MatchmakingData")]
    pub matchmaking_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConnectionDetails {
    #[serde(rename = "GameServerHosts")]
    pub game_server_hosts: Vec<String>,
    #[serde(rename = "GameServerHost")]
    pub game_server_host: String,
    #[serde(rename = "GameServerPort")]
    pub game_server_port: u64,
    #[serde(rename = "GameServerObfuscatedIP")]
    pub game_server_obfuscated_ip: u64,
    #[serde(rename = "GameClientHash")]
    pub game_client_hash: u64,
    #[serde(rename = "PlayerKey")]
    pub player_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Player {
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "TeamID")]
    pub team_id: String,
    #[serde(rename = "CharacterID")]
    pub character_id: String,
    #[serde(rename = "PlayerIdentity")]
    pub player_identity: PlayerIdentity,
    #[serde(rename = "SeasonalBadgeInfo")]
    pub seasonal_badge_info: SeasonalBadgeInfo,
    #[serde(rename = "IsCoach")]
    pub is_coach: bool,
    #[serde(rename = "IsAssociated")]
    pub is_associated: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerIdentity {
    #[serde(rename = "Subject")]
    pub subject: String,
    #[serde(rename = "PlayerCardID")]
    pub player_card_id: String,
    #[serde(rename = "PlayerTitleID")]
    pub player_title_id: String,
    #[serde(rename = "AccountLevel")]
    pub account_level: u64,
    #[serde(rename = "PreferredLevelBorderID")]
    pub preferred_level_border_id: String,
    #[serde(rename = "Incognito")]
    pub incognito: bool,
    #[serde(rename = "HideAccountLevel")]
    pub hide_account_level: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SeasonalBadgeInfo {
    #[serde(rename = "SeasonID")]
    pub season_id: String,
    #[serde(rename = "NumberOfWins")]
    pub number_of_wins: u64,
    #[serde(rename = "WinsByTier")]
    pub wins_by_tier: Option<serde_json::Value>,
    #[serde(rename = "Rank")]
    pub rank: u64,
    #[serde(rename = "LeaderboardRank")]
    pub leaderboard_rank: u64,
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
#[derive(Debug, Clone, Deserialize)]
pub struct MatchDetailsResponse {
    #[serde(rename = "matchInfo")]
    pub match_info: MatchInfo,
    #[serde(rename = "players")]
    pub players: Vec<MatchPlayer>,
    #[serde(rename = "teams")]
    pub teams: Vec<MatchTeam>,
    #[serde(rename = "roundResults")]
    pub round_results: Option<Vec<RoundResult>>,
    #[serde(rename = "kills")]
    pub kills: Option<Vec<MatchKillEvent>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchInfo {
    #[serde(rename = "matchId")]
    pub match_id: String,
    #[serde(rename = "mapId")]
    pub map_id: String,
    #[serde(rename = "gameLengthMillis")]
    pub game_length_millis: Option<u64>,
    #[serde(rename = "gameStartMillis")]
    pub game_start_millis: Option<u64>,
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
    #[serde(rename = "numPoints")]
    pub num_points: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoundResult {
    #[serde(rename = "roundNum")]
    pub round_num: u32,
    #[serde(rename = "roundResult")]
    pub round_result: String,
    #[serde(rename = "roundCeremony")]
    pub round_ceremony: Option<String>,
    #[serde(rename = "winningTeam")]
    pub winning_team: String,
    #[serde(rename = "playerStats")]
    pub player_stats: Vec<RoundPlayerStats>,
    #[serde(rename = "playerScores")]
    pub player_scores: Option<Vec<RoundPlayerScore>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoundPlayerStats {
    pub subject: String,
    pub kills: Vec<KillEvent>,
    pub score: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoundPlayerScore {
    pub subject: String,
    pub score: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct KillEvent {
    /// Milliseconds since game start
    #[serde(rename = "gameTime")]
    pub game_time: Option<u64>,
    /// Milliseconds since round start
    #[serde(rename = "roundTime")]
    pub round_time: Option<u64>,
    pub killer: String,
    pub victim: String,
    pub assistants: Vec<String>,
    #[serde(rename = "finishingDamage")]
    pub finishing_damage: Option<FinishingDamage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MatchKillEvent {
    /// Milliseconds since game start
    #[serde(rename = "gameTime")]
    pub game_time: u64,
    /// Milliseconds since round start
    #[serde(rename = "roundTime")]
    pub round_time: u64,
    pub round: u32,
    pub killer: String,
    pub victim: String,
    pub assistants: Vec<String>,
    #[serde(rename = "finishingDamage")]
    pub finishing_damage: Option<FinishingDamage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FinishingDamage {
    #[serde(rename = "damageType")]
    pub damage_type: String,
    #[serde(rename = "damageItem")]
    pub damage_item: String,
    #[serde(rename = "isSecondaryFireMode")]
    pub is_secondary_fire_mode: bool,
}