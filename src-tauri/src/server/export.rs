use serde::Serialize;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use crate::server::manager::ServerMeta;
use crate::utils::download::file_sha1;
use crate::utils::paths;

#[derive(Debug, Clone, Serialize)]
pub struct ExportResult {
    pub path: String,
    pub name: String,
    pub files: usize,
}

fn loader_name(core: &str) -> Option<&'static str> {
    match core {
        "forge" => Some("forge"),
        "neoforge" => Some("neoforge"),
        "fabric" => Some("fabric-loader"),
        "quilt" => Some("quilt-loader"),
        _ => None,
    }
}

/// Exports the server's mod/plugin files as a Modrinth `.mrpack` that the
/// app's own Import page can re-import (files are embedded, offline-capable).
pub fn export_mrpack(
    root: &Path,
    meta: &ServerMeta,
    servers_dir: &Path,
) -> Result<ExportResult, String> {
    let exports_dir = servers_dir.join("exports");
    std::fs::create_dir_all(&exports_dir).map_err(|e| e.to_string())?;
    let base = if meta.name.is_empty() {
        "server".to_string()
    } else {
        meta.name.clone()
    };
    let fname = format!("{}.mrpack", crate::utils::paths::sane_filename(&base));
    let dest = exports_dir.join(&fname);

    let file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let mod_dir = meta.mod_folder(); // "mods" or "plugins"
    let mod_path = root.join(mod_dir);
    std::fs::create_dir_all(&mod_path).ok();

    let mut pack_files: Vec<serde_json::Value> = Vec::new();
    let mut jar_items = 0usize;

    if let Ok(entries) = std::fs::read_dir(&mod_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if !path.is_file() {
                continue;
            }
            if !(name.ends_with(".jar")
                || name.ends_with(".jar.disabled")
                || name.ends_with(".zip")
                || name.ends_with(".jar.disabled.zip"))
            {
                continue;
            }
            let rel = format!("{mod_dir}/{name}");
            let sha = file_sha1(&path).unwrap_or_default();
            zip.start_file(rel.clone(), opts)
                .map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, &mut zip).map_err(|e| e.to_string())?;
            let mut hashes = HashMap::new();
            hashes.insert("sha1".to_string(), sha);
            pack_files.push(serde_json::json!({
                "path": rel,
                "hashes": hashes,
                "env": { "client": "unsupported", "server": "required" },
                "downloads": [],
            }));
            jar_items += 1;
        }
    }

    let mut deps = HashMap::new();
    deps.insert("minecraft".to_string(), meta.mc_version.clone());
    if let Some(l) = loader_name(&meta.core) {
        deps.insert(
            l.to_string(),
            meta.loader_version
                .clone()
                .unwrap_or_else(|| "*".to_string()),
        );
    }

    let index = serde_json::json!({
        "formatVersion": 1,
        "game": "minecraft",
        "versionId": meta.mc_version,
        "name": meta.name,
        "summary": format!("Exported from MC Server Manager ({})", meta.core),
        "files": pack_files,
        "dependencies": deps,
    });
    zip.start_file("modrinth.index.json", opts)
        .map_err(|e| e.to_string())?;
    let idx_bytes = serde_json::to_vec_pretty(&index).map_err(|e| e.to_string())?;
    zip.write_all(&idx_bytes).map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;

    Ok(ExportResult {
        path: dest.to_string_lossy().into_owned(),
        name: fname,
        files: jar_items,
    })
}
