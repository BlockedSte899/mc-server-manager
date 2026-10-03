use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::api::client;
use crate::utils::download::urlencode;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GalleryItem {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub featured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthProject {
    /// Search hits call it "project_id", the full project schema (bulk
    /// endpoint) calls it "id" — accept both.
    #[serde(default, alias = "id")]
    pub project_id: String,
    #[serde(default)]
    pub slug: Option<String>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub project_type: String,
    #[serde(default)]
    pub downloads: i64,
    #[serde(default)]
    pub icon_url: Option<String>,
    #[serde(default)]
    pub server_side: Option<String>,
    #[serde(default)]
    pub client_side: Option<String>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    /// Search hits don't include it; the bulk projects endpoint does.
    #[serde(default)]
    pub gallery: Vec<GalleryItem>,
}

/// Fetches full project records (incl. `gallery`) for the given ids in one call.
pub async fn projects_bulk(ids: &[String]) -> Result<Vec<ModrinthProject>, String> {
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    let ids_json = serde_json::to_string(ids).map_err(|e| e.to_string())?;
    // The JSON array must be percent-encoded — raw [ ] " , in a query string
    // break the request.
    let url = format!(
        "https://api.modrinth.com/v2/projects?ids={}",
        crate::utils::download::urlencode(&ids_json)
    );
    let resp = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Modrinth API error: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Modrinth API {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
    #[serde(default)]
    pub size: i64,
    #[serde(default)]
    pub primary: bool,
    #[serde(default)]
    pub env: Option<FileEnv>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FileEnv {
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModrinthVersion {
    pub id: String,
    #[serde(default)]
    pub project_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version_number: String,
    #[serde(default)]
    pub date_published: String,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub dependency_type: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
}

#[derive(Deserialize)]
struct SearchResp {
    #[serde(default)]
    hits: Vec<ModrinthProject>,
}

pub async fn search(
    query: &str,
    project_type: &str,
    loaders: &[String],
    game_version: Option<&str>,
    limit: i64,
) -> Result<Vec<ModrinthProject>, String> {
    let mut facets: Vec<Vec<String>> = Vec::new();
    if !project_type.is_empty() {
        facets.push(vec![format!("project_type:{project_type}")]);
    }
    if let Some(loader) = loaders.first() {
        facets.push(vec![format!("loaders:{loader}")]);
    }
    if let Some(gv) = game_version {
        facets.push(vec![format!("versions:{gv}")]);
    }
    let facets_json = serde_json::to_string(&facets).map_err(|e| e.to_string())?;
    let url = format!(
        "https://api.modrinth.com/v2/search?query={}&limit={}&facets={}",
        urlencode(query),
        limit,
        urlencode(&facets_json)
    );
    let resp = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Modrinth search: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Modrinth search {}", resp.status()));
    }
    let data: SearchResp = resp.json().await.map_err(|e| e.to_string())?;
    Ok(data.hits)
}

pub async fn project_versions(
    project_id: &str,
    game_version: Option<&str>,
    loader: Option<&str>,
) -> Result<Vec<ModrinthVersion>, String> {
    let url = format!(
        "https://api.modrinth.com/v2/project/{}/version",
        urlencode(project_id)
    );
    let resp = client()
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Modrinth versions: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Modrinth versions {}", resp.status()));
    }
    let mut versions: Vec<ModrinthVersion> = resp.json().await.map_err(|e| e.to_string())?;
    versions.retain(|v| {
        let ok_game = game_version
            .map(|g| v.game_versions.iter().any(|x| x == g))
            .unwrap_or(true);
        let ok_loader = loader
            .map(|l| v.loaders.iter().any(|x| x == l))
            .unwrap_or(true);
        ok_game && ok_loader
    });
    versions.sort_by(|a, b| b.date_published.cmp(&a.date_published));
    Ok(versions)
}

/// Fetches a remote icon via the native HTTP client (bypassing the webview's
/// image network) and returns it as a `data:image/...;base64,...` URL.
pub async fn icon_data_url(url: &str) -> Result<Option<String>, String> {
    if !url.starts_with("https://") {
        return Ok(None);
    }
    let resp = client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Modrinth icon: {e}"))?;
    if !resp.status().is_success() {
        return Ok(None);
    }
    let ctype = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();
    let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
    if bytes.is_empty() || bytes.len() > 5_000_000 {
        return Ok(None);
    }
    let b64 = base64::encode(&bytes);
    Ok(Some(format!("data:{ctype};base64,{b64}")))
}

/// Maps a server core to the modrinth loader + project type for filtering.
pub fn loader_for_core(core: &str) -> (&'static str, &'static str) {
    match core {
        "paper" | "purpur" | "spigot" | "bukkit" => ("paper", "plugin"),
        "velocity" => ("velocity", "plugin"),
        "forge" => ("forge", "mod"),
        "neoforge" => ("neoforge", "mod"),
        "fabric" => ("fabric", "mod"),
        "quilt" => ("quilt", "mod"),
        _ => ("", "mod"),
    }
}
