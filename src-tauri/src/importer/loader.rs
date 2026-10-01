use std::path::Path;

use crate::api;
use crate::utils::download::download_file;
use tauri::AppHandle;

/// The build of a pack determines how the server launcher jar is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoaderKind {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    Vanilla,
}

pub struct InstallOutcome {
    pub core: String,
    pub loader_version: Option<String>,
    pub jar: String,
}

/// Extracts the first plausible dotted version from a dependency string
/// (handles forms like ">=47.2.0", "1.20.4", "0.16.10-beta").
pub fn clean_version(raw: &str) -> String {
    let mut best = String::new();
    let mut cur = String::new();
    for c in raw.trim().chars() {
        if c.is_ascii_digit() || c == '.' {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                best = cur.clone();
            }
            cur.clear();
        }
    }
    if !cur.is_empty() {
        best = cur;
    }
    best.trim_matches('.').to_string()
}

// extracts "maximal" latest loader version helper
async fn latest_of(list: Result<Vec<String>, String>) -> Result<String, String> {
    let mut v = list?;
    v.retain(|x| {
        let l = x.to_lowercase();
        !l.contains("alpha") && !l.contains("beta") && !l.contains("pre")
    });
    v.into_iter()
        .next()
        .ok_or_else(|| "no matching loader version found".to_string())
}

/// Resolves the value for a dependency key from the pack index.
/// Returns None if the dependency is absent or resolves to nothing.
pub async fn resolve_loader_value(
    raw: Option<&str>,
    kind: LoaderKind,
    mc: &str,
) -> Result<Option<String>, String> {
    let cleaned = raw.map(clean_version).unwrap_or_default();
    if !cleaned.is_empty() {
        return Ok(Some(cleaned));
    }
    match kind {
        LoaderKind::Fabric => Ok(Some(
            latest_of(api::fabric::fabric_loader_versions().await).await?,
        )),
        LoaderKind::Quilt => Ok(Some(
            latest_of(api::fabric::quilt_loader_versions().await).await?,
        )),
        LoaderKind::Forge => Ok(api::forge::forge_recommended_loader(mc).await?),
        LoaderKind::NeoForge => {
            let all = api::forge::neoforge_versions().await?;
            let matches = api::forge::neoforge_loaders_for_mc(mc, &all);
            Ok(matches.into_iter().next())
        }
        LoaderKind::Vanilla => Ok(None),
    }
}

/// Installs the loader / server jar into `server_dir` and returns launch info.
pub async fn install_loader(
    app: &AppHandle,
    java: &Path,
    server_dir: &Path,
    minecraft: &str,
    raw_loader: Option<&str>,
    kind: LoaderKind,
    progress_key: &str,
) -> Result<InstallOutcome, String> {
    let client = api::client();
    match kind {
        LoaderKind::Fabric | LoaderKind::Quilt => {
            let loader = resolve_loader_value(raw_loader, kind, minecraft)
                .await?
                .ok_or("loader version unresolved")?;
            let tmp = server_dir.join(".installers");
            std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
            let (installer_url, installer_name, run) = if kind == LoaderKind::Fabric {
                let iv = api::fabric::fabric_installer_latest().await?;
                (
                    api::fabric::fabric_installer_url(&iv),
                    format!("fabric-installer-{iv}.jar"),
                    "fabric",
                )
            } else {
                let iv = api::fabric::quilt_installer_latest().await?;
                (
                    api::fabric::quilt_installer_url(&iv),
                    format!("quilt-installer-{iv}.jar"),
                    "quilt",
                )
            };
            let installer_path = tmp.join(&installer_name);
            download_file(&client, &installer_url, &installer_path, app, progress_key).await?;
            let jar = if run == "fabric" {
                crate::server::installers::install_fabric(
                    java,
                    server_dir,
                    &installer_path,
                    minecraft,
                    &loader,
                )
                .await?
            } else {
                crate::server::installers::install_quilt(
                    java,
                    server_dir,
                    &installer_path,
                    minecraft,
                    &loader,
                )
                .await?
            };
            let _ = std::fs::remove_dir_all(&tmp);
            Ok(InstallOutcome {
                core: run.to_string(),
                loader_version: Some(loader),
                jar,
            })
        }
        LoaderKind::Forge | LoaderKind::NeoForge => {
            let loader = resolve_loader_value(raw_loader, kind, minecraft)
                .await?
                .ok_or("loader version unresolved")?;
            let (url, filename) = if kind == LoaderKind::Forge {
                (
                    api::forge::installer_url(minecraft, &loader),
                    format!("forge-{minecraft}-{loader}-installer.jar"),
                )
            } else {
                (
                    api::forge::neoforge_installer_url(&loader),
                    format!("neoforge-{loader}-installer.jar"),
                )
            };
            let installer_path = server_dir.join(&filename);
            download_file(&client, &url, &installer_path, app, progress_key).await?;
            let core = if kind == LoaderKind::Forge {
                "forge"
            } else {
                "neoforge"
            };
            let jar = crate::server::manager::install_forge(
                app, java, server_dir, core, minecraft, &loader, &filename,
            )
            .await?
            .ok_or("unable to locate installed server jar")?;
            Ok(InstallOutcome {
                core: core.to_string(),
                loader_version: Some(loader),
                jar,
            })
        }
        LoaderKind::Vanilla => {
            let url = api::vanilla::server_url(minecraft).await?;
            let jar = format!("minecraft_server-{minecraft}.jar");
            download_file(&client, &url, &server_dir.join(&jar), app, progress_key).await?;
            Ok(InstallOutcome {
                core: "vanilla".into(),
                loader_version: None,
                jar,
            })
        }
    }
}
