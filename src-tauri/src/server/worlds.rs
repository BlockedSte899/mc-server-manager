use serde::Serialize;
use std::path::Path;
use zip::write::SimpleFileOptions;

use crate::utils::paths::sane_filename;

#[derive(Debug, Clone, Serialize)]
pub struct WorldInfo {
    pub name: String,
    pub size_bytes: u64,
    pub modified: String,
    pub backup: bool,
}

pub const BACKUP_DIR: &str = "backups";
pub const PACK_DIR: &str = "server-pack";

fn dir_size(path: &Path) -> u64 {
    let mut total = 0u64;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(m) = p.metadata() {
                total += m.len();
            }
        }
    }
    total
}

fn is_world_dir(path: &Path) -> bool {
    path.join("level.dat").exists()
        || path.join("level.dat_old").exists()
        || path.join("region").is_dir()
        || (path.join("DIM-1").is_dir() && path.join("DIM1").is_dir())
}

/// Lists worlds: any dir containing level.dat, plus the configured level-name if missing.
pub fn list(server_dir: &Path, level_name: Option<&str>) -> Vec<WorldInfo> {
    let mut out = Vec::new();
    let mut found: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(server_dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if !p.is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') || name == BACKUP_DIR || name == PACK_DIR {
                continue;
            }
            if is_world_dir(&p) {
                found.push(name.clone());
                let modified = p
                    .metadata()
                    .and_then(|m| m.modified())
                    .ok()
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                    .unwrap_or_default();
                out.push(WorldInfo {
                    backup: backup_exists(server_dir, &name),
                    name,
                    size_bytes: dir_size(&p),
                    modified,
                });
            }
        }
    }
    if let Some(lvl) = level_name {
        if !lvl.is_empty() && !found.iter().any(|f| f == lvl) {
            let p = server_dir.join(lvl);
            if p.is_dir() {
                out.push(WorldInfo {
                    name: lvl.to_string(),
                    size_bytes: dir_size(&p),
                    modified: String::new(),
                    backup: backup_exists(server_dir, lvl),
                });
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

pub fn backup_dir_name(name: &str) -> String {
    format!(
        "{}-{}.zip",
        sane_filename(name),
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    )
}

pub fn backup_dir(server_dir: &Path) -> std::path::PathBuf {
    server_dir.join(BACKUP_DIR)
}

fn backup_exists(server_dir: &Path, world: &str) -> bool {
    let dir = backup_dir(server_dir);
    let sane_world = sane_filename(world);
    let prefix = format!("{sane_world}-");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if (name.starts_with(&prefix) || name.starts_with(&sane_world))
                && name.ends_with(".zip")
            {
                return true;
            }
        }
    }
    false
}

pub fn list_backups(server_dir: &Path) -> Vec<String> {
    let dir = backup_dir(server_dir);
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".zip") {
                out.push(name);
            }
        }
    }
    out.sort();
    out
}

