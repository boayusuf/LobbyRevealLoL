//! Deserialization models for the LCU endpoints we consume, plus the
//! serializable view-models we push to the frontend.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ---------------------------------------------------------------------------
// Champion select session (`/lol-champ-select/v1/session`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChampSelectSession {
    #[serde(default)]
    pub actions: Vec<Vec<Action>>,
    #[serde(default)]
    pub my_team: Vec<TeamMember>,
    #[serde(default)]
    pub their_team: Vec<TeamMember>,
    #[serde(default)]
    pub local_player_cell_id: i64,
    #[serde(default)]
    pub timer: Timer,
    #[serde(default)]
    pub is_custom_game: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Timer {
    #[serde(default)]
    pub adjusted_time_left_in_phase: i64,
    #[serde(default)]
    pub total_time_in_phase: i64,
    #[serde(default)]
    pub phase: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMember {
    #[serde(default)]
    pub cell_id: i64,
    #[serde(default)]
    pub champion_id: i64,
    #[serde(default)]
    pub champion_pick_intent: i64,
    #[serde(default)]
    pub assigned_position: String,
    #[serde(default)]
    pub puuid: String,
    #[serde(default)]
    pub summoner_id: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    #[serde(default)]
    pub id: i64,
    #[serde(default)]
    pub actor_cell_id: i64,
    #[serde(default)]
    pub champion_id: i64,
    #[serde(default, rename = "type")]
    pub action_type: String,
    #[serde(default)]
    pub completed: bool,
    #[serde(default)]
    pub is_in_progress: bool,
    #[serde(default)]
    pub is_ally_action: bool,
}

// ---------------------------------------------------------------------------
// Summoner + ranked lookups
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Summoner {
    #[serde(default)]
    pub game_name: String,
    #[serde(default)]
    pub tag_line: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default)]
    pub summoner_level: i64,
    #[serde(default)]
    pub puuid: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RankedStats {
    #[serde(default)]
    pub queue_map: HashMap<String, QueueStats>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueStats {
    #[serde(default)]
    pub tier: String,
    #[serde(default)]
    pub division: String,
    #[serde(default)]
    pub league_points: i64,
    #[serde(default)]
    pub wins: i64,
    #[serde(default)]
    pub losses: i64,
}

// ---------------------------------------------------------------------------
// Match history (`/lol-match-history/v1/products/lol/{puuid}/matches`)
// Used to compute a recent-games win rate, since ranked-stats hides losses for
// other players.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct MatchHistory {
    #[serde(default)]
    pub games: GamesWrap,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GamesWrap {
    #[serde(default)]
    pub games: Vec<HistGame>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistGame {
    #[serde(default)]
    pub participants: Vec<HistParticipant>,
    #[serde(default)]
    pub participant_identities: Vec<HistIdentity>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistParticipant {
    #[serde(default)]
    pub participant_id: i64,
    #[serde(default)]
    pub stats: HistStats,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistStats {
    #[serde(default)]
    pub win: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistIdentity {
    #[serde(default)]
    pub participant_id: i64,
    #[serde(default)]
    pub player: HistPlayer,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistPlayer {
    #[serde(default)]
    pub puuid: String,
}

// ---------------------------------------------------------------------------
// Ready check (`/lol-matchmaking/v1/ready-check`)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyCheck {
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub player_response: String,
}

// ---------------------------------------------------------------------------
// Chat participants (`/chat/v5/participants`)
//
// The champ-select session no longer exposes teammates' puuid/summonerId, so we
// recover identities from the champ-select chat room instead — the same trick
// steele123/reveal uses. Each participant carries their Riot ID + puuid; we keep
// only those whose conversation id (`cid`) is the champ-select room.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatParticipants {
    #[serde(default)]
    pub participants: Vec<ChatParticipant>,
}

// NOTE: the Riot chat service returns snake_case fields (game_name, game_tag),
// unlike the camelCase LCU. Do NOT add rename_all = "camelCase" here, or the
// names come back empty (which hides the Riot ID and the OP.GG link).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChatParticipant {
    #[serde(default)]
    pub puuid: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub game_name: String,
    #[serde(default)]
    pub game_tag: String,
    #[serde(default)]
    pub cid: String,
}

// ---------------------------------------------------------------------------
// Settings (frontend -> backend) and view-models (backend -> frontend)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub auto_accept: bool,
    pub auto_dodge: bool,
    /// During FINALIZATION, dodge once the timer drops below this (ms).
    pub dodge_threshold_ms: i64,
    pub auto_pick: bool,
    pub auto_pick_champion_id: i64,
    pub auto_ban: bool,
    pub auto_ban_champion_id: i64,
    /// op.gg region slug, e.g. "euw", "na", "kr".
    pub region: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_accept: false,
            auto_dodge: false,
            dodge_threshold_ms: 1500,
            auto_pick: false,
            auto_pick_champion_id: 0,
            auto_ban: false,
            auto_ban_champion_id: 0,
            region: "euw".into(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPlayer {
    pub cell_id: i64,
    pub position: String,
    pub champion_id: i64,
    pub riot_id: String,
    pub level: i64,
    pub rank: String,
    pub lp: i64,
    /// Win rate over recent games (from match history), 0 if unavailable.
    pub recent_winrate: i64,
    /// How many recent games the win rate is based on (0 = none available).
    pub recent_games: i64,
    pub is_local: bool,
    pub opgg_url: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UiState {
    pub connected: bool,
    pub phase: String,
    pub in_champ_select: bool,
    pub champ_phase: String,
    pub time_left_ms: i64,
    pub players: Vec<UiPlayer>,
    pub message: String,
    /// Diagnostic readout shown in the UI while we debug the reveal source.
    pub debug: String,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Champion {
    pub id: i64,
    pub name: String,
}
