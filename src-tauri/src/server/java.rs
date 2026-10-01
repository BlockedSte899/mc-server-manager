use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::settings::Settings;

#[derive(Debug, Clone, Serialize)]
pub struct JavaInstall {
    pub major: u32,
    pub path: String,
    pub vendor: String,
    pub source: String,
}

fn bin_java() -> &'static str {
    if cfg!(windows) {
        "java.exe"
    } else {
        "java"
    }
}

/// Parses the major version from `java -version` stderr.
/// java 17 -> 17, java 1.8 -> 8.
pub fn parse_java_major(line: &str) -> Option<u32> {
    let start = line.find('"')? + 1;
    let rest = &line[start..];
    let end = rest.find('"')? + start;
    let ver = &line[start..end];
    let mut parts = ver.split('.');
    let first: u32 = parts.next()?.parse().ok()?;
    if first == 1 {
        parts.next()?.parse().ok()
    } else {
        Some(first)
    }
}

fn xdg_data_home() -> PathBuf {
    std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".local").join("share"))
                .unwrap_or_default()
        })
}

fn home_dir() -> PathBuf {
    std::env::var("HOME").map(PathBuf::from).unwrap_or_default()
}

/// Candidate java binaries paired with a human-readable origin ("JAVA_HOME",
/// "PATH", "Prism Launcher", …). Origins are shown in the Java settings,
/// which makes custom launcher runtimes easy to recognise.
fn java_candidates() -> Vec<(PathBuf, String)> {
    let mut out: Vec<(PathBuf, String)> = Vec::new();
    if let Ok(home) = std::env::var("JAVA_HOME") {
        out.push((
            PathBuf::from(home).join("bin").join(bin_java()),
            "JAVA_HOME".into(),
        ));
    }
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            if !dir.as_os_str().is_empty() {
                out.push((dir.join(bin_java()), "PATH".into()));
            }
        }
    }
    if let Ok(entries) = std::fs::read_dir("/usr/lib/jvm") {
        for e in entries.flatten() {
            out.push((e.path().join("bin").join(bin_java()), "system".into()));
        }
    }
    out.push((
        PathBuf::from("/run/current-system/sw/bin/java"),
        "system".into(),
    ));
    #[cfg(windows)]
    windows_registry_candidates(&mut out);
    launcher_candidates(&mut out);
    nix_store_candidates(&mut out);

    let mut seen = std::collections::HashSet::new();
    out.into_iter()
        .filter(|(p, _)| {
            let key = std::fs::canonicalize(p)
                .map(|c| c.to_string_lossy().to_string())
                .unwrap_or_else(|_| p.to_string_lossy().to_string());
            seen.insert(key)
        })
        .collect()
}

