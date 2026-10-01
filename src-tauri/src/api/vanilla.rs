use serde::Deserialize;

use crate::api::client;

#[derive(Deserialize)]
struct Manifest {
    versions: Vec<ManifestVersion>,
}

#[derive(Deserialize)]
struct ManifestVersion {
    id: String,
    #[serde(rename = "type")]
    kind: String,
    url: String,
}

/// Returns minecraft versions from the official manifest, newest first.
/// `include_snapshots` also keeps `snapshot`-type entries (used only by
/// cores that can run them, e.g. vanilla/fabric/quilt).
pub async fn versions(include_snapshots: bool) -> Result<Vec<String>, String> {
    let resp = client()
        .get("https://piston-meta.mojang.com/mc/game/version_manifest_v2.json")
        .send()
        .await
        .map_err(|e| format!("vanilla manifest: {e}"))?;
    let manifest: Manifest = resp.json().await.map_err(|e| e.to_string())?;
    let list: Vec<String> = manifest
        .versions
        .into_iter()
        .filter(|v| match v.kind.as_str() {
            "release" => true,
            "snapshot" => include_snapshots,
            _ => false,
        })
        .map(|v| v.id)
        .collect();
    Ok(crate::utils::mcver::newest_first(list))
}

#[derive(Deserialize)]
struct VersionDetail {
    downloads: Downloads,
}

#[derive(Deserialize)]
struct Downloads {
    server: Option<Artifact>,
}

#[derive(Deserialize)]
struct Artifact {
    url: String,
}

/// Returns the official vanilla server jar download URL for a minecraft version.
pub async fn server_url(mc: &str) -> Result<String, String> {
    let manifest_url = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
    let resp = client()
        .get(manifest_url)
        .send()
        .await
        .map_err(|e| format!("vanilla manifest: {e}"))?;
    let manifest: Manifest = resp.json().await.map_err(|e| e.to_string())?;
    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == mc)
        .ok_or_else(|| format!("minecraft version {mc} not found"))?;
    let resp = client()
        .get(&entry.url)
        .send()
        .await
        .map_err(|e| format!("vanilla version: {e}"))?;
    let detail: VersionDetail = resp.json().await.map_err(|e| e.to_string())?;
    detail
        .downloads
        .server
        .map(|a| a.url)
        .ok_or_else(|| "no server artifact for version".into())
}
