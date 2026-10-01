use std::collections::HashMap;
use std::io::Read;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;

use crate::api;
use crate::importer::loader::{clean_version, install_loader, InstallOutcome, LoaderKind};
use crate::server::manager::{default_server_properties, save_meta, ServerMeta};
use crate::settings::Settings;
use crate::utils::download::{download_file, file_sha1};
use crate::utils::paths::now_iso;
use crate::utils::slug::{slugify, unique_folder};

#[derive(Debug, Deserialize, Serialize)]
pub struct MrpackIndex {
    #[serde(default)]
    pub formatVersion: i64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub versionId: String,
    #[serde(default)]
    pub files: Vec<PackFile>,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(default)]
    pub overrides: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackFile {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub hashes: HashMap<String, String>,
    #[serde(default)]
    pub env: Option<PackEnv>,
    #[serde(default)]
    pub downloads: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PackEnv {
    #[serde(default)]
    pub client: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

fn clean_index(entry: &mut zip::ZipArchive<std::fs::File>) -> bool {
    entry.by_name("modrinth.index.json").is_ok()
}

/// Extracts override folders into the server directory.
fn extract_overrides(
    zip: &mut zip::ZipArchive<std::fs::File>,
    server_dir: &std::path::Path,
    overrides: &Option<HashMap<String, String>>,
) -> Result<(), String> {
    let map = match overrides {
        Some(m) => m,
        None => return Ok(()),
    };
    let mut entries: Vec<String> = Vec::new();
    for i in 0..zip.len() {
        if let Ok(e) = zip.by_index(i) {
            entries.push(e.name().to_string());
        }
    }
    for (folder, dest) in map {
        let prefix = format!("{folder}/");
        let dest_root =
            if dest.is_empty() || dest == "." || dest == "minecraft" || dest == "instance" {
                server_dir.to_path_buf()
            } else {
                server_dir.join(dest.trim_start_matches('/'))
            };
        for name in &entries {
            let Some(rel) = name.strip_prefix(&prefix) else {
                continue;
            };
            if rel.is_empty() {
                continue;
            }
            let target = dest_root.join(rel.trim_end_matches('/'));
            if name.ends_with('/') {
                std::fs::create_dir_all(&target).map_err(|e| e.to_string())?;
                continue;
            }
            let mut file = zip.by_name(name).map_err(|e| e.to_string())?;
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = std::fs::File::create(&target).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut out).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Maps a pack file path to a target inside the server dir.
fn resolve_file_target(server_dir: &std::path::Path, path: &str) -> std::path::PathBuf {
    let rel = path.strip_prefix("minecraft/").unwrap_or(path);
    let rel = rel.trim_start_matches('/');
    server_dir.join(rel)
}

fn loader_from_deps(deps: &HashMap<String, String>) -> (LoaderKind, Option<String>) {
    if let Some(v) = deps.get("fabric-loader") {
        (LoaderKind::Fabric, Some(v.clone()))
    } else if let Some(v) = deps.get("quilt-loader") {
        (LoaderKind::Quilt, Some(v.clone()))
    } else if let Some(v) = deps.get("neoforge") {
        (LoaderKind::NeoForge, Some(v.clone()))
    } else if let Some(v) = deps.get("forge") {
        (LoaderKind::Forge, Some(v.clone()))
    } else {
        (LoaderKind::Vanilla, None)
    }
}

pub async fn import(
    app: &AppHandle,
    settings: &Settings,
    req: crate::importer::ImportRequest,
) -> Result<ServerMeta, String> {
    let src = std::path::Path::new(&req.path);
    let file = std::fs::File::open(src).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("invalid mrpack: {e}"))?;
    if !clean_index(&mut zip) {
        return Err("not a valid .mrpack (missing modrinth.index.json)".into());
    }
    let mut index_text = String::new();
    zip.by_name("modrinth.index.json")
        .map_err(|e| e.to_string())?
        .read_to_string(&mut index_text)
        .map_err(|e| e.to_string())?;
    let index: MrpackIndex = serde_json::from_str(&index_text).map_err(|e| e.to_string())?;

    let minecraft = clean_version(
        index
            .dependencies
            .get("minecraft")
            .map(|s| s.as_str())
            .unwrap_or(""),
    );
    if minecraft.is_empty() {
        return Err("pack does not declare a minecraft version".into());
    }

    let name = req
        .name
        .clone()
        .filter(|n| !n.trim().is_empty())
        .unwrap_or_else(|| {
            if index.name.is_empty() {
                "Imported Server".to_string()
            } else {
                index.name.clone()
            }
        });

    let (id, dir) = unique_folder(&settings.servers_dir(), &slugify(&name));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let progress_key = format!("server:{id}:core");

    let java = crate::server::java::resolve_java(settings, req.java).await?;
    let (kind, raw_loader) = loader_from_deps(&index.dependencies);
    let raw_loader_ref = raw_loader.as_deref();

    let mut outcome: InstallOutcome = install_loader(
        app,
        &java,
        &dir,
        &minecraft,
        raw_loader_ref,
        kind,
        &progress_key,
    )
    .await?;

    extract_overrides(&mut zip, &dir, &index.overrides)?;

    // download declared files (client-uninstalls/server-unsupported skipped)
    let client = api::client();
    let mut warnings: Vec<String> = Vec::new();
    let mut count = 0u32;
    for f in &index.files {
        let server_env = f
            .env
            .as_ref()
            .and_then(|e| e.server.as_deref())
            .unwrap_or("required");
        if server_env == "unsupported" {
            continue;
        }
        let target = resolve_file_target(&dir, &f.path);
        // Prefer extracting an embedded copy of the file (offline packs, e.g. our exports).
        let embedded: Option<Vec<u8>> = {
            let candidates = [f.path.clone(), format!("minecraft/{}", f.path)];
            let mut found = None;
            for name in candidates {
                if let Ok(mut entry) = zip.by_name(&name) {
                    let mut buf = Vec::new();
                    if entry.read_to_end(&mut buf).is_ok() {
                        found = Some(buf);
                    }
                    break;
                }
            }
            found
        };
        if let Some(buf) = embedded {
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            std::fs::write(&target, &buf).map_err(|e| e.to_string())?;
            if !buf.is_empty() {
                count += 1;
                continue;
            }
        }
        let Some(url) = f.downloads.iter().find(|u| !u.is_empty()) else {
            count += 1;
            continue;
        };
        let key = format!("server:{id}:pack:{}", count);
        if let Err(e) = download_file(&client, url, &target, app, &key).await {
            if server_env == "optional" {
                warnings.push(format!("skipped optional file {}: {e}", f.path));
                continue;
            }
            return Err(e);
        }
        if let Some(sha1) = f.hashes.get("sha1") {
            if let Ok(actual) = file_sha1(&target) {
                if !actual.eq_ignore_ascii_case(sha1) {
                    return Err(format!("sha1 mismatch for {}", f.path));
                }
            }
        }
        count += 1;
    }

    // eula + props + meta
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

    use tauri::Emitter;
    #[derive(Clone, Serialize)]
    struct Stage {
        id: String,
        stage: String,
    }
    let _ = app.emit(
        "imp:progress",
        Stage {
            id: meta.id.clone(),
            stage: "done".into(),
        },
    );
    println!(
        "imported mrpack: {} ({} files) warnings: {:?}",
        meta.name, count, warnings
    );
    Ok(meta)
}
