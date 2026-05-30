//! High-level wrappers over the raw LCU REST endpoints.

use super::connection::Connection;
use super::models::*;
use reqwest::Method;

pub type Result<T> = std::result::Result<T, reqwest::Error>;

/// Current gameflow phase, e.g. "Lobby", "Matchmaking", "ReadyCheck",
/// "ChampSelect", "InProgress", "None". Returned by the LCU as a bare JSON
/// string.
pub async fn gameflow_phase(conn: &Connection) -> Result<String> {
    let phase: String = conn
        .request(Method::GET, "/lol-gameflow/v1/gameflow-phase")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(phase)
}

pub async fn champ_select_session(conn: &Connection) -> Result<ChampSelectSession> {
    conn.request(Method::GET, "/lol-champ-select/v1/session")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

pub async fn ready_check(conn: &Connection) -> Result<ReadyCheck> {
    conn.request(Method::GET, "/lol-matchmaking/v1/ready-check")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
}

pub async fn accept_ready_check(conn: &Connection) -> Result<()> {
    conn.request(Method::POST, "/lol-matchmaking/v1/ready-check/accept")
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

/// Resolve a puuid to a summoner (Riot ID + level). Works even when the name is
/// visually hidden in champ select — this is the core of the "reveal".
pub async fn summoner_by_puuid(conn: &Connection, puuid: &str) -> Result<Summoner> {
    conn.request(
        Method::GET,
        &format!("/lol-summoner/v2/summoners/puuid/{puuid}"),
    )
    .send()
    .await?
    .error_for_status()?
    .json()
    .await
}

/// puuid of the logged-in player, used to flag "you" in the revealed list.
pub async fn current_summoner_puuid(conn: &Connection) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct Cur {
        #[serde(default)]
        puuid: String,
    }
    let cur: Cur = conn
        .request(Method::GET, "/lol-summoner/v1/current-summoner")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(cur.puuid)
}

/// All chat participants. Callers filter to the champ-select room (`cid`
/// contains "champ-select") to reveal allies — the session no longer carries
/// their identities.
pub async fn chat_participants(conn: &Connection) -> Result<Vec<ChatParticipant>> {
    let lobby: ChatParticipants = conn
        .request(Method::GET, "/chat/v5/participants")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(lobby.participants)
}

pub async fn ranked_by_puuid(conn: &Connection, puuid: &str) -> Result<RankedStats> {
    conn.request(
        Method::GET,
        &format!("/lol-ranked/v1/ranked-stats/{puuid}"),
    )
    .send()
    .await?
    .error_for_status()?
    .json()
    .await
}

/// Win rate over the player's most recent `count` games (wins, total).
/// Works by puuid; returns (0, 0) if the player's history isn't accessible.
pub async fn recent_winrate(conn: &Connection, puuid: &str, count: i64) -> Result<(i64, i64)> {
    let path = format!(
        "/lol-match-history/v1/products/lol/{puuid}/matches?begIndex=0&endIndex={}",
        count.max(1) - 1
    );
    let hist: MatchHistory = conn
        .request(Method::GET, &path)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut wins = 0;
    let mut total = 0;
    for game in hist.games.games.iter().take(count as usize) {
        let id = game
            .participant_identities
            .iter()
            .find(|i| i.player.puuid == puuid)
            .map(|i| i.participant_id);
        if let Some(id) = id {
            if let Some(p) = game.participants.iter().find(|p| p.participant_id == id) {
                total += 1;
                if p.stats.win {
                    wins += 1;
                }
            }
        }
    }
    Ok((wins, total))
}

/// Hover a champion on an action without locking it in (sets `championId`
/// only). Some clients reject an immediate completed ban unless the champion is
/// hovered first, so we always hover before completing.
pub async fn hover_action(conn: &Connection, action_id: i64, champion_id: i64) -> Result<()> {
    let body = serde_json::json!({ "championId": champion_id });
    conn.request(
        Method::PATCH,
        &format!("/lol-champ-select/v1/session/actions/{action_id}"),
    )
    .json(&body)
    .send()
    .await?
    .error_for_status()?;
    Ok(())
}

/// Hover/ban/pick a champion by PATCHing a champ-select action.
/// When `complete` is true the action is locked in (final ban / final pick).
pub async fn patch_action(
    conn: &Connection,
    action_id: i64,
    champion_id: i64,
    complete: bool,
) -> Result<()> {
    let body = serde_json::json!({ "championId": champion_id, "completed": complete });
    conn.request(
        Method::PATCH,
        &format!("/lol-champ-select/v1/session/actions/{action_id}"),
    )
    .json(&body)
    .send()
    .await?
    .error_for_status()?;
    Ok(())
}

/// Leave champ select (dodge). Uses the legacy LCDS `quitV2` invoke, which is
/// the established way to bail out of a draft lobby.
pub async fn dodge(conn: &Connection) -> Result<()> {
    conn.request(Method::POST, "/lol-login/v1/session/invoke")
        .query(&[
            ("destination", "lcdsServiceProxy"),
            ("method", "call"),
            ("args", "[\"\",\"teambuilder-draft\",\"quitV2\",\"\"]"),
        ])
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
