use serde::{Deserialize, Serialize};

use crate::api::client;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfMod {
    pub id: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub downloadCount: i64,
    #[serde(default)]
    pub logo: Option<CfLogo>,
    #[serde(default)]
    pub latestFilesIndexes: Vec<CfFileIndex>,
    /// Only present on the bulk `/v1/mods` endpoint, not on search.
    #[serde(default)]
    pub screenshots: Vec<CfScreenshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfScreenshot {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub thumbnailUrl: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfLogo {
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfFileIndex {
    #[serde(default)]
    pub gameVersion: Option<String>,
    #[serde(default)]
    pub fileId: i64,
    #[serde(default)]
    pub gameVersionTypeId: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfFile {
    pub id: i64,
    #[serde(default)]
    pub fileName: String,
    #[serde(default)]
    pub downloadUrl: Option<String>,
    #[serde(default)]
    pub length: i64,
    #[serde(default)]
    pub gameVersions: Vec<String>,
    #[serde(default)]
    pub releaseType: Option<String>,
}

#[derive(Deserialize)]
struct CfResp<T> {
    data: Vec<T>,
}

pub enum CfError {
    MissingKey,
    Api(String),
}

impl std::fmt::Display for CfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CfError::MissingKey => {
                write!(f, "A CurseForge API key is required. Add it in Settings.")
            }
            CfError::Api(e) => write!(f, "{e}"),
        }
    }
}

impl From<CfError> for String {
    fn from(e: CfError) -> Self {
        e.to_string()
    }
}

fn loaders_id(loader: &str) -> Option<i64> {
    match loader {
        "vanilla" => Some(0),
        "forge" => Some(1),
        "fabric" => Some(4),
        "quilt" => Some(5),
        "neoforge" => Some(6),
        _ => None,
    }
}

async fn get<T: serde::de::DeserializeOwned>(key: &str, url: &str) -> Result<Vec<T>, CfError> {
    if key.trim().is_empty() {
        return Err(CfError::MissingKey);
    }
    let resp = client()
        .get(url)
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| CfError::Api(format!("CurseForge request: {e}")))?;
    if resp.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err(CfError::Api("CurseForge API key rejected (401)".into()));
    }
    if !resp.status().is_success() {
        return Err(CfError::Api(format!("CurseForge API {}", resp.status())));
    }
    let data: CfResp<T> = resp
        .json()
        .await
        .map_err(|e| CfError::Api(format!("CurseForge parse: {e}")))?;
    Ok(data.data)
}

pub async fn search(
    key: &str,
    query: &str,
    game_version: &str,
    loader: &str,
    limit: i64,
) -> Result<Vec<CfMod>, String> {
    let mut url = format!(
        "https://api.curseforge.com/v1/mods/search?gameId=432&searchFilter={}&sortField=6&sortOrder=desc&pageSize={}",
        crate::utils::download::urlencode(query),
        limit.clamp(1, 50)
    );
    if !game_version.is_empty() {
        url.push_str(&format!(
            "&gameVersion={}",
            crate::utils::download::urlencode(game_version)
        ));
    }
    if let Some(id) = loaders_id(loader) {
        url.push_str(&format!("&modLoaderType={id}"));
    }
    get(key, &url).await.map_err(Into::into)
}

/// Fetches full mod records (incl. `screenshots`) for the given ids in one call.
pub async fn mods_bulk(key: &str, ids: &[String]) -> Result<Vec<CfMod>, String> {
    if key.trim().is_empty() {
        return Err(CfError::MissingKey.into());
    }
    let num_ids: Vec<i64> = ids.iter().filter_map(|s| s.parse().ok()).collect();
    if num_ids.is_empty() {
        return Ok(Vec::new());
    }
    let body = serde_json::json!({ "modIds": num_ids });
    let resp = client()
        .post("https://api.curseforge.com/v1/mods")
        .header("x-api-key", key)
        .header("Accept", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| CfError::Api(format!("CurseForge request: {e}")))?;
    if !resp.status().is_success() {
        return Err(CfError::Api(format!("CurseForge API {}", resp.status())).into());
    }
    let data: CfResp<CfMod> = resp
        .json()
        .await
        .map_err(|e| CfError::Api(format!("CurseForge parse: {e}")))?;
    Ok(data.data)
}

pub async fn files(key: &str, project_id: i64) -> Result<Vec<CfFile>, String> {
    let url = format!("https://api.curseforge.com/v1/mods/{project_id}/files?pageSize=1000");
    get(key, &url).await.map_err(Into::into)
}

pub async fn file_download_url(
    key: &str,
    project_id: i64,
    file_id: i64,
) -> Result<Option<String>, String> {
    let files = files(key, project_id).await?;
    Ok(files
        .into_iter()
        .find(|f| f.id == file_id)
        .and_then(|f| f.downloadUrl))
}
