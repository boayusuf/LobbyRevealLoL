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