/// Zips a world folder into backups/. For bukkit-style layouts the separated
/// `<world>_nether` and `<world>_the_end` folders are included as well.
pub fn backup(server_dir: &Path, world: &str) -> Result<String, String> {
    let src = server_dir.join(world);
    if !src.is_dir() {
        return Err(format!("world '{world}' not found"));
    }
    std::fs::create_dir_all(backup_dir(server_dir)).map_err(|e| e.to_string())?;
    let filename = backup_dir_name(world);
    let dest = backup_dir(server_dir).join(&filename);
    let file = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);
    add_dir_to_zip(&mut zip, &src, &src, &options)?;
    for suffix in ["_nether", "_the_end"] {
        let extra = server_dir.join(format!("{world}{suffix}"));
        if extra.is_dir() {
            add_dir_to_zip(&mut zip, server_dir, &extra, &options)?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(filename)
}

fn add_dir_to_zip(
    zip: &mut zip::ZipWriter<std::fs::File>,
    base: &Path,
    dir: &Path,
    options: &SimpleFileOptions,
) -> Result<(), String> {
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
        let name = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            zip.add_directory(format!("{name}/"), *options)
                .map_err(|e| e.to_string())?;
            add_dir_to_zip(zip, base, &path, options)?;
        } else {
            zip.start_file(&name, *options).map_err(|e| e.to_string())?;
            let mut f = std::fs::File::open(&path).map_err(|e| e.to_string())?;
            std::io::copy(&mut f, zip).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Restores a backup (root-level dir under backups/{zip}) into the server dir.
pub fn restore(server_dir: &Path, zipfile: &str) -> Result<(), String> {
    let src = backup_dir(server_dir).join(zipfile);
    let file = std::fs::File::open(&src).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    // safe extraction: strip any traversal
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let mut rel = entry
            .enclosed_name()
            .ok_or("unsafe path in archive")?
            .to_path_buf();
        if rel.as_os_str().is_empty() {
            continue;
        }
        let dest = server_dir.join(&rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = std::fs::File::create(&dest).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut out).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Deletes a world folder (and its separated nether/end folders for bukkit
/// layouts, plus matching backups).
pub fn delete(server_dir: &Path, world: &str) -> Result<(), String> {
    if world.is_empty() || world == "." || world == ".." {
        return Err("invalid world name".into());
    }
    let mut removed_any = false;
    for suffix in ["", "_nether", "_the_end"] {
        let target = server_dir.join(format!("{world}{suffix}"));
        if target.is_dir() {
            std::fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
            removed_any = true;
        }
    }
    if !removed_any {
        return Err(format!("world '{world}' not found"));
    }
    delete_backups_for(server_dir, world).ok();
    Ok(())
}

fn valid_backup_name(name: &str) -> Result<String, String> {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains("..")
        || !name.ends_with(".zip")
    {
        return Err("invalid backup name".into());
    }
    Ok(name.to_string())
}

/// Removes a single backup archive (validated against path traversal).
pub fn delete_backup(server_dir: &Path, zipfile: &str) -> Result<(), String> {
    let zipfile = valid_backup_name(zipfile)?;
    let target = backup_dir(server_dir).join(&zipfile);
    if !target.is_file() {
        return Err("backup not found".into());
    }
    std::fs::remove_file(&target).map_err(|e| e.to_string())
}

/// Renames a backup archive (e.g. to a human-friendly label). Appends the
/// `.zip` extension when omitted and refuses to overwrite an existing backup.
pub fn rename_backup(server_dir: &Path, from: &str, to: &str) -> Result<String, String> {
    let from = valid_backup_name(from)?;
    let mut new_name = to.trim().to_string();
    if new_name.is_empty() {
        return Err("new name cannot be empty".into());
    }
    if !new_name.to_lowercase().ends_with(".zip") {
        new_name.push_str(".zip");
    }
    let new_name = valid_backup_name(&new_name)?;
    if new_name == from {
        return Ok(from);
    }
    let src = backup_dir(server_dir).join(&from);
    let dest = backup_dir(server_dir).join(&new_name);
    if !src.is_file() {
        return Err("backup not found".into());
    }
    if dest.exists() {
        return Err("a backup with that name already exists".into());
    }
    std::fs::rename(&src, &dest).map_err(|e| e.to_string())?;
    Ok(new_name)
}

/// The most recently created backup (name + modification time), if any.
pub fn newest_backup(server_dir: &Path) -> Option<(String, std::time::SystemTime)> {
    let dir = backup_dir(server_dir);
    let mut best: Option<(String, std::time::SystemTime)> = None;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".zip") {
                continue;
            }
            if let Ok(mt) = e.metadata().and_then(|m| m.modified()) {
                if best.as_ref().map(|(_, t)| *t < mt).unwrap_or(true) {
                    best = Some((name, mt));
                }
            }
        }
    }
    best
}

/// Deletes every backup older than `keep_days` days. Returns how many were
/// removed. Backups whose modification time cannot be read are left alone.
pub fn delete_old_backups(server_dir: &Path, keep_days: i64) -> Result<usize, String> {
    if keep_days <= 0 {
        return Ok(0);
    }
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(keep_days as u64 * 86_400))
        .ok_or("invalid retention period")?;
    let dir = backup_dir(server_dir);
    let mut removed = 0usize;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if !name.ends_with(".zip") {
                continue;
            }
            if let Ok(mt) = e.metadata().and_then(|m| m.modified()) {
                if mt < cutoff && std::fs::remove_file(e.path()).is_ok() {
                    removed += 1;
                }
            }
        }
    }
    Ok(removed)
}