/// Java runtimes bundled with launchers (Prism Launcher, MultiMC). They keep
/// their own portable JVMs; each runtime lives in `<launcher data>/java/<name>`
/// and instances can pin a custom `JavaPath=` in `<launcher data>/instances/<i>/instance.cfg`.
fn launcher_candidates(out: &mut Vec<(PathBuf, String)>) {
    let mut launcher_dirs: Vec<(PathBuf, String)> = Vec::new();
    #[cfg(not(windows))]
    {
        let data = xdg_data_home();
        launcher_dirs.push((data.join("PrismLauncher"), "Prism Launcher".into()));
        launcher_dirs.push((data.join("multiMC"), "MultiMC".into()));
        launcher_dirs.push((
            home_dir().join(".var/app/org.prismlauncher.PrismLauncher/data/PrismLauncher"),
            "Prism Launcher (Flatpak)".into(),
        ));
        // PrismLauncher ships a flatpak-styled data dir for AppImage too.
        launcher_dirs.push((
            home_dir().join(".local/share/PrismLauncher"),
            "Prism Launcher".into(),
        ));
        launcher_dirs.push((
            home_dir().join("Library/Application Support/PrismLauncher"),
            "Prism Launcher (macOS)".into(),
        ));
    }
    #[cfg(windows)]
    {
        let appdata = std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_default();
        launcher_dirs.push((appdata.join("PrismLauncher"), "Prism Launcher".into()));
        launcher_dirs.push((appdata.join("multiMC"), "MultiMC".into()));
    }

    for (base, label) in &launcher_dirs {
        // bundled runtimes: <base>/java/<name>/bin/java
        let java_dir = base.join("java");
        if let Ok(entries) = std::fs::read_dir(&java_dir) {
            for e in entries.flatten() {
                out.push((e.path().join("bin").join(bin_java()), label.clone()));
            }
        }
        // per-instance pinned java: <base>/instances/<name>/instance.cfg → JavaPath=
        let instances = base.join("instances");
        if let Ok(entries) = std::fs::read_dir(&instances) {
            for e in entries.flatten() {
                let cfg = e.path().join("instance.cfg");
                let Ok(text) = std::fs::read_to_string(&cfg) else {
                    continue;
                };
                for line in text.lines() {
                    if let Some((k, v)) = line.split_once('=') {
                        if k.trim() == "JavaPath" && !v.trim().is_empty() {
                            let p = PathBuf::from(v.trim());
                            // Bare names like "java" resolve through PATH, which is
                            // already scanned separately; never probe cwd-relative paths.
                            if !p.is_absolute() {
                                continue;
                            }
                            out.push((p.clone(), label.clone()));
                            if p.is_dir() {
                                out.push((p.join("bin").join(bin_java()), label.clone()));
                            }
                        }
                    }
                }
            }
        }
        // launcher remembers its own java choice in the top-level config:
        // <base>/prismlauncher.cfg  →  JavaPath=<executable>
        let cfg_paths = ["prismlauncher.cfg", "multimc.cfg"];
        for name in cfg_paths {
            let cfg = base.join(name);
            let Ok(text) = std::fs::read_to_string(&cfg) else {
                continue;
            };
            for line in text.lines() {
                if let Some((k, v)) = line.split_once('=') {
                    if k.trim() == "JavaPath" && !v.trim().is_empty() {
                        let p = PathBuf::from(v.trim());
                        if p.is_absolute() {
                            out.push((p, label.clone()));
                        }
                    }
                }
            }
        }
    }
}

/// NixOS / nix store hosts keep self-contained JVMs under /nix/store. They are
/// usually not on PATH (only the active profile's java is), so a scan that only
/// follows PATH/JAVA_HOME silently misses them. Cheap readdir + name filter.
fn nix_store_candidates(out: &mut Vec<(PathBuf, String)>) {
    let Ok(entries) = std::fs::read_dir("/nix/store") else {
        return;
    };
    let markers = [
        "openjdk", "openjre", "temurin", "zulu", "corretto", "liberica", "graalvm",
    ];
    for e in entries.flatten() {
        let Ok(meta) = e.metadata() else { continue };
        if !meta.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().to_lowercase();
        if !markers.iter().any(|m| name.contains(m)) {
            continue;
        }
        let java = e.path().join("bin").join(bin_java());
        if java.exists() {
            out.push((java, "nix store".into()));
        }
    }
}

#[cfg(windows)]
fn windows_registry_candidates(list: &mut Vec<(PathBuf, String)>) {
    use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};
    use winreg::RegKey;
    let roots = [
        "SOFTWARE\\JavaSoft\\JDK",
        "SOFTWARE\\JavaSoft\\Java Development Kit",
        "SOFTWARE\\JavaSoft\\JRE",
        "SOFTWARE\\WOW6432Node\\JavaSoft\\JDK",
        "SOFTWARE\\WOW6432Node\\JavaSoft\\Java Development Kit",
    ];
    for root in roots {
        if let Ok(key) = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey_with_flags(root, KEY_READ) {
            for (name, _) in key.enum_keys().flatten() {
                if let Ok(sub) = key.open_subkey_with_flags(name, KEY_READ) {
                    if let Ok(home) = sub.get_value::<String, _>("JavaHome") {
                        list.push((
                            PathBuf::from(home).join("bin").join("java.exe"),
                            "registry".into(),
                        ));
                    }
                }
            }
        }
    }
}

