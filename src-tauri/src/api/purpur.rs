use serde::Deserialize;

use crate::api::client;

#[derive(Deserialize)]
struct VersionsResp {
    #[serde(default)]
    versions: Vec<String>,
}

pub async fn versions() -> Result<Vec<String>, String> {
    let url = "https://api.purpurmc.org/v2/purpur";
    let resp = client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Purpur API error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Purpur API {}", resp.status()));
    }
    let data: VersionsResp = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data.versions)
}

/// Returns the latest build number for a minecraft version.
pub async fn latest_build(mc: &str) -> Result<i64, String> {
    let url = format!("https://api.purpurmc.org/v2/purpur/{mc}/latest");
    let resp = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Purpur API error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Purpur has no build for {mc} ({})", resp.status()));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    // The Purpur API returns "build" as a JSON *string* (e.g. "2535"); older
    // releases used a number — accept both.
    let build = data
        .get("build")
        .and_then(|b| match b {
            serde_json::Value::String(s) => s.parse::<i64>().ok(),
            serde_json::Value::Number(n) => n.as_i64(),
            _ => None,
        })
        .ok_or("no build found")?;
    Ok(build)
}

pub fn download_url(mc: &str, build: i64) -> String {
    format!("https://api.purpurmc.org/v2/purpur/{mc}/{build}/download")
}
