use serde::Serialize;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
    pub modified: String,
}

/// Resolves a relative POSIX path safely against `root`. The result is
/// guaranteed to stay inside `root` (canonicalized check).
pub fn resolve(root: &Path, rel: &str) -> Result<PathBuf, String> {
    let base = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut joined = base.clone();
    if !rel.is_empty() {
        for comp in rel.split('/') {
            if comp.is_empty() || comp == "." {
                continue;
            }
            if comp == ".." {
                if !joined.pop() {
                    return Err("illegal path (escapes server folder)".into());
                }
                continue;
            }
            if comp.contains('\\') || comp.starts_with('~') {
                return Err("illegal path component".into());
            }
            joined.push(comp);
        }
    }
    let canon = joined.canonicalize().unwrap_or_else(|_| joined.clone());
    if !canon.starts_with(&base) {
        return Err("path escapes server folder".into());
    }
    Ok(canon)
}

pub fn rel_display(base: &Path, target: &Path) -> String {
    target
        .strip_prefix(base)
        .map(|r| {
            let s = r.to_string_lossy().replace('\\', "/");
            if s.is_empty() {
                String::new()
            } else {
                s
            }
        })
        .unwrap_or_else(|_| target.to_string_lossy().into_owned())
}

fn modified_str(m: Option<SystemTime>) -> String {
    m.map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
        .unwrap_or_default()
}

pub fn list(root: &Path, rel: &str) -> Result<Vec<FileEntry>, String> {
    let dir = resolve(root, rel)?;
    if !dir.is_dir() {
        return Err("not a directory".into());
    }
    let mut items: Vec<FileEntry> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let md = entry.metadata().ok();
            let is_dir = md.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            items.push(FileEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                is_dir,
                size: md.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: modified_str(md.as_ref().and_then(|m| m.modified().ok())),
            });
        }
    }
    items.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(items)
}

const MAX_READ_BYTES: u64 = 2 * 1024 * 1024;

