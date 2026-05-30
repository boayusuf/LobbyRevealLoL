//! Fetches the full champion id<->name list from Riot's public Data Dragon CDN.
//! Used to populate the auto-pick / auto-ban selectors in the UI.

use super::models::Champion;
use std::collections::HashMap;

#[derive(serde::Deserialize)]
struct DdragonChampion {
    key: String,
    name: String,
}

#[derive(serde::Deserialize)]
struct DdragonChampionData {
    data: HashMap<String, DdragonChampion>,
}

pub async fn fetch() -> Result<Vec<Champion>, reqwest::Error> {
    let client = reqwest::Client::new();
    let versions: Vec<String> = client
        .get("https://ddragon.leagueoflegends.com/api/versions.json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let version = versions.first().cloned().unwrap_or_else(|| "latest".into());

    let url = format!(
        "https://ddragon.leagueoflegends.com/cdn/{version}/data/en_US/champion.json"
    );
    let data: DdragonChampionData = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let mut champions: Vec<Champion> = data
        .data
        .into_values()
        .filter_map(|c| {
            c.key.parse::<i64>().ok().map(|id| Champion { id, name: c.name })
        })
        .collect();
    champions.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(champions)
}
