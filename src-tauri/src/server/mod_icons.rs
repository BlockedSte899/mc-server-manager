use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde::Deserialize;
use std::io::Read;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

const MAX_ICON_BYTES: u64 = 256 * 1024;

#[derive(Deserialize)]
struct FabricMeta {
    #[serde(default)]
    icon: Option<String>,
}

#[derive(Deserialize)]
struct ForgeMeta {
    #[serde(default)]
    mods: Vec<ForgeMod>,
}

#[derive(Deserialize)]
struct ForgeMod {
    #[serde(default)]
    logoFile: Option<String>,
}

/// Try to extract a mod JAR's icon as a base64 data URL.
/// Supports fabric/quilt (`fabric.mod.json` / `quilt.mod.json` "icon" field)
/// and forge (`META-INF/mods.toml` "logoFile", defaulting to root "logo.png").
pub fn mod_icon(jar: &Path) -> Option<String> {
    let file = std::fs::File::open(jar).ok()?;
    let mut archive = ZipArchive::new(file).ok()?;

    let mut candidates: Vec<String> = Vec::new();

    for meta_name in ["fabric.mod.json", "quilt.mod.json"] {
        if let Ok(mut f) = archive.by_name(meta_name) {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                if let Ok(meta) = serde_json::from_str::<FabricMeta>(&s) {
                    if let Some(icon) = meta.icon {
                        if !icon.is_empty() {
                            candidates.push(icon);
                        }
                    }
                }
            }
        }
    }

    if archive.by_name("META-INF/mods.toml").is_ok() {
        let mut toml_icon = None;
        if let Ok(mut f) = archive.by_name("META-INF/mods.toml") {
            let mut s = String::new();
            if f.read_to_string(&mut s).is_ok() {
                if let Ok(meta) = toml::from_str::<ForgeMeta>(&s) {
                    toml_icon = meta
                        .mods
                        .iter()
                        .find_map(|m| m.logoFile.as_deref().filter(|l| !l.is_empty()))
                        .map(String::from);
                }
            }
        }
        if let Some(icon) = toml_icon {
            candidates.push(icon);
        }
        candidates.push("logo.png".to_string());
    }

    for name in candidates {
        if let Some(bytes) = read_zip_file(&mut archive, &name) {
            let lower = String::from_utf8_lossy(name.as_bytes()).to_lowercase();
            let kind = if lower.ends_with(".png") {
                "image/png"
            } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
                "image/jpeg"
            } else if lower.ends_with(".gif") {
                "image/gif"
            } else {
                "image/png"
            };
            return Some(format!("data:{kind};base64,{}", B64.encode(bytes)));
        }
    }
    None
}

fn read_zip_file(archive: &mut ZipArchive<std::fs::File>, name: &str) -> Option<Vec<u8>> {
    let target: PathBuf = PathBuf::from(name);
    let entry_name = archive
        .file_names()
        .find(|n| {
            let p = PathBuf::from(n);
            p == target || p.file_name() == target.file_name()
        })
        .map(String::from)?;
    let mut f = archive.by_name(&entry_name).ok()?;
    if !f.is_file() || f.size() > MAX_ICON_BYTES {
        return None;
    }
    let mut bytes = Vec::with_capacity(f.size() as usize);
    f.read_to_end(&mut bytes).ok()?;
    if bytes.is_empty() {
        return None;
    }
    Some(bytes)
}
