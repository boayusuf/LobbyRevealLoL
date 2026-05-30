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
use std::collections::{HashMap, HashSet};
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};
use tokio::time::sleep;

const TICK: Duration = Duration::from_secs(1);
const RECONNECT_DELAY: Duration = Duration::from_secs(3);

type PlayerCache = HashMap<String, (Summoner, Option<QueueStats>)>;

pub async fn run(app: AppHandle) {
    let mut conn: Option<Connection> = None;
    let mut cache: PlayerCache = HashMap::new();
    let mut attempted: HashSet<i64> = HashSet::new();
    let mut was_in_champ_select = false;

    loop {
        // 1. Ensure a connection.
        if conn.is_none() {
            if let Some((port, token)) = connection::discover() {
                conn = Connection::new(port, &token).ok();
            }
        }
        let Some(active) = conn.as_ref() else {
            emit_disconnected(&app, "League client not found — start League and log in.");
            sleep(RECONNECT_DELAY).await;
            continue;
        };

        // Read a snapshot of settings + the one-shot manual dodge flag.
        let (settings, manual_dodge) = {
            let state = app.state::<AppState>();
            let settings = state.settings.lock().unwrap().clone();
            let manual = state.dodge_requested.swap(false, Ordering::SeqCst);
            (settings, manual)
        };

        // 2. Determine where the client is in its flow.
        let phase = match api::gameflow_phase(active).await {
            Ok(p) => p,
            Err(_) => {
                // The client went away (closed, or token rotated). Reset.
                conn = None;
                cache.clear();
                attempted.clear();
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
                handle_actions(active, &session, &settings, &mut attempted).await;

                let timer = &session.timer;
                let last_second = settings.auto_dodge
                    && timer.phase == "FINALIZATION"
                    && timer.adjusted_time_left_in_phase > 0
                    && timer.adjusted_time_left_in_phase <= settings.dodge_threshold_ms;
                if manual_dodge || last_second {
                    let _ = api::dodge(active).await;
                }

                let players = build_players(active, &session, &settings, &mut cache).await;
                let ui = UiState {
                    connected: true,
                    phase,
                    in_champ_select: true,
                    champ_phase: timer.phase.clone(),
                    time_left_ms: timer.adjusted_time_left_in_phase,
                    players,
                    message: String::new(),
                };
                let _ = app.emit("lcu-update", &ui);
                was_in_champ_select = true;
            }
        } else {
            // Left champ select: invalidate per-lobby caches once.
            if was_in_champ_select {
                cache.clear();
                attempted.clear();
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
/// automation is enabled, lock in the configured champion. Each action is only
/// attempted once so a rejected champion (already banned, not owned, …) doesn't
/// get hammered every tick.
async fn handle_actions(
    conn: &Connection,
    session: &ChampSelectSession,
    settings: &Settings,
    attempted: &mut HashSet<i64>,
) {
    let local = session.local_player_cell_id;
    for action in session.actions.iter().flatten() {
        if action.actor_cell_id != local
            || action.completed
            || !action.is_in_progress
            || attempted.contains(&action.id)
        {
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
        // Mark attempted regardless of outcome to avoid spamming the client.
        attempted.insert(action.id);
        let _ = api::patch_action(conn, action.id, champion, true).await;
    }
}

/// Resolve every ally cell into a display row, caching lookups per puuid so we
/// only hit the summoner/ranked endpoints once per lobby.
async fn build_players(
    conn: &Connection,
    session: &ChampSelectSession,
    settings: &Settings,
    cache: &mut PlayerCache,
) -> Vec<UiPlayer> {
    // Reveal everyone the client exposes — your subteam *and* anyone listed on
    // the other side (e.g. Arena lists multiple sub-teams). Dedupe by cell id.
    let mut players = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let others = session
        .their_team
        .iter()
        .filter(|m| !m.puuid.is_empty());
    for member in session.my_team.iter().chain(others) {
        if !seen.insert(member.cell_id) {
            continue;
        }
        let is_local = member.cell_id == session.local_player_cell_id;
        let champion_id = if member.champion_id > 0 {
            member.champion_id
        } else {
            member.champion_pick_intent
        };

        // Allies with no exposed puuid can't be revealed (rare); show the slot.
        if member.puuid.is_empty() {
            players.push(UiPlayer {
                cell_id: member.cell_id,
                position: member.assigned_position.clone(),
                champion_id,
                riot_id: "Hidden".into(),
                is_local,
                ..Default::default()
            });
            continue;
        }

        if !cache.contains_key(&member.puuid) {
            if let Ok(summoner) = api::summoner_by_puuid(conn, &member.puuid).await {
                let ranked = api::ranked_by_puuid(conn, &member.puuid)
                    .await
                    .ok()
                    .and_then(|r| r.queue_map.get("RANKED_SOLO_5x5").cloned());
                cache.insert(member.puuid.clone(), (summoner, ranked));
            }
        }

        let Some((summoner, ranked)) = cache.get(&member.puuid) else {
            continue;
        };

        let riot_id = if !summoner.game_name.is_empty() {
            format!("{}#{}", summoner.game_name, summoner.tag_line)
        } else {
            summoner.display_name.clone()
        };
        let (rank, lp, wins, losses, winrate) = match ranked {
            Some(q) if !q.tier.is_empty() && q.tier != "NONE" => {
                let total = q.wins + q.losses;
                let wr = if total > 0 {
                    (q.wins as f64 / total as f64 * 100.0).round() as i64
                } else {
                    0
                };
                (format_rank(q), q.league_points, q.wins, q.losses, wr)
            }
            _ => ("Unranked".into(), 0, 0, 0, 0),
        };

        players.push(UiPlayer {
            cell_id: member.cell_id,
            position: member.assigned_position.clone(),
            champion_id,
            riot_id: riot_id.clone(),
            level: summoner.summoner_level,
            rank,
            lp,
            wins,
            losses,
            winrate,
            is_local,
            opgg_url: opgg_url(&settings.region, &summoner.game_name, &summoner.tag_line),
        });
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
