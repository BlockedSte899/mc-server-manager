use std::path::Path;

/// Picks the most likely runnable server jar in a directory,
/// preferring ones that look like the core (forge/neoforge/fabric/quilt/nuke).
pub fn detect_server_jar(dir: &Path, core: &str) -> Option<String> {
    let keywords: Vec<&str> = match core {
        "forge" => vec!["forge"],
        "neoforge" => vec!["neoforge"],
        "fabric" => vec!["fabric"],
        "quilt" => vec!["quilt"],
        "vanilla" => vec!["minecraft_server", "server"],
        _ => vec![core],
    };
    let mut jars: Vec<(std::time::SystemTime, String)> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "jar").unwrap_or(false) {
                let name = p
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                let lower = name.to_lowercase();
                if lower.contains("installer") || lower.contains("-sources") {
                    continue;
                }
                if keywords.is_empty() {
                    continue;
                }
                if !keywords.iter().any(|k| lower.contains(k)) {
                    continue;
                }
                let modified = p
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::UNIX_EPOCH);
                jars.push((modified, name));
            }
        }
    }
    jars.sort_by(|a, b| b.0.cmp(&a.0));
    jars.into_iter().map(|(_, n)| n).next()
}

/// Runs a Fabric server installer (`fabric-installer.jar server ...`).
/// Returns the produced server jar name (usually `fabric-server-launch.jar`).
pub async fn install_fabric(
    java: &Path,
    server_dir: &Path,
    installer: &Path,
    mc: &str,
    loader: &str,
) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg("-jar")
        .arg(installer)
        .arg("server")
        .arg("-mcversion")
        .arg(mc)
        .arg("-loader")
        .arg(loader)
        .arg("-dir")
        .arg(server_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let status = tokio::time::timeout(std::time::Duration::from_secs(900), cmd.status())
        .await
        .map_err(|_| "fabric installer timed out".to_string())?
        .map_err(|e| format!("failed to launch fabric installer: {e}"))?;
    if !status.success() {
        return Err(format!("fabric installer exited with {status}"));
    }
    let _ = std::fs::remove_file(installer);
    Ok("fabric-server-launch.jar".to_string())
}

/// Runs a Quilt server installer.
pub async fn install_quilt(
    java: &Path,
    server_dir: &Path,
    installer: &Path,
    mc: &str,
    loader: &str,
) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new(java);
    cmd.arg("-jar")
        .arg(installer)
        .arg("install")
        .arg("server")
        .arg(mc)
        .arg("--loader")
        .arg(loader)
        .arg("--install-dir")
        .arg(server_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let status = tokio::time::timeout(std::time::Duration::from_secs(900), cmd.status())
        .await
        .map_err(|_| "quilt installer timed out".to_string())?
        .map_err(|e| format!("failed to launch quilt installer: {e}"))?;
    if !status.success() {
        return Err(format!("quilt installer exited with {status}"));
    }
    let _ = std::fs::remove_file(installer);
    Ok("quilt-server-launch.jar".to_string())
}
