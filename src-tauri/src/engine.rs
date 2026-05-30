//! Background polling loop. Every tick it:
//!   1. ensures a live connection to the League Client,
//!   2. auto-accepts ready checks (Phase 2),
//!   3. in champ select: reveals allies, runs auto-ban / auto-pick (Phase 2),
//!      and performs last-second / manual dodges (Phase 2),
//!   4. emits a `lcu-update` event carrying the full UI state (Phase 1).
//!
//! Polling (rather than the LCU WebSocket) keeps the implementation simple and
//! resilient to the client restarting; 1s granularity is plenty for a lobby UI.

use crate::lcu::{api, connection, models::*, Connection};
use crate::state::AppState;
use std::collections::HashMap;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

const TICK: Duration = Duration::from_secs(1);
const RECONNECT_DELAY: Duration = Duration::from_secs(3);
/// How many recent games to base the win rate on.
const RECENT_GAMES: i64 = 10;
/// Cap auto-pick/ban attempts per action so a non-lockable champion can't spam.
const MAX_ATTEMPTS: u32 = 10;

/// Per-player stats cached for the duration of a lobby.
#[derive(Clone, Default)]
struct PlayerStats {
    ranked: Option<QueueStats>,
    recent_wins: i64,
    recent_total: i64,
}
type PlayerCache = HashMap<String, PlayerStats>;

pub async fn run(app: AppHandle) {
    let mut conn: Option<Connection> = None;
    // Separate connection to the Riot Client (serves the chat endpoint used for
    // the reveal). Distinct local server from the League Client.
    let mut riot_conn: Option<Connection> = None;
    let mut cache: PlayerCache = HashMap::new();
    let mut attempts: HashMap<i64, u32> = HashMap::new();
    let mut self_puuid = String::new();
    let mut was_in_champ_select = false;

    loop {
        // 1. Ensure a connection.
        if conn.is_none() {
            if let Some(d) = connection::discover() {
                if let Ok(c) = Connection::new(d.lcu.0, &d.lcu.1) {
                    *app.state::<AppState>().conn_info.lock().unwrap() =
                        Some(d.lcu.clone());
                    conn = Some(c);
                    riot_conn = d
                        .riot
                        .as_ref()
                        .and_then(|(p, t)| Connection::new(*p, t).ok());
                }
            }
        }
        let Some(active) = conn.as_ref() else {
            emit_disconnected(&app, "League client not found — start League and log in.");
            sleep(RECONNECT_DELAY).await;
            continue;
        };

        let settings = app.state::<AppState>().settings.lock().unwrap().clone();

        // 2. Determine where the client is in its flow.
        let phase = match api::gameflow_phase(active).await {
            Ok(p) => p,
            Err(_) => {
                // The client went away (closed, or token rotated). Reset.
                conn = None;
                riot_conn = None;
                *app.state::<AppState>().conn_info.lock().unwrap() = None;
                cache.clear();
                attempts.clear();
                emit_disconnected(&app, "Lost connection to the League client.");
                sleep(RECONNECT_DELAY).await;
                continue;
            }
        };

        if phase == "ReadyCheck" && settings.auto_accept {
            if let Ok(rc) = api::ready_check(active).await {
                if rc.state == "InProgress" && rc.player_response != "Accepted" {
                    let _ = api::accept_ready_check(active).await;
                }
            }
        }

        if phase == "ChampSelect" {
            if let Ok(session) = api::champ_select_session(active).await {
                handle_actions(active, &session, &settings, &mut attempts).await;

                let timer = &session.timer;
                let last_second = settings.auto_dodge
                    && timer.phase == "FINALIZATION"
                    && timer.adjusted_time_left_in_phase > 0
                    && timer.adjusted_time_left_in_phase <= settings.dodge_threshold_ms;
                if last_second {
                    let _ = api::dodge(active).await;
                }

                // Reveal allies from the champ-select chat room (the session no
                // longer carries their identities). Learn our own puuid once so
                // we can flag "you".
                if self_puuid.is_empty() {
                    if let Ok(p) = api::current_summoner_puuid(active).await {
                        self_puuid = p;
                    }
                }
                // The chat roster lives on the Riot Client, not the League
                // Client — use that connection.
                let (players, debug) = match riot_conn.as_ref() {
                    None => (
                        Vec::new(),
                        "Riot Client port not found in client args.".to_string(),
                    ),
                    Some(rc) => match api::chat_participants(rc).await {
                        Ok(parts) => {
                            let champ =
                                parts.iter().filter(|p| p.cid.contains("champ")).count();
                            let sample: String = parts
                                .iter()
                                .map(|p| p.cid.clone())
                                .find(|c| !c.is_empty())
                                .unwrap_or_default()
                                .chars()
                                .take(40)
                                .collect();
                            let dbg = format!(
                                "riot chat: {} participants, {} champ-room · cid≈{}",
                                parts.len(),
                                champ,
                                sample
                            );
                            (
                                build_players(active, &parts, &self_puuid, &settings, &mut cache)
                                    .await,
                                dbg,
                            )
                        }
                        Err(e) => (Vec::new(), format!("riot chat error: {e}")),
                    },
                };
                let champ_phase = timer.phase.clone();
                let time_left_ms = timer.adjusted_time_left_in_phase;
                let ui = UiState {
                    connected: true,
                    phase,
                    in_champ_select: true,
                    champ_phase,
                    time_left_ms,
                    players,
                    message: String::new(),
                    debug,
                };
                let _ = app.emit("lcu-update", &ui);
                was_in_champ_select = true;
            }
        } else {
            // Left champ select: invalidate per-lobby caches once.
            if was_in_champ_select {
                cache.clear();
                attempts.clear();
                was_in_champ_select = false;
            }
            let ui = UiState {
                connected: true,
                message: friendly_phase(&phase),
                phase,
                ..Default::default()
            };
            let _ = app.emit("lcu-update", &ui);
        }

        sleep(TICK).await;
    }
}