async fn probe_version(path: &Path) -> Option<(u32, String)> {
    if !path.exists() {
        return None;
    }
    let out = tokio::process::Command::new(path)
        .arg("-version")
        .output()
        .await
        .ok()?;
    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
    let major = parse_java_major(&stderr)?;
    let vendor = if stderr.contains("Temurin") {
        "Eclipse Temurin".to_string()
    } else if stderr.contains("Zulu") {
        "Azul Zulu".to_string()
    } else if stderr.contains("Microsoft") {
        "Microsoft".to_string()
    } else if stderr.contains("OpenJDK") || stderr.contains("openjdk") {
        "OpenJDK".to_string()
    } else {
        "Unknown".to_string()
    };
    Some((major, vendor))
}

/// Returns all detected Java installations, deduplicated, sorted by major.
pub async fn detect() -> Vec<JavaInstall> {
    let mut found: Vec<JavaInstall> = Vec::new();
    for (path, source) in java_candidates() {
        if let Some((major, vendor)) = probe_version(&path).await {
            if found.iter().any(|j| j.path == path.to_string_lossy()) {
                continue;
            }
            found.push(JavaInstall {
                major,
                path: path.to_string_lossy().to_string(),
                vendor,
                source,
            });
        }
    }
    found.sort_by_key(|j| j.major);
    found
}

/// Resolves a java executable for a given major version.
/// Prefers user override, then scans the system once.
pub async fn resolve_java(settings: &Settings, major: u32) -> Result<PathBuf, String> {
    if let Some(p) = settings.java_overrides.get(&major) {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
        return Err(format!(
            "Java {major} override path does not exist: {}",
            p.display()
        ));
    }
    for (path, _) in java_candidates() {
        if let Some((m, _)) = probe_version(&path).await {
            if m == major {
                return Ok(path);
            }
        }
    }
    let available = detect().await;
    let list: Vec<String> = available.iter().map(|j| j.major.to_string()).collect();
    Err(format!(
        "No Java {major} found on the system (available: {}). Install it or set a custom path in Settings.",
        if list.is_empty() {
            "none".to_string()
        } else {
            list.join(", ")
        }
    ))
}

/// Resolves the java for a server: an explicit `java_path` wins, otherwise
/// falls back to resolving the server's required major version.
pub async fn server_java(
    settings: &Settings,
    java_path: Option<&str>,
    major: u32,
) -> Result<PathBuf, String> {
    if let Some(p) = java_path {
        let p = PathBuf::from(p);
        if p.exists() {
            return Ok(p);
        }
        return Err(format!("Custom Java path does not exist: {}", p.display()));
    }
    resolve_java(settings, major).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_major_modern() {
        assert_eq!(
            parse_java_major("openjdk version \"17.0.10\" 2024-01-16"),
            Some(17)
        );
        assert_eq!(parse_java_major("\"21.0.2\" 2024-01-16"), Some(21));
    }

    #[test]
    fn parse_major_java8() {
        assert_eq!(parse_java_major("openjdk version \"1.8.0_392\""), Some(8));
    }

    #[test]
    fn parse_major_garbage() {
        assert_eq!(parse_java_major("not a java line"), None);
        assert_eq!(parse_java_major(""), None);
    }

    #[tokio::test]
    async fn server_java_custom_path_wins() {
        let settings = Settings::default();
        let tmp = std::env::temp_dir()
            .join("mcsm-java-test")
            .join(format!("inst{}", std::process::id()));
        std::fs::create_dir_all(&tmp).unwrap();
        let f = tmp.join("java");
        std::fs::write(&f, "#!/bin/sh\necho 17\n").unwrap();
        let path = f.to_string_lossy().to_string();
        let got = server_java(&settings, Some(&path), 21).await;
        assert_eq!(got.unwrap().to_string_lossy(), path);
        std::fs::remove_dir_all(tmp.parent().unwrap().join("mcsm-java-test")).ok();
    }

    #[tokio::test]
    async fn server_java_missing_custom_path_errors() {
        let settings = Settings::default();
        let got = server_java(&settings, Some("/nonexistent/definitely/missing/java"), 21).await;
        assert!(got.is_err());
    }
}
