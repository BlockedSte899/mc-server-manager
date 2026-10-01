use crate::api::client;
use crate::utils::download::extract_xml_tags;

pub async fn fabric_loader_versions() -> Result<Vec<String>, String> {
    let url = "https://maven.fabricmc.net/net/fabricmc/fabric-loader/maven-metadata.xml";
    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    let xml = resp.text().await.map_err(|e| e.to_string())?;
    let mut versions = extract_xml_tags(&xml, "version");
    versions.retain(|v| !v.to_lowercase().contains("pre") && !v.to_lowercase().contains("build"));
    versions.reverse();
    Ok(versions)
}

pub async fn fabric_installer_versions() -> Result<Vec<String>, String> {
    let url = "https://maven.fabricmc.net/net/fabricmc/fabric-installer/maven-metadata.xml";
    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    let xml = resp.text().await.map_err(|e| e.to_string())?;
    let mut versions = extract_xml_tags(&xml, "version");
    versions.reverse();
    Ok(versions)
}

pub async fn fabric_installer_latest() -> Result<String, String> {
    let mut v = fabric_installer_versions().await?;
    v.retain(|x| !x.to_lowercase().contains("pre"));
    v.into_iter()
        .next()
        .ok_or_else(|| "no fabric installer".into())
}

pub fn fabric_installer_url(version: &str) -> String {
    format!("https://maven.fabricmc.net/net/fabricmc/fabric-installer/{version}/fabric-installer-{version}.jar")
}

pub async fn quilt_loader_versions() -> Result<Vec<String>, String> {
    let url =
        "https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-loader/maven-metadata.xml";
    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    let xml = resp.text().await.map_err(|e| e.to_string())?;
    let mut versions = extract_xml_tags(&xml, "version");
    versions.reverse();
    Ok(versions)
}

pub async fn quilt_installer_latest() -> Result<String, String> {
    let url = "https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-installer/maven-metadata.xml";
    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    let xml = resp.text().await.map_err(|e| e.to_string())?;
    let mut versions = extract_xml_tags(&xml, "version");
    versions.reverse();
    versions
        .into_iter()
        .next()
        .ok_or_else(|| "no quilt installer".into())
}

pub fn quilt_installer_url(version: &str) -> String {
    format!("https://maven.quiltmc.org/repository/release/org/quiltmc/quilt-installer/{version}/quilt-installer-{version}.jar")
}

/// Loader versions that can actually run a given Minecraft version (Fabric
/// meta API, newest first).
pub async fn fabric_loaders_for(mc: &str) -> Result<Vec<String>, String> {
    let url = format!("https://meta.fabricmc.net/v2/versions/loader/{mc}");
    let resp = client().get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    let arr: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(arr
        .into_iter()
        .filter_map(|v| v.get("loader")?.get("version")?.as_str().map(String::from))
        .collect())
}

/// Loader versions compatible with a given Minecraft version (Quilt meta API).
pub async fn quilt_loaders_for(mc: &str) -> Result<Vec<String>, String> {
    let url = format!("https://meta.quiltmc.org/v3/versions/loader/{mc}");
    let resp = client().get(&url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Ok(Vec::new());
    }
    let arr: Vec<serde_json::Value> = resp.json().await.map_err(|e| e.to_string())?;
    Ok(arr
        .into_iter()
        .filter_map(|v| v.get("loader")?.get("version")?.as_str().map(String::from))
        .collect())
}