/// Auto-ban / auto-pick: when it's the local player's turn and the matching
/// automation is enabled, hover the configured champion then lock it in.
///
/// We retry every tick until the client reports the action `completed` (the
/// guard below stops us once it is), rather than giving up after one request —
/// that's what caused "only works after a reopen": a first request that returned
/// OK but didn't actually lock was never retried. `attempts` caps the retries so
/// a champion that genuinely can't be locked (already banned, not owned) can't
/// spam the client forever.
async fn handle_actions(
    conn: &Connection,
    session: &ChampSelectSession,
    settings: &Settings,
    attempts: &mut HashMap<i64, u32>,
) {
    let local = session.local_player_cell_id;
    for action in session.actions.iter().flatten() {
        if action.actor_cell_id != local || action.completed || !action.is_in_progress {
            continue;
        }
        let champion = match action.action_type.as_str() {
            "ban" if settings.auto_ban && settings.auto_ban_champion_id > 0 => {
                settings.auto_ban_champion_id
            }
            "pick" if settings.auto_pick && settings.auto_pick_champion_id > 0 => {
                settings.auto_pick_champion_id
            }
            _ => continue,
        };
        let tries = attempts.entry(action.id).or_insert(0);
        if *tries >= MAX_ATTEMPTS {
            continue;
        }
        *tries += 1;
        // Hover first (bans often require it), then lock in.
        let _ = api::hover_action(conn, action.id, champion).await;
        let _ = api::patch_action(conn, action.id, champion, true).await;
    }
}

/// Build the revealed-player rows from the champ-select chat participants,
/// caching ranked lookups per puuid so we only hit the endpoint once per lobby.
async fn build_players(
    conn: &Connection,
    participants: &[ChatParticipant],
    self_puuid: &str,
    settings: &Settings,
    cache: &mut PlayerCache,
) -> Vec<UiPlayer> {
    let mut players = Vec::new();
    let mut cell = 0;
    for p in participants {
        // Keep only the champ-select room. Show the name even if we can't look
        // up rank (some entries may not expose a puuid).
        if !p.cid.contains("champ") {
            continue;
        }

        // Resolve ranked + recent-games win rate once per puuid, then cache.
        let stats = if p.puuid.is_empty() {
            PlayerStats::default()
        } else {
            if !cache.contains_key(&p.puuid) {
                let ranked = api::ranked_by_puuid(conn, &p.puuid)
                    .await
                    .ok()
                    .and_then(|r| r.queue_map.get("RANKED_SOLO_5x5").cloned());
                let (recent_wins, recent_total) = api::recent_winrate(conn, &p.puuid, RECENT_GAMES)
                    .await
                    .unwrap_or((0, 0));
                cache.insert(
                    p.puuid.clone(),
                    PlayerStats {
                        ranked,
                        recent_wins,
                        recent_total,
                    },
                );
            }
            cache.get(&p.puuid).cloned().unwrap_or_default()
        };

        // Riot ID: prefer the structured gameName#tagLine, fall back to `name`.
        let (game_name, tag_line, riot_id) = if !p.game_name.is_empty() {
            (
                p.game_name.clone(),
                p.game_tag.clone(),
                format!("{}#{}", p.game_name, p.game_tag),
            )
        } else {
            let (n, t) = p.name.split_once('#').unwrap_or((p.name.as_str(), ""));
            (n.to_string(), t.to_string(), p.name.clone())
        };

        let (rank, lp) = match &stats.ranked {
            Some(q) if !q.tier.is_empty() && q.tier != "NONE" => {
                (format_rank(q), q.league_points)
            }
            _ => ("Unranked".into(), 0),
        };
        let recent_winrate = if stats.recent_total > 0 {
            (stats.recent_wins as f64 / stats.recent_total as f64 * 100.0).round() as i64
        } else {
            0
        };

        players.push(UiPlayer {
            cell_id: cell,
            position: String::new(),
            champion_id: 0,
            riot_id,
            level: 0,
            rank,
            lp,
            recent_winrate,
            recent_games: stats.recent_total,
            is_local: !self_puuid.is_empty() && p.puuid == self_puuid,
            opgg_url: opgg_url(&settings.region, &game_name, &tag_line),
        });
        cell += 1;
    }
    players
}

fn format_rank(q: &QueueStats) -> String {
    let tier = title_case(&q.tier);
    if q.division.is_empty() || q.division == "NA" {
        tier
    } else {
        format!("{tier} {}", q.division)
    }
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

fn opgg_url(region: &str, game_name: &str, tag_line: &str) -> String {
    if game_name.is_empty() {
        return String::new();
    }
    let name = urlencoding::encode(game_name);
    let tag = urlencoding::encode(tag_line);
    format!("https://www.op.gg/summoners/{region}/{name}-{tag}")
}

fn friendly_phase(phase: &str) -> String {
    match phase {
        "None" | "" => "Connected. Waiting in client…".into(),
        "Lobby" => "In lobby.".into(),
        "Matchmaking" => "Searching for a match…".into(),
        "ReadyCheck" => "Match found!".into(),
        "InProgress" => "Game in progress.".into(),
        other => format!("Connected ({other})."),
    }
}

fn emit_disconnected(app: &AppHandle, message: &str) {
    let ui = UiState {
        connected: false,
        message: message.to_string(),
        ..Default::default()
    };
    let _ = app.emit("lcu-update", &ui);
}