fn delete_backups_for(server_dir: &Path, world: &str) -> Result<(), String> {
    let dir = backup_dir(server_dir);
    let sane_world = sane_filename(world);
    let prefixes = [
        format!("{sane_world}-"),
        format!("{sane_world}_nether-"),
        format!("{sane_world}_the_end-"),
    ];
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".zip") && prefixes.iter().any(|p| name.starts_with(p.as_str())) {
                let _ = std::fs::remove_file(e.path());
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// datapacks
// ---------------------------------------------------------------------------

const DISABLED_DIR: &str = ".disabled";

fn valid_world(server_dir: &Path, world: &str) -> Result<std::path::PathBuf, String> {
    if world.is_empty()
        || world == "."
        || world == ".."
        || world.contains('/')
        || world.contains('\\')
    {
        return Err("invalid world name".into());
    }
    let dir = server_dir.join(world);
    if !dir.is_dir() {
        return Err(format!("world '{world}' not found"));
    }
    Ok(dir)
}

fn valid_pack(name: &str) -> Result<String, String> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err("invalid datapack name".into());
    }
    Ok(name.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct DatapackInfo {
    pub name: String,
    pub enabled: bool,
    pub kind: String, // "zip" | "dir"
}

/// Lists datapacks of a world. Enabled ones live directly in `<world>/datapacks`,
/// disabled ones are moved into `<world>/datapacks/.disabled`.
pub fn list_datapacks(server_dir: &Path, world: &str) -> Result<Vec<DatapackInfo>, String> {
    let dir = valid_world(server_dir, world)?.join("datapacks");
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let p = e.path();
            let kind = if p.is_dir() { "dir" } else { "zip" };
            out.push(DatapackInfo {
                name,
                enabled: true,
                kind: kind.into(),
            });
        }
    }
    let disabled = dir.join(DISABLED_DIR);
    if let Ok(entries) = std::fs::read_dir(&disabled) {
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let p = e.path();
            let kind = if p.is_dir() { "dir" } else { "zip" };
            out.push(DatapackInfo {
                name,
                enabled: false,
                kind: kind.into(),
            });
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

/// Installs a datapack zip/folder from a local path into the world's enabled
/// datapacks. Colliding names get ` (N)` appended.
pub fn install_datapack(server_dir: &Path, world: &str, source: &str) -> Result<String, String> {
    let dir = valid_world(server_dir, world)?.join("datapacks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let src = std::path::Path::new(source);
    if !src.exists() {
        return Err("source path not found".into());
    }
    let base = src
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("datapack")
        .to_string();
    let dest = unique_pack_dest(&dir, &base);
    if src.is_dir() {
        copy_dir(src, &dest).map_err(|e| e.to_string())?;
    } else {
        std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    }
    Ok(dest
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(&base)
        .to_string())
}

/// Downloads a datapack zip from a URL into the world's enabled datapacks.
pub async fn download_datapack(
    server_dir: &Path,
    world: &str,
    url: &str,
) -> Result<String, String> {
    let dir = valid_world(server_dir, world)?.join("datapacks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let filename = url
        .split('/')
        .last()
        .filter(|f| !f.is_empty())
        .unwrap_or("datapack.zip")
        .to_string();
    if filename.contains('?') || filename.contains('#') {
        return Err("unsupported URL filename".into());
    }
    let dest = unique_pack_dest(&dir, &filename);
    let bytes = crate::api::client()
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    Ok(dest
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(&filename)
        .to_string())
}

/// Downloads a datapack zip from Modrinth, choosing the newest version
/// compatible with the given game version (falling back to the newest overall).
pub async fn install_modrinth_datapack(
    server_dir: &Path,
    world: &str,
    project_id: &str,
    game_version: Option<&str>,
) -> Result<String, String> {
    let dir = valid_world(server_dir, world)?.join("datapacks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let versions =
        crate::api::modrinth::project_versions(project_id, game_version, Some("datapack")).await?;
    let ver = versions
        .into_iter()
        .next()
        .ok_or("no datapack version compatible with this server")?;
    let file = ver
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| ver.files.first())
        .ok_or("no downloadable file")?;
    let filename = crate::utils::paths::sane_filename(&file.filename);
    let dest = unique_pack_dest(&dir, &filename);
    let bytes = crate::api::client()
        .get(&file.url)
        .send()
        .await
        .map_err(|e| format!("download failed: {e}"))?
        .error_for_status()
        .map_err(|e| format!("download failed: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("download failed: {e}"))?;
    std::fs::write(&dest, &bytes).map_err(|e| e.to_string())?;
    Ok(dest
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(&filename)
        .to_string())
}

fn unique_pack_dest(dir: &Path, base: &str) -> std::path::PathBuf {
    let mut name = base.to_string();
    let mut i = 1;
    while dir.join(&name).exists() {
        let stem = std::path::Path::new(base)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("datapack");
        let ext = std::path::Path::new(base)
            .extension()
            .and_then(|s| s.to_str())
            .map(|e| format!(".{e}"))
            .unwrap_or_default();
        name = format!("{stem} ({i}){ext}");
        i += 1;
    }
    dir.join(name)
}

/// Toggles a datapack between enabled (in `datapacks/`) and disabled
/// (moved into `datapacks/.disabled/`).
pub fn toggle_datapack(
    server_dir: &Path,
    world: &str,
    name: &str,
    enabled: bool,
) -> Result<(), String> {
    let name = valid_pack(name)?;
    let packs = valid_world(server_dir, world)?.join("datapacks");
    let src = if enabled {
        packs.join(DISABLED_DIR).join(&name)
    } else {
        packs.join(&name)
    };
    if !src.exists() {
        return Err(format!("datapack '{name}' not found"));
    }
    let dest = if enabled {
        packs.join(&name)
    } else {
        packs.join(DISABLED_DIR).join(&name)
    };
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&src, &dest).map_err(|e| e.to_string())?;
    Ok(())
}

/// Deletes a datapack (zip or folder), enabled or disabled.
pub fn delete_datapack(server_dir: &Path, world: &str, name: &str) -> Result<(), String> {
    let name = valid_pack(name)?;
    let packs = valid_world(server_dir, world)?.join("datapacks");
    for candidate in [packs.join(&name), packs.join(DISABLED_DIR).join(&name)] {
        if candidate.is_dir() {
            std::fs::remove_dir_all(&candidate).map_err(|e| e.to_string())?;
            return Ok(());
        } else if candidate.is_file() {
            std::fs::remove_file(&candidate).map_err(|e| e.to_string())?;
            return Ok(());
        }
    }
    Err(format!("datapack '{name}' not found"))
}

fn copy_dir(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let to = dest.join(entry.file_name());
        if entry.path().is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{Duration, SystemTime};

    fn temp_server() -> PathBuf {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static N: AtomicUsize = AtomicUsize::new(0);
        let base = std::env::temp_dir().join(format!(
            "mcsm-test-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        let dir = base.join("server");
        std::fs::create_dir_all(dir.join("backups")).expect("create backup dir");
        dir
    }

    fn set_mtime(path: &Path, age: Duration) {
        let t = SystemTime::now().checked_sub(age).unwrap();
        std::fs::File::open(path)
            .and_then(|f| f.set_modified(t))
            .expect("set mtime");
    }

    #[test]
    fn rename_backup_basics() {
        let server = temp_server();
        std::fs::write(server.join("backups/world-20240101-000000.zip"), b"zip").unwrap();

        // appends the missing .zip extension
        let n = rename_backup(&server, "world-20240101-000000.zip", "before launch").unwrap();
        assert_eq!(n, "before launch.zip");
        assert!(server.join("backups/before launch.zip").is_file());
        assert!(!server.join("backups/world-20240101-000000.zip").exists());

        // rename full name; identity rename is a no-op
        assert_eq!(
            rename_backup(&server, "before launch.zip", "final.zip").unwrap(),
            "final.zip"
        );
        assert_eq!(
            rename_backup(&server, "final.zip", "final.zip").unwrap(),
            "final.zip"
        );

        // refuses to overwrite, empty, and traversal names
        std::fs::write(server.join("backups/taken.zip"), b"x").unwrap();
        assert!(rename_backup(&server, "final.zip", "taken.zip").is_err());
        assert!(rename_backup(&server, "final.zip", "   ").is_err());
        assert!(rename_backup(&server, "final.zip", "../evil.zip").is_err());
        assert!(rename_backup(&server, "../final.zip", "ok.zip").is_err());

        let _ = std::fs::remove_dir_all(server.parent().unwrap());
    }

    #[test]
    fn delete_old_backups_respects_retention() {
        let server = temp_server();
        let dir = server.join("backups");
        let old = dir.join("old.zip");
        let fresh = dir.join("fresh.zip");
        std::fs::write(&old, b"x").unwrap();
        std::fs::write(&fresh, b"x").unwrap();
        set_mtime(&old, Duration::from_secs(8 * 86_400));
        set_mtime(&fresh, Duration::from_secs(1 * 86_400));

        // only the expired one is removed
        assert_eq!(delete_old_backups(&server, 7).unwrap(), 1);
        assert!(!old.exists());
        assert!(fresh.exists());

        // keep-forever (0) never deletes
        assert_eq!(delete_old_backups(&server, 0).unwrap(), 0);
        assert!(fresh.exists());

        let _ = std::fs::remove_dir_all(server.parent().unwrap());
    }

    #[test]
    fn newest_backup_picks_latest() {
        let server = temp_server();
        let dir = server.join("backups");
        let a = dir.join("a.zip");
        let b = dir.join("b.zip");
        std::fs::write(&a, b"x").unwrap();
        std::fs::write(&b, b"x").unwrap();
        set_mtime(&a, Duration::from_secs(9 * 86_400));
        set_mtime(&b, Duration::from_secs(2 * 86_400));

        let (name, _) = newest_backup(&server).unwrap();
        assert_eq!(name, "b.zip");
        assert!(newest_backup(&server.join("does-not-exist")).is_none());

        let _ = std::fs::remove_dir_all(server.parent().unwrap());
    }
}
