use std::io::Read;

use serde::Deserialize;
use tauri::AppHandle;

use crate::api;
use crate::importer::loader::{clean_version, install_loader, InstallOutcome, LoaderKind};
use crate::server::manager::{default_server_properties, save_meta, ServerMeta};
use crate::settings::Settings;
use crate::utils::download::download_file;
use crate::utils::paths::now_iso;
use crate::utils::slug::{slugify, unique_folder};

#[derive(Debug, Deserialize)]
pub struct CfManifest {
    #[serde(default)]
    pub minecraft: CfMinecraft,
    #[serde(default)]
    pub modLoaders: Vec<String>,
    #[serde(default)]
    pub files: Vec<CfManifestFile>,
    #[serde(default)]
    pub overrides: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct CfMinecraft {
    #[serde(default)]
    pub version: String,
}

#[derive(Debug, Deserialize)]
pub struct CfManifestFile {
    #[serde(default)]
    pub projectID: i64,
    #[serde(default)]
    pub fileID: i64,
    #[serde(default)]
    pub required: bool,
}

fn loader_kind(raw: &str) -> Option<(LoaderKind, String)> {
    let (kind, ver) = raw.split_once('-')?;
    let ver = clean_version(ver);
    let kind = kind.to_lowercase();
    let lk = match kind.as_str() {
        "fabric" => LoaderKind::Fabric,
        "quilt" => LoaderKind::Quilt,
        "forge" => LoaderKind::Forge,
        "neoforge" => LoaderKind::NeoForge,
        _ => return None,
    };
    Some((lk, ver))
}

/// Extracts a CurseForge "overrides" folder recursively into the server dir.
fn extract_overrides(
    zip: &mut zip::ZipArchive<std::fs::File>,
    server_dir: &std::path::Path,
    folder: &str,
) -> Result<(), String> {
    let prefix = format!("{folder}/");
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let Some(rel) = name.strip_prefix(&prefix) else {
            continue;
        };
        if rel.is_empty() {
            continue;
        }
        let target = server_dir.join(rel.trim_end_matches('/'));
        if name.ends_with('/') {
            std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
            continue;
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
        std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub async fn import(
    app: &AppHandle,
    settings: &Settings,
    req: crate::importer::ImportRequest,
) -> Result<ServerMeta, String> {
    let src = std::path::Path::new(&req.path);
    let file = std::fs::File::open(src).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("invalid zip: {e}"))?;

    let mut manifest_text = String::new();
    zip.by_name("manifest.json")
        .map_err(|e| e.to_string())?
        .read_to_string(&mut manifest_text)
        .map_err(|e| e.to_string())?;
    let manifest: CfManifest = serde_json::from_str(&manifest_text).map_err(|e| e.to_string())?;

    let minecraft = manifest.minecraft.version;
    if minecraft.is_empty() {
        return Err("manifest does not declare a minecraft version".into());
    }
    let name = req
        .name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .or_else(|| manifest.name.clone())
        .unwrap_or_else(|| "Imported Server".to_string());

    let (id, dir) = unique_folder(&settings.servers_dir(), &slugify(&name));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let progress_key = format!("server:{id}:core");

    let java = crate::server::java::resolve_java(settings, req.java).await?;

    // choose loader from the manifest's first modLoader entry
    let loader_spec = manifest
        .modLoaders
        .first()
        .map(|s| s.as_str())
        .unwrap_or_default();
    let (kind, loader_ver) = match loader_kind(loader_spec) {
        Some((k, v)) => (k, Some(v)),
        None => (LoaderKind::Vanilla, None),
    };
    let loader_ver_ref = loader_ver.as_deref();
    let mut outcome: InstallOutcome = install_loader(
        app,
        &java,
        &dir,
        &minecraft,
        loader_ver_ref,
        kind,
        &progress_key,
    )
    .await?;

    // overrides
    let overrides_folder = manifest.overrides.as_deref().unwrap_or("overrides");
    extract_overrides(&mut zip, &dir, overrides_folder)?;

    // resolve & download mods via CurseForge (requires API key)
    let curseforge_key = settings.curseforge_api_key.clone().unwrap_or_default();
    let mut warnings: Vec<String> = Vec::new();
    let client = api::client();
    let key_present = !curseforge_key.trim().is_empty();
    let mut count = 0u32;
    for mf in &manifest.files {
        if !key_present {
            warnings.push(format!(
                "CurseForge API key missing — mod projectID {} not downloaded",
                mf.projectID
            ));
            continue;
        }
        let dl_url = match api::curseforge::file_download_url(
            &curseforge_key,
            mf.projectID,
            mf.fileID,
        )
        .await
        {
            Ok(Some(u)) => u,
            Ok(None) => {
                warnings.push(format!("no download for file {}", mf.fileID));
                continue;
            }
            Err(e) => {
                warnings.push(format!("project {}: {e}", mf.projectID));
                continue;
            }
        };
        let filename = dl_url
            .rsplit('/')
            .next()
            .map(|s| s.to_string())
            .unwrap_or_else(|| format!("mod-{}.jar", mf.projectID));
        let target = dir.join("mods").join(&filename);
        let key = format!("server:{id}:pack:{}", count);
        if let Err(e) = download_file(&client, &dl_url, &target, app, &key).await {
            if mf.required {
                return Err(e);
            }
            warnings.push(format!("{filename}: {e}"));
        }
        count += 1;
    }

    std::fs::write(dir.join("eula.txt"), "eula=true\n").map_err(|e| e.to_string())?;
    crate::utils::props::write_properties(
        &dir.join("server.properties"),
        &default_server_properties(&name),
    )?;

    let meta = ServerMeta {
        name,
        id,
        java: req.java,
        core: outcome.core.clone(),
        mc_version: minecraft,
        loader_version: outcome.loader_version.take(),
        build: None,
        jar: outcome.jar.clone(),
        min_ram: req.min_ram,
        max_ram: req.max_ram,
        created_at: now_iso(),
        is_proxy: false,
        source_url: Some(req.path.clone()),
        warnings: warnings.clone(),
        auto_backup_days: 0,
        auto_backup_keep: 0,
        java_path: None,
        jvm_preset: "auto".to_string(),
        jvm_args: String::new(),
        auto_start_on_boot: false,
        auto_restart_on_crash: false,
        schedule: Vec::new(),
    };
    save_meta(&dir, &meta)?;
    Ok(meta)
}