pub fn read_text(root: &Path, rel: &str) -> Result<String, String> {
    let path = resolve(root, rel)?;
    if !path.is_file() {
        return Err("not a file".into());
    }
    let len = path.metadata().map_err(|e| e.to_string())?.len();
    if len > MAX_READ_BYTES {
        return Err("file is too large to open in the editor".into());
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Reads a (binary) file and returns it as a base64 data URL, used for the
/// image preview in the file manager.
pub fn read_data_url(root: &Path, rel: &str, max_bytes: u64) -> Result<String, String> {
    let path = resolve(root, rel)?;
    if !path.is_file() {
        return Err("not a file".into());
    }
    let len = path.metadata().map_err(|e| e.to_string())?.len();
    if len > max_bytes {
        return Err("file is too large to preview".into());
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("")
        .to_lowercase();
    let kind = if name.ends_with(".png") {
        "image/png"
    } else if name.ends_with(".jpg") || name.ends_with(".jpeg") {
        "image/jpeg"
    } else if name.ends_with(".gif") {
        "image/gif"
    } else if name.ends_with(".webp") {
        "image/webp"
    } else if name.ends_with(".bmp") {
        "image/bmp"
    } else if name.ends_with(".svg") {
        "image/svg+xml"
    } else if name.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    };
    use base64::engine::general_purpose::STANDARD as B64;
    use base64::Engine;
    Ok(format!("data:{kind};base64,{}", B64.encode(bytes)))
}

pub fn write_text(root: &Path, rel: &str, content: &str) -> Result<(), String> {
    let path = resolve(root, rel)?;
    let parent = path.parent().ok_or("invalid path")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn create_entry(root: &Path, rel: &str, is_dir: bool) -> Result<(), String> {
    let path = resolve(root, rel)?;
    if is_dir {
        std::fs::create_dir_all(&path).map_err(|e| e.to_string())
    } else {
        if path.exists() {
            return Err("file already exists".into());
        }
        let parent = path.parent().ok_or("invalid path")?;
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        File::create(&path).map_err(|e| e.to_string()).map(|_| ())
    }
}

pub fn delete_many(root: &Path, rels: &[String]) -> Result<usize, String> {
    let mut removed = 0usize;
    for rel in rels {
        let path = resolve(root, rel)?;
        if path.is_dir() {
            std::fs::remove_dir_all(&path).map_err(|e| e.to_string())?;
        } else {
            std::fs::remove_file(&path).map_err(|e| e.to_string())?;
        }
        removed += 1;
    }
    Ok(removed)
}

pub fn rename(root: &Path, rel: &str, new_name: &str) -> Result<(), String> {
    if new_name.contains('/') || new_name.contains('\\') || new_name.is_empty() {
        return Err("invalid name".into());
    }
    let src = resolve(root, rel)?;
    let parent = src.parent().ok_or("invalid path")?;
    let dest = parent.join(new_name);
    if dest.exists() {
        return Err("a file with that name already exists".into());
    }
    std::fs::rename(&src, &dest).map_err(|e| e.to_string())
}

pub fn copy_move(
    root: &Path,
    rels: &[String],
    dest_dir: &str,
    is_move: bool,
) -> Result<(), String> {
    let dest = resolve(root, dest_dir)?;
    if !dest.is_dir() {
        return Err("destination is not a directory".into());
    }
    for rel in rels {
        let src = resolve(root, rel)?;
        let name = src
            .file_name()
            .ok_or("invalid name")?
            .to_string_lossy()
            .into_owned();
        let mut target = dest.join(&name);
        if target == src {
            continue;
        }
        let base_name = name.clone();
        let mut n = 1u32;
        while target.exists() {
            let stem = Path::new(&base_name)
                .file_stem()
                .map(|s| s.to_string_lossy())
                .unwrap_or_default()
                .to_string();
            let ext = Path::new(&base_name)
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default();
            target = dest.join(format!("{stem} ({n}){ext}"));
            n += 1;
        }
        if is_move {
            std::fs::rename(&src, &target).map_err(|e| e.to_string())?;
        } else {
            copy_recursive(&src, &target).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn copy_recursive(src: &Path, dest: &Path) -> std::io::Result<()> {
    let md = src.metadata()?;
    if md.is_dir() {
        std::fs::create_dir_all(dest)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            copy_recursive(&entry.path(), &dest.join(entry.file_name()))?;
        }
        Ok(())
    } else {
        std::fs::create_dir_all(dest.parent().unwrap_or(dest))?;
        std::fs::copy(src, dest)?;
        Ok(())
    }
}

/// Copies source files (absolute paths, e.g. from a drag & drop or dialog)
/// into `root`/`rel`, renaming on collision.
pub fn upload_many(root: &Path, rel: &str, sources: &[String]) -> Result<usize, String> {
    let dest_dir = resolve(root, rel)?;
    if !dest_dir.is_dir() {
        return Err("destination is not a directory".into());
    }
    let mut n = 0usize;
    for src_s in sources {
        let src = Path::new(src_s);
        if !src.is_file() {
            continue;
        }
        let name = src
            .file_name()
            .map(|f| crate::utils::paths::sane_filename(&f.to_string_lossy()))
            .unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let mut target = dest_dir.join(&name);
        let stem = Path::new(&name)
            .file_stem()
            .map(|s| s.to_string_lossy())
            .unwrap_or_default()
            .to_string();
        let ext = Path::new(&name)
            .extension()
            .map(|e| format!(".{}", e.to_string_lossy()))
            .unwrap_or_default();
        let mut i = 1u32;
        while target.exists() {
            target = dest_dir.join(format!("{stem} ({i}){ext}"));
            i += 1;
        }
        std::fs::copy(&src, &target).map_err(|e| e.to_string())?;
        n += 1;
    }
    Ok(n)
}

/// Creates a zip archive `<base>/<name>.zip` containing the selected files
/// (relative to `root`), paths stored POSIX-style.
pub fn make_archive(
    root: &Path,
    rels: &[String],
    base_dir: &str,
    name: String,
) -> Result<String, String> {
    let name = if name.trim().is_empty() {
        "archive".to_string()
    } else {
        name.trim().to_string()
    };
    let name = format!("{}.zip", name.trim_end_matches(".zip"));
    let base = resolve(root, base_dir)?;
    if !base.is_dir() {
        return Err("target folder is not a directory".into());
    }
    let dest = base.join(&name);
    if dest.exists() {
        return Err("archive already exists".into());
    }
    let file = File::create(&dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for rel in rels {
        let src = resolve(root, rel)?;
        if src.is_dir() {
            collect_dir(&mut zip, &options, &src, root, true)?;
        } else {
            add_entry(&mut zip, &options, &src, root)?;
        }
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(dest.to_string_lossy().into_owned())
}

fn collect_dir(
    zip: &mut zip::ZipWriter<File>,
    options: &zip::write::SimpleFileOptions,
    dir: &Path,
    root: &Path,
    top: bool,
) -> Result<(), String> {
    if top {
        zip.add_directory(format!("{}/", rel_display(root, dir)), *options)
            .map_err(|e| e.to_string())?;
    }
    for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())?.flatten() {
        let path = entry.path();
        let md = entry.metadata().map_err(|e| e.to_string())?;
        if md.is_dir() {
            collect_dir(zip, options, &path, root, false)?;
        } else {
            add_entry(zip, options, &path, root)?;
        }
    }
    Ok(())
}

fn add_entry(
    zip: &mut zip::ZipWriter<File>,
    options: &zip::write::SimpleFileOptions,
    path: &Path,
    root: &Path,
) -> Result<(), String> {
    let name = rel_display(root, path);
    zip.start_file(name, *options).map_err(|e| e.to_string())?;
    let mut f = File::open(path).map_err(|e| e.to_string())?;
    std::io::copy(&mut f, zip).map_err(|e| e.to_string())?;
    Ok(())
}

/// Extracts a zip archive into `root`/`rel` dir. Entry names are sanitized so
/// nothing escapes the server folder.
pub fn extract_archive(root: &Path, rel: &str, dest_dir: &str) -> Result<(), String> {
    let zip_path = resolve(root, rel)?;
    if !zip_path.is_file() {
        return Err("not a file".into());
    }
    let dest = resolve(root, dest_dir)?;
    std::fs::create_dir_all(&dest).map_err(|e| e.to_string())?;
    let file = File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let raw = entry.name().to_string();
        if raw.ends_with('/') {
            continue;
        }
        let cleaned = raw.replace('\\', "/");
        let relative: Vec<&str> = cleaned
            .split('/')
            .filter(|c| !c.is_empty() && *c != "." && *c != "..")
            .collect();
        if relative.is_empty() {
            continue;
        }
        let target = resolve(&dest, &relative.join("/"))?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let mut out = File::create(&target).map_err(|e| e.to_string())?;
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf).map_err(|e| e.to_string())?;
        out.write_all(&buf).map_err(|e| e.to_string())?;
    }
    Ok(())
}
