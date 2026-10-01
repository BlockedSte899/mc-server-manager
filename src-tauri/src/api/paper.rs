use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::api::client;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PaperDownload {
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
}

#[derive(Deserialize)]
struct ProjectsResp {
    #[serde(default)]
    versions: BTreeMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct BuildsLatestResp {
    #[serde(default)]
    downloads: BTreeMap<String, PaperDownload>,
}

async fn get_json(url: &str) -> Result<String, String> {
    let resp = client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("PaperMC API error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("PaperMC API {} for {url}", resp.status()));
    }
    resp.text().await.map_err(|e| format!("API error: {e}"))
}

fn is_stable_release(v: &str) -> bool {
    // stable ids look like "26.3" or "1.21.9"; rcs/snapshots contain '-'
    v.chars().all(|c| c.is_ascii_digit() || c == '.')
}

/// Returns available (stable) minecraft/velocity versions for a PaperMC project
/// from the v3 Fill API.
pub async fn versions(project: &str) -> Result<Vec<String>, String> {
    let url = format!("https://fill.papermc.io/v3/projects/{project}");
    let text = get_json(&url).await?;
    let data: ProjectsResp = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for list in data.versions.values() {
        for v in list {
            if !is_stable_release(v) || v.is_empty() {
                continue;
            }
            if seen.insert(v.clone()) {
                out.push(v.clone());
            }
        }
    }
    Ok(crate::utils::mcver::newest_first(out))
}

/// Returns the latest build's download artifact for a project/version.
pub async fn latest_build(project: &str, version: &str) -> Result<PaperDownload, String> {
    let url =
        format!("https://fill.papermc.io/v3/projects/{project}/versions/{version}/builds/latest");
    let text = get_json(&url).await?;
    let data: BuildsLatestResp = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    data.downloads
        .get("server:default")
        .cloned()
        .ok_or_else(|| format!("no server download artifact for {project} {version}"))
}
