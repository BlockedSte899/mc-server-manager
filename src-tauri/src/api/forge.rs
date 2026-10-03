use crate::api::client;
use crate::utils::download::extract_xml_tags;

/// Fetches the Forge promotions map, returns { mc_version: recommended_loader }.
pub async fn forge_promotions() -> Result<std::collections::HashMap<String, String>, String> {
    let url = "https://files.minecraftforge.net/net/minecraftforge/forge/promotions_slim.json";
    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Forge promotions API {}", resp.status()));
    }
    let data: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let promos = data
        .get("promos")
        .and_then(|p| p.as_object())
        .ok_or("bad promotions json")?;
    let mut out = std::collections::HashMap::new();
    for (k, v) in promos {
        if let (Some(lv), Some(val)) = (k.split_once('-').map(|(a, _)| a), v.as_str()) {
            out.insert(lv.to_string(), val.to_string());
        }
    }
    Ok(out)
}

/// All installer versions from the Forge maven metadata ("{mc}-{loader}").
async fn forge_metadata_versions() -> Result<Vec<String>, String> {
    let url = "https://maven.minecraftforge.net/net/minecraftforge/forge/maven-metadata.xml";
    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("Forge maven {}", resp.status()));
    }
    let xml = resp.text().await.map_err(|e| e.to_string())?;
    Ok(extract_xml_tags(&xml, "version"))
}

/// Numeric-aware loader comparison: "52.1.16" > "52.0.63" > "52.0.9".
fn cmp_loader(a: &str, b: &str) -> std::cmp::Ordering {
    let seg = |s: &str| -> Vec<u64> {
        s.split('.')
            .map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>())
            .filter(|p| !p.is_empty())
            .map(|p| p.parse::<u64>().unwrap_or(0))
            .collect()
    };
    let (a, b) = (seg(a), seg(b));
    for i in 0..a.len().max(b.len()) {
        let (x, y) = (a.get(i).unwrap_or(&0), b.get(i).unwrap_or(&0));
        match x.cmp(y) {
            std::cmp::Ordering::Equal => continue,
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// MineCraft versions that have any forge builds, newest first.
pub async fn forge_minecraft_versions() -> Result<Vec<String>, String> {
    let versions = forge_metadata_versions().await?;
    let mut mcs = std::collections::BTreeSet::new();
    for v in versions {
        // entries look like "1.21.1-52.1.16"
        if let Some((mc, _)) = v.split_once('-') {
            if mc.starts_with("1.") {
                mcs.insert(mc.to_string());
            }
        }
    }
    Ok(crate::utils::mcver::newest_first(mcs.into_iter().collect()))
}

/// Loader versions (e.g. "47.2.0") available for a minecraft version, newest first.
pub async fn forge_loaders(mc: &str) -> Result<Vec<String>, String> {
    let versions = forge_metadata_versions().await?;
    let prefix = format!("{mc}-");
    let mut loaders: Vec<String> = versions
        .iter()
        .filter_map(|v| v.strip_prefix(&prefix).map(|s| s.to_string()))
        .collect();
    // numeric-aware descending sort
    loaders.sort_by(|a, b| cmp_loader(b, a));
    Ok(loaders)
}

/// The recommended loader for a minecraft version, if any.
pub async fn forge_recommended_loader(mc: &str) -> Result<Option<String>, String> {
    let promos = forge_promotions().await?;
    let value = promos.get(mc).cloned().unwrap_or_default();
    if value == "latest" {
        return Ok(forge_loaders(mc).await?.into_iter().next());
    }
    if value.is_empty() {
        Ok(None)
    } else {
        Ok(Some(value))
    }
}

pub fn installer_url(mc: &str, loader: &str) -> String {
    // The maven artifact name carries the "forge-" prefix; "{mc}-{loader}-installer.jar"
    // (without it) returns 404.
    format!("https://maven.minecraftforge.net/net/minecraftforge/forge/{mc}-{loader}/forge-{mc}-{loader}-installer.jar")
}

/// All NeoForge loader versions, newest first.
pub async fn neoforge_versions() -> Result<Vec<String>, String> {
    let url = "https://maven.neoforged.net/releases/net/neoforged/neoforge/maven-metadata.xml";
    let resp = client().get(url).send().await.map_err(|e| e.to_string())?;
    let xml = resp.text().await.map_err(|e| e.to_string())?;
    let mut versions = extract_xml_tags(&xml, "version");
    versions.retain(|v| !(v.to_lowercase().contains("alpha") || v.to_lowercase().contains("beta")));
    versions.reverse();
    Ok(versions)
}

/// MineCraft versions that map to neoforge loaders, newest first.
pub async fn neoforge_minecraft_versions() -> Result<Vec<String>, String> {
    let versions = neoforge_versions().await?;
    let mut mcs = std::collections::BTreeSet::new();
    for v in versions {
        if let Some(mc) = loader_to_mc(&v) {
            mcs.insert(mc);
        }
    }
    Ok(mcs.into_iter().rev().collect())
}

/// Converts "21.1.144" -> "1.21.1", "20.4.237" -> "1.20.4".
pub fn loader_to_mc(loader: &str) -> Option<String> {
    let parts: Vec<&str> = loader.split('.').collect();
    if parts.len() >= 2 {
        let major: u32 = parts[0].parse().ok()?;
        let minor: u32 = parts[1].parse().ok()?;
        if major >= 8 {
            Some(format!("1.{major}.{minor}"))
        } else {
            Some(format!("1.{major}.{minor}"))
        }
    } else {
        None
    }
}

/// Loaders matching a minecraft version like "1.21.1" -> "21.1.x".
pub fn neoforge_loaders_for_mc(mc: &str, all: &[String]) -> Vec<String> {
    if let Some((major, minor)) = mc.strip_prefix("1.").and_then(|r| r.split_once('.')) {
        let prefix = format!("{major}.{minor}.");
        all.iter()
            .filter(|v| v.starts_with(&prefix))
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}

pub fn neoforge_installer_url(loader: &str) -> String {
    format!(
        "https://maven.neoforged.net/releases/net/neoforged/neoforge/{loader}/neoforge-{loader}-installer.jar"
    )
}
