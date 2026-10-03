mod api;
mod importer;
mod notify;
mod server;
mod settings;
#[cfg(target_os = "linux")]
mod tray_ksni;
mod utils;

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::SystemTime;
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    pub process: server::process::ProcessManager,
    pub settings: tokio::sync::Mutex<settings::Settings>,
    pub system: Arc<tokio::sync::Mutex<sysinfo::System>>,
    pub players: server::process::PlayersCache,
    pub icons: tokio::sync::Mutex<std::collections::HashMap<String, String>>,
    pub tray: std::sync::Mutex<Option<tauri::tray::TrayIcon>>,
    pub quitting: Arc<AtomicBool>,
    /// Set once the tray icon is actually running. Window close only hides to
    /// the tray when this is true — otherwise the app would become unreachable.
    pub tray_ok: Arc<AtomicBool>,
    /// Whether the main window is currently visible. When it is hidden (trayed
    /// out) the live metrics polling is throttled — nobody is looking at it.
    pub ui_visible: Arc<AtomicBool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServerListItem {
    pub meta: server::manager::ServerMeta,
    pub status: server::process::StatusInfo,
    pub players: Option<i32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModInstallResult {
    pub filename: String,
    pub folder: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileItem {
    pub name: String,
    pub enabled: bool,
    pub size: u64,
    pub modified: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct VelocityCandidate {
    pub id: String,
    pub name: String,
    pub port: u16,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogInfo {
    pub path: String,
    pub lines: Vec<String>,
    pub total_lines: u64,
    pub truncated: bool,
}

// ---------------------------------------------------------------------------
// settings
// ---------------------------------------------------------------------------

/// Starts every server that opted into "launch with the app". Runs once,
/// shortly after startup, and is a no-op unless a server enabled the flag.
async fn boot_autostart(app: AppHandle) {
    let st = app.state::<AppState>();
    let settings = st.settings.lock().await.clone();
    for meta in server::manager::list_servers(&settings) {
        if !meta.auto_start_on_boot {
            continue;
        }
        match server::process::start(
            &app,
            &st.process,
            st.system.clone(),
            &settings,
            &meta,
            st.players.clone(),
        )
        .await
        {
            Ok(()) => eprintln!("[boot] auto-started {}", meta.id),
            Err(e) => eprintln!("[boot] auto-start {} failed: {e}", meta.id),
        }
    }
}

/// Evaluates every server's schedule rules against the current local time.
/// `fired` guards against a rule firing twice inside the same minute.
async fn scheduler_tick(
    app: AppHandle,
    now: &chrono::DateTime<chrono::Local>,
    fired: &mut std::collections::HashSet<String>,
) -> Result<(), String> {
    use chrono::Datelike;
    use chrono::Timelike;
    let st = app.state::<AppState>();
    let settings = st.settings.lock().await.clone();
    let today = now.weekday().number_from_monday(); // 1=mon .. 7=sun
    let today_name = match today {
        1 => "mon",
        2 => "tue",
        3 => "wed",
        4 => "thu",
        5 => "fri",
        6 => "sat",
        _ => "sun",
    };
    let (hour, minute) = (now.hour(), now.minute());

    for meta in server::manager::list_servers(&settings) {
        for (idx, rule) in meta.schedule.iter().enumerate() {
            if rule.hour != hour || rule.minute != minute {
                continue;
            }
            if rule.day != "every" && rule.day != today_name {
                continue;
            }
            let key = format!("{}:{}:{}", meta.id, idx, hour * 60 + minute);
            if !fired.insert(key) {
                continue;
            }
            let running = server::process::status(&st.process, &meta.id).await.state == "running";
            match rule.action.as_str() {
                "start" if !running => {
                    eprintln!("[scheduler] starting {}", meta.id);
                    if let Err(e) = server::process::start(
                        &app,
                        &st.process,
                        st.system.clone(),
                        &settings,
                        &meta,
                        st.players.clone(),
                    )
                    .await
                    {
                        eprintln!("[scheduler] start {} failed: {e}", meta.id);
                    }
                }
                "stop" if running => {
                    eprintln!("[scheduler] stopping {}", meta.id);
                    if let Err(e) = server::process::stop(&app, &st.process, &meta.id).await {
                        eprintln!("[scheduler] stop {} failed: {e}", meta.id);
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<settings::Settings, String> {
    Ok(state.settings.lock().await.clone())
}

#[tauri::command]
async fn set_settings(
    state: State<'_, AppState>,
    new_settings: settings::Settings,
) -> Result<settings::Settings, String> {
    let mut guard = state.settings.lock().await;
    *guard = new_settings.clone();
    guard.save()?;
    Ok(guard.clone())
}

#[tauri::command]
async fn detect_java() -> Result<Vec<server::java::JavaInstall>, String> {
    Ok(server::java::detect().await)
}

// ---------------------------------------------------------------------------
// servers
// ---------------------------------------------------------------------------

#[tauri::command]
async fn list_servers(state: State<'_, AppState>) -> Result<Vec<ServerListItem>, String> {
    let settings = state.settings.lock().await.clone();
    let metas = server::manager::list_servers(&settings);
    let mut out = Vec::new();
    for meta in metas {
        let status = server::process::status(&state.process, &meta.id).await;
        let players = if status.state == "running" {
            let online = state.players.lock().await.get(&meta.id).cloned();
            match online {
                Some((names, max)) => Some(names.len() as i32).filter(|_| max > 0),
                None => {
                    let log = server::process::console_tail(&state.process, &meta.id).await;
                    let (names, max) = server::players::parse_online(&log);
                    if max > 0 {
                        Some(names.len() as i32)
                    } else {
                        None
                    }
                }
            }
        } else {
            None
        };
        out.push(ServerListItem {
            meta,
            status,
            players,
        });
    }
    Ok(out)
}

#[tauri::command]
async fn get_server(
    state: State<'_, AppState>,
    id: String,
) -> Result<server::manager::ServerMeta, String> {
    let settings = state.settings.lock().await.clone();
    server::manager::find_server(&settings, &id)
}

#[tauri::command]
async fn server_status(
    state: State<'_, AppState>,
    id: String,
) -> Result<server::process::StatusInfo, String> {
    Ok(server::process::status(&state.process, &id).await)
}

#[tauri::command]
async fn start_server(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    server::process::start(
        &app,
        &state.process,
        state.system.clone(),
        &settings,
        &meta,
        state.players.clone(),
    )
    .await
}

#[tauri::command]
async fn stop_server(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    server::process::stop(&app, &state.process, &id).await?;
    let settings = state.settings.lock().await.clone();
    if let Ok(meta) = server::manager::find_server(&settings, &id) {
        notify::notify(&settings, &meta.name, notify::Event::Stop).await;
    }
    Ok(())
}

#[tauri::command]
async fn restart_server(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    server::process::stop(&app, &state.process, &id).await?;
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    server::process::start(
        &app,
        &state.process,
        state.system.clone(),
        &settings,
        &meta,
        state.players.clone(),
    )
    .await
}

#[tauri::command]
async fn kill_server(app: AppHandle, state: State<'_, AppState>, id: String) -> Result<(), String> {
    server::process::kill(&app, &state.process, &id).await
}

#[tauri::command]
async fn console_send(state: State<'_, AppState>, id: String, line: String) -> Result<(), String> {
    server::process::send_input(&state.process, &id, &line).await
}

#[tauri::command]
async fn console_tail(state: State<'_, AppState>, id: String) -> Result<Vec<String>, String> {
    Ok(server::process::console_tail(&state.process, &id).await)
}

#[tauri::command]
async fn console_clear(state: State<'_, AppState>, id: String) -> Result<(), String> {
    server::process::clear_console(&state.process, &id).await;
    Ok(())
}

#[tauri::command]
async fn server_logs(
    state: State<'_, AppState>,
    id: String,
    max_lines: usize,
) -> Result<LogInfo, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    let max = if max_lines == 0 {
        400
    } else {
        max_lines.min(5000)
    };
    if !folder.is_dir() {
        return Err("server folder not found".to_string());
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    let logs_dir = folder.join("logs");
    let latest = logs_dir.join("latest.log");
    if latest.is_file() {
        candidates.push(latest);
    }
    if let Ok(rd) = std::fs::read_dir(&logs_dir) {
        let mut files: Vec<(SystemTime, PathBuf)> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .filter(|e| e.path().extension().map(|x| x == "log").unwrap_or(false))
            .filter_map(|e| {
                e.metadata()
                    .ok()
                    .map(|m| (m.modified().unwrap_or(SystemTime::UNIX_EPOCH), e.path()))
            })
            .collect();
        files.sort_by_key(|(t, _)| *t);
        if let Some((_, newest)) = files.last().cloned() {
            candidates.push(newest);
        }
    }
    let root_log = folder.join("server.log");
    if root_log.is_file() {
        candidates.push(root_log);
    }

    let Some(path) = candidates.first() else {
        return Ok(LogInfo {
            path: String::new(),
            lines: Vec::new(),
            total_lines: 0,
            truncated: false,
        });
    };

    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => return Err(format!("failed to read {}: {e}", path.display())),
    };
    let all: Vec<&str> = content.lines().collect();
    let total = all.len();
    let take = total.saturating_sub(max);
    let truncated = take > 0;
    let lines = all[take..].iter().map(|s| s.to_string()).collect();
    Ok(LogInfo {
        path: path.to_string_lossy().into_owned(),
        lines,
        total_lines: total as u64,
        truncated,
    })
}

#[tauri::command]
async fn server_ping(state: State<'_, AppState>, id: String) -> Result<utils::ServerPing, String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let folder = settings.server_folder(&id);
    let props =
        utils::props::read_properties(&folder.join("server.properties")).unwrap_or_default();
    let port: u16 = props
        .get("server-port")
        .and_then(|p| p.parse().ok())
        .unwrap_or(if meta.is_proxy() { 25577 } else { 25565 });
    utils::ping::ping("127.0.0.1", port).await
}

#[tauri::command]
async fn create_server(
    app: AppHandle,
    state: State<'_, AppState>,
    req: server::manager::CreateServerRequest,
) -> Result<server::manager::ServerMeta, String> {
    let settings = state.settings.lock().await.clone();
    server::manager::create_server(&app, &settings, req).await
}

#[tauri::command]
async fn delete_server(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    let _ = server::process::stop(&app, &state.process, &id).await;
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    if folder.is_dir() {
        std::fs::remove_dir_all(&folder).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// versions / loaders
// ---------------------------------------------------------------------------

#[tauri::command]
async fn fetch_core_versions(core: String, include_snapshots: bool) -> Result<Vec<String>, String> {
    let versions = match core.as_str() {
        "paper" | "purpur" | "spigot" | "bukkit" => api::paper::versions("paper").await?,
        "vanilla" | "fabric" | "quilt" => api::vanilla::versions(include_snapshots).await?,
        "velocity" => api::paper::versions("velocity").await?,
        "forge" => api::forge::forge_minecraft_versions().await?,
        "neoforge" => api::forge::neoforge_minecraft_versions().await?,
        _ => return Err(format!("unknown core {core}")),
    };
    Ok(crate::utils::mcver::newest_first(versions))
}

#[tauri::command]
async fn fetch_loaders(core: String, mc: String) -> Result<Vec<String>, String> {
    match core.as_str() {
        "forge" => api::forge::forge_loaders(&mc).await,
        "neoforge" => {
            let all = api::forge::neoforge_versions().await?;
            Ok(api::forge::neoforge_loaders_for_mc(&mc, &all))
        }
        "fabric" => api::fabric::fabric_loaders_for(&mc).await,
        "quilt" => api::fabric::quilt_loaders_for(&mc).await,
        _ => Ok(Vec::new()),
    }
}

// ---------------------------------------------------------------------------
// server.properties
// ---------------------------------------------------------------------------

#[tauri::command]
async fn server_properties_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    server::properties::read(&folder.join("server.properties"))
}

#[tauri::command]
async fn server_properties_set(
    state: State<'_, AppState>,
    id: String,
    values: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    let path = folder.join("server.properties");
    server::properties::write(&path, &values)?;
    server::properties::read(&path)
}

// ---------------------------------------------------------------------------
// players
// ---------------------------------------------------------------------------

#[tauri::command]
async fn get_players(
    state: State<'_, AppState>,
    id: String,
) -> Result<server::players::PlayersView, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    let mut view = server::players::read_view(&folder);
    let cached = state.players.lock().await.get(&id).cloned();
    if let Some((names, max)) = cached {
        view.online = names;
        if max > 0 {
            view.max_online = max;
        }
        return Ok(view);
    }
    let log = server::process::console_tail(&state.process, &id).await;
    let (mut online, mut max) = server::players::parse_online(&log);
    if max == 0 {
        let (o, m) = server::players::read_online_from_logs(&folder);
        if m > 0 {
            online = o;
            max = m;
        }
    }
    view.online = online;
    if max > 0 {
        view.max_online = max;
    }
    Ok(view)
}

#[tauri::command]
async fn run_command(
    state: State<'_, AppState>,
    id: String,
    command: String,
) -> Result<(), String> {
    server::process::send_input(&state.process, &id, &command).await
}

// ---------------------------------------------------------------------------
// server icon
// ---------------------------------------------------------------------------

/// Returns the current `server-icon.png` as a `data:image/png;base64,` URL, if
/// present.
#[tauri::command]
async fn server_icon_get(state: State<'_, AppState>, id: String) -> Result<Option<String>, String> {
    let settings = state.settings.lock().await.clone();
    let path = settings.server_folder(&id).join("server-icon.png");
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let b64 = base64::encode(&bytes);
    Ok(Some(format!("data:image/png;base64,{b64}")))
}

/// Copies a chosen image file into `server-icon.png` (64×64 square PNG
/// recommended) and returns the new data URL.
#[tauri::command]
async fn server_icon_set(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Option<String>, String> {
    let src = Path::new(&path);
    if !src.is_file() {
        return Err("file not found".into());
    }
    let settings = state.settings.lock().await.clone();
    let dest = settings.server_folder(&id).join("server-icon.png");
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    let bytes = std::fs::read(&dest).map_err(|e| e.to_string())?;
    let b64 = base64::encode(&bytes);
    Ok(Some(format!("data:image/png;base64,{b64}")))
}

// ---------------------------------------------------------------------------
// mods & plugins
// ---------------------------------------------------------------------------

#[tauri::command]
async fn modrinth_search(
    query: String,
    core: String,
    mc: String,
    limit: i64,
) -> Result<Vec<api::modrinth::ModrinthProject>, String> {
    let (loader, proj_type) = api::modrinth::loader_for_core(&core);
    let loaders = if loader.is_empty() {
        Vec::new()
    } else {
        vec![loader.to_string()]
    };
    api::modrinth::search(&query, proj_type, &loaders, Some(&mc), limit).await
}

#[tauri::command]
async fn modrinth_icon(state: State<'_, AppState>, url: String) -> Result<Option<String>, String> {
    // WebKitGTK images can fail silently inside overlay dialogs; fetch through
    // the native client and embed as a data URL instead.
    {
        let cache = state.icons.lock().await;
        if let Some(hit) = cache.get(&url) {
            return Ok(Some(hit.clone()));
        }
    }
    let data = api::modrinth::icon_data_url(&url).await?;
    if let Some(data) = data.clone() {
        state.icons.lock().await.insert(url, data);
    }
    Ok(data)
}

#[tauri::command]
async fn modrinth_project_versions(
    project_id: String,
    core: String,
    mc: String,
) -> Result<Vec<api::modrinth::ModrinthVersion>, String> {
    let (loader, _) = api::modrinth::loader_for_core(&core);
    let loader = if loader.is_empty() {
        None
    } else {
        Some(loader)
    };
    api::modrinth::project_versions(&project_id, Some(&mc), loader).await
}

#[tauri::command]
async fn install_mod(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    project_id: String,
    mc: String,
) -> Result<ModInstallResult, String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let (loader, _) = api::modrinth::loader_for_core(&meta.core);
    let loader = if loader.is_empty() {
        None
    } else {
        Some(loader)
    };
    let versions = api::modrinth::project_versions(&project_id, Some(&mc), loader).await?;
    let ver = versions
        .into_iter()
        .next()
        .ok_or("no compatible version for this server")?;
    let file = ver
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| {
            ver.files.iter().find(|f| {
                f.env
                    .as_ref()
                    .map(|e| e.server.as_deref() != Some("unsupported"))
                    .unwrap_or(true)
            })
        })
        .or_else(|| ver.files.first())
        .ok_or("no downloadable file")?;
    let folder = meta.mod_folder().to_string();
    let filename = utils::paths::sane_filename(&file.filename);
    let dest = settings.server_folder(&id).join(&folder).join(&filename);
    let key = format!("server:{id}:mod:{}", filename);
    utils::download::download_file(&api::client(), &file.url, &dest, &app, &key).await?;
    if let Some(sha1) = file.hashes.get("sha1") {
        if let Ok(actual) = utils::download::file_sha1(&dest) {
            if !actual.eq_ignore_ascii_case(sha1) {
                return Err(format!("sha1 mismatch for {filename}"));
            }
        }
    }
    Ok(ModInstallResult {
        filename,
        folder,
        path: dest.to_string_lossy().to_string(),
    })
}

#[tauri::command]
async fn list_mods(state: State<'_, AppState>, id: String) -> Result<Vec<FileItem>, String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let folder = settings.server_folder(&id).join(meta.mod_folder());
    let mut items = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let lower = name.to_lowercase();
            if !lower.ends_with(".jar") && !lower.ends_with(".jar.disabled") {
                continue;
            }
            let enabled = !lower.ends_with(".disabled");
            let meta_md = path.metadata().ok();
            items.push(FileItem {
                name,
                enabled,
                size: meta_md.as_ref().map(|m| m.len()).unwrap_or(0),
                modified: meta_md
                    .and_then(|m| m.modified().ok())
                    .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                    .unwrap_or_default(),
            });
        }
    }
    items.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(items)
}

#[tauri::command]
async fn set_mod_enabled(
    state: State<'_, AppState>,
    id: String,
    filename: String,
    enabled: bool,
) -> Result<FileItem, String> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("invalid filename".into());
    }
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let folder = settings.server_folder(&id).join(meta.mod_folder());
    let target = if enabled {
        // enable: strip .disabled
        let base = filename.strip_suffix(".disabled").ok_or("bad filename")?;
        base.to_string()
    } else {
        format!("{filename}.disabled")
    };
    let src = folder.join(&filename);
    if !src.exists() {
        return Err("file not found".into());
    }
    std::fs::rename(&src, folder.join(&target)).map_err(|e| e.to_string())?;
    let meta_md = folder.join(&target).metadata().ok();
    Ok(FileItem {
        name: target,
        enabled,
        size: meta_md.as_ref().map(|m| m.len()).unwrap_or(0),
        modified: meta_md
            .and_then(|m| m.modified().ok())
            .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
            .unwrap_or_default(),
    })
}

#[tauri::command]
async fn install_local_mod(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let src = Path::new(&path);
    if !src.is_file() {
        return Err("file not found".into());
    }
    let filename = src
        .file_name()
        .map(|f| utils::paths::sane_filename(&f.to_string_lossy()))
        .unwrap_or_else(|| "mod.jar".to_string());
    let dest = settings
        .server_folder(&id)
        .join(meta.mod_folder())
        .join(&filename);
    std::fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::copy(src, &dest).map_err(|e| e.to_string())?;
    Ok(filename)
}

#[tauri::command]
async fn delete_mod(
    state: State<'_, AppState>,
    id: String,
    filename: String,
) -> Result<(), String> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("invalid filename".into());
    }
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let target = settings
        .server_folder(&id)
        .join(meta.mod_folder())
        .join(&filename);
    if !target.exists() {
        return Err("file not found".into());
    }
    std::fs::remove_file(&target).map_err(|e| e.to_string())
}

/// Extracts the mod's own icon from inside a JAR (fabric/quilt/forge) and
/// returns it as a base64 data URL. Returns None if the JAR has no icon.
#[tauri::command]
async fn mod_icon(
    state: State<'_, AppState>,
    id: String,
    name: String,
) -> Result<Option<String>, String> {
    if name.contains('/') || name.contains('\\') || name.contains("..") {
        return Err("invalid filename".into());
    }
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let target = settings
        .server_folder(&id)
        .join(meta.mod_folder())
        .join(&name);
    if !target.exists() {
        return Err("file not found".into());
    }
    let result = tauri::async_runtime::spawn_blocking(move || server::mod_icons::mod_icon(&target))
        .await
        .map_err(|e| e.to_string())?;
    Ok(result)
}

/// Re-downloads the latest compatible version of an installed mod/plugin from
/// Modrinth and replaces the old file. The project is located by searching for
/// the installed file's name.
#[tauri::command]
async fn update_mod(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    filename: String,
) -> Result<ModInstallResult, String> {
    if filename.contains('/') || filename.contains('\\') || filename.contains("..") {
        return Err("invalid filename".into());
    }
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let folder = settings.server_folder(&id).join(meta.mod_folder());
    let old = folder.join(&filename);
    if !old.exists() {
        return Err("file not found".into());
    }
    let (loader, _) = api::modrinth::loader_for_core(&meta.core);
    let loader_opt = if loader.is_empty() {
        None
    } else {
        Some(loader.to_string())
    };
    let mc = meta.mc_version.as_str();

    let query: String = filename
        .trim_end_matches(".jar")
        .trim_end_matches(".disabled")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| !p.is_empty() && p.chars().any(|c| c.is_alphabetic()))
        .collect::<Vec<_>>()
        .join("+");
    let projects = api::modrinth::search(&query, "mod", &[loader.to_string()], Some(mc), 3).await?;
    let project = projects
        .into_iter()
        .next()
        .ok_or("could not find a matching project on Modrinth")?;
    let versions =
        api::modrinth::project_versions(&project.project_id, Some(mc), loader_opt.as_deref())
            .await?;
    let ver = versions
        .into_iter()
        .next()
        .ok_or("no compatible version found")?;
    let file = ver
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| {
            ver.files.iter().find(|f| {
                f.env
                    .as_ref()
                    .map(|e| e.server.as_deref() != Some("unsupported"))
                    .unwrap_or(true)
            })
        })
        .or_else(|| ver.files.first())
        .ok_or("no downloadable file")?;
    let new_filename = utils::paths::sane_filename(&file.filename);
    let dest = folder.join(&new_filename);
    let key = format!("server:{id}:mod:{}", new_filename);
    utils::download::download_file(&api::client(), &file.url, &dest, &app, &key).await?;
    if let Some(sha1) = file.hashes.get("sha1") {
        if let Ok(actual) = utils::download::file_sha1(&dest) {
            if !actual.eq_ignore_ascii_case(sha1) {
                return Err(format!("sha1 mismatch for {new_filename}"));
            }
        }
    }
    if dest != old {
        std::fs::remove_file(&old).map_err(|e| e.to_string())?;
    }
    Ok(ModInstallResult {
        filename: new_filename,
        folder: meta.mod_folder().to_string(),
        path: dest.to_string_lossy().to_string(),
    })
}

// ---------------------------------------------------------------------------
// curseforge
// ---------------------------------------------------------------------------

#[tauri::command]
async fn curseforge_search(
    state: State<'_, AppState>,
    query: String,
    mc: String,
    core: String,
    limit: i64,
) -> Result<Vec<api::curseforge::CfMod>, String> {
    let settings = state.settings.lock().await.clone();
    let key = settings.curseforge_api_key.clone().unwrap_or_default();
    api::curseforge::search(&key, &query, &mc, &core, limit).await
}

/// Screenshot galleries for search results: Modrinth via the bulk projects
/// endpoint, CurseForge via the bulk /v1/mods endpoint.
#[tauri::command]
async fn mods_gallery(
    state: State<'_, AppState>,
    source: String,
    ids: Vec<String>,
) -> Result<std::collections::HashMap<String, Vec<String>>, String> {
    let mut out: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
    if source == "curseforge" {
        let settings = state.settings.lock().await.clone();
        let key = settings.curseforge_api_key.clone().unwrap_or_default();
        let mods = api::curseforge::mods_bulk(&key, &ids)
            .await
            .map_err(|e| e.to_string())?;
        for m in mods {
            let urls: Vec<String> = m
                .screenshots
                .iter()
                .filter(|s| !s.url.is_empty())
                .map(|s| s.url.clone())
                .take(8)
                .collect();
            out.insert(m.id.to_string(), urls);
        }
    } else {
        for p in api::modrinth::projects_bulk(&ids).await? {
            let urls: Vec<String> = p
                .gallery
                .iter()
                .map(|g| g.url.clone())
                .filter(|u| !u.is_empty())
                .take(8)
                .collect();
            out.insert(p.project_id.clone(), urls);
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// worlds
// ---------------------------------------------------------------------------

#[tauri::command]
async fn worlds_list(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<server::worlds::WorldInfo>, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    let props =
        utils::props::read_properties(&folder.join("server.properties")).unwrap_or_default();
    let level = props.get("level-name").map(|s| s.to_string());
    let folder2 = folder.clone();
    Ok(
        tokio::task::spawn_blocking(move || server::worlds::list(&folder2, level.as_deref()))
            .await
            .map_err(|e| e.to_string())?,
    )
}

#[tauri::command]
async fn worlds_backup(
    state: State<'_, AppState>,
    id: String,
    world: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::backup(&folder, &world))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_list_backups(
    state: State<'_, AppState>,
    id: String,
) -> Result<Vec<String>, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    Ok(server::worlds::list_backups(&folder))
}

#[tauri::command]
async fn worlds_restore(
    state: State<'_, AppState>,
    id: String,
    zipfile: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::restore(&folder, &zipfile))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_delete_backup(
    state: State<'_, AppState>,
    id: String,
    zipfile: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::delete_backup(&folder, &zipfile))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_rename_backup(
    state: State<'_, AppState>,
    id: String,
    from: String,
    to: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::rename_backup(&folder, &from, &to))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn server_backup_config_set(
    state: State<'_, AppState>,
    id: String,
    auto_backup_days: i64,
    auto_backup_keep: i64,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let mut meta = server::manager::find_server(&settings, &id)?;
    meta.auto_backup_days = auto_backup_days.max(0);
    meta.auto_backup_keep = auto_backup_keep.max(0);
    let folder = settings.server_folder(&id);
    server::manager::save_meta(&folder, &meta)
}

#[tauri::command]
async fn server_set_java(
    state: State<'_, AppState>,
    id: String,
    java_path: Option<String>,
) -> Result<server::manager::ServerMeta, String> {
    let settings = state.settings.lock().await.clone();
    let mut meta = server::manager::find_server(&settings, &id)?;
    meta.java_path = java_path.filter(|p| !p.trim().is_empty());
    let folder = settings.server_folder(&id);
    server::manager::save_meta(&folder, &meta)?;
    Ok(meta)
}

/// Per-server runtime configuration (JVM flags, auto-start, watchdog, schedule).
/// Everything is optional so the UI can PATCH a single section at a time.
#[derive(Debug, Clone, serde::Deserialize)]
struct ServerRuntimeConfig {
    #[serde(default)]
    jvm_preset: Option<String>,
    #[serde(default)]
    jvm_args: Option<String>,
    #[serde(default)]
    auto_start_on_boot: Option<bool>,
    #[serde(default)]
    auto_restart_on_crash: Option<bool>,
    #[serde(default)]
    schedule: Option<Vec<server::manager::ScheduleRule>>,
    #[serde(default)]
    min_ram: Option<i64>,
    #[serde(default)]
    max_ram: Option<i64>,
}

/// Updates any subset of a server's runtime configuration and returns the
/// fresh meta so the UI can patch its store without a full refetch.
#[tauri::command]
async fn server_update_config(
    state: State<'_, AppState>,
    id: String,
    config: ServerRuntimeConfig,
) -> Result<server::manager::ServerMeta, String> {
    let settings = state.settings.lock().await.clone();
    let mut meta = server::manager::find_server(&settings, &id)?;
    if let Some(p) = config.jvm_preset {
        meta.jvm_preset = p;
    }
    if let Some(a) = config.jvm_args {
        meta.jvm_args = a;
    }
    if let Some(v) = config.auto_start_on_boot {
        meta.auto_start_on_boot = v;
    }
    if let Some(v) = config.auto_restart_on_crash {
        meta.auto_restart_on_crash = v;
    }
    if let Some(s) = config.schedule {
        meta.schedule = s;
    }
    if let Some(v) = config.min_ram {
        meta.min_ram = v.max(128);
    }
    if let Some(v) = config.max_ram {
        meta.max_ram = v.max(meta.min_ram).max(256);
    }
    let folder = settings.server_folder(&id);
    server::manager::save_meta(&folder, &meta)?;
    Ok(meta)
}

/// Returns the exact java command line that will be used to launch a server.
#[tauri::command]
async fn server_launch_command(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let java = server::java::server_java(&settings, meta.java_path.as_deref(), meta.java).await?;
    let folder = settings.server_folder(&id);
    Ok(server::jvm::full_command(
        &java.to_string_lossy(),
        &meta,
        &folder,
    ))
}

/// Replaces a server's core with another Minecraft version. Refuses to run
/// while the server is up — a live world cannot be migrated under it.
#[tauri::command]
async fn server_change_version(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
    mc_version: String,
    loader_version: Option<String>,
) -> Result<server::manager::ServerMeta, String> {
    let settings = state.settings.lock().await.clone();
    let running = server::process::status(&state.process, &id).await.state == "running";
    if running {
        return Err("stop the server before changing its version".into());
    }
    let meta = server::manager::find_server(&settings, &id)?;
    server::manager::change_version(&app, &settings, &meta, &mc_version, &loader_version).await
}

/// Resolves the exact java executable a server will use (custom path or the
/// one picked for its required major version). Used to display it in the UI.
#[tauri::command]
async fn server_java_resolve(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    server::java::server_java(&settings, meta.java_path.as_deref(), meta.java)
        .await
        .map(|p| p.to_string_lossy().to_string())
}

/// Periodic auto-backup pass: creates backups for worlds whose newest archive
/// is older than the server's configured interval, and prunes backups past
/// their retention period. Runs hourly; the first tick fires at startup.
/// Servers that are currently running are skipped (a live world cannot be
/// zipped consistently).
async fn auto_backup_tick(app: AppHandle) -> Result<(), String> {
    let st = app.state::<AppState>();
    let settings = st.settings.lock().await.clone();
    for meta in server::manager::list_servers(&settings) {
        let folder = settings.server_folder(&meta.id);
        let level_name = server::players::read_prop(&folder, "level-name")
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| "world".to_string());

        if meta.auto_backup_days > 0 {
            let status = server::process::status(&st.process, &meta.id).await;
            if status.state == "running" {
                continue;
            }
            let worlds: Vec<String> = server::worlds::list(&folder, Some(&level_name))
                .into_iter()
                .map(|w| w.name)
                .collect();
            if worlds.is_empty() {
                continue;
            }
            let recent = server::worlds::newest_backup(&folder);
            let due = match recent {
                None => true,
                Some((_, mt)) => std::time::SystemTime::now()
                    .duration_since(mt)
                    .map(|age| age.as_secs() >= meta.auto_backup_days as u64 * 86_400)
                    .unwrap_or(false),
            };
            if due {
                let f = folder.clone();
                let created = tokio::task::spawn_blocking(move || {
                    let mut names = Vec::new();
                    for w in worlds {
                        names.push(server::worlds::backup(&f, &w)?);
                    }
                    Ok::<Vec<String>, String>(names)
                })
                .await
                .map_err(|e| e.to_string())??;
                eprintln!("[autobackup] {} created {:?}", meta.id, created);
                notify::notify(&settings, &meta.name, notify::Event::Backup).await;
            }
        }

        if meta.auto_backup_keep > 0 {
            let f = folder.clone();
            let removed = tokio::task::spawn_blocking(move || {
                server::worlds::delete_old_backups(&f, meta.auto_backup_keep)
            })
            .await
            .map_err(|e| e.to_string())??;
            if removed > 0 {
                eprintln!(
                    "[autobackup] {} removed {removed} expired backup(s)",
                    meta.id
                );
            }
        }
    }
    Ok(())
}

#[tauri::command]
async fn worlds_delete(
    state: State<'_, AppState>,
    id: String,
    world: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::delete(&folder, &world))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_datapacks(
    state: State<'_, AppState>,
    id: String,
    world: String,
) -> Result<Vec<server::worlds::DatapackInfo>, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::list_datapacks(&folder, &world))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_install_datapack(
    state: State<'_, AppState>,
    id: String,
    world: String,
    source: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::install_datapack(&folder, &world, &source))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_download_datapack(
    state: State<'_, AppState>,
    id: String,
    world: String,
    url: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    server::worlds::download_datapack(&folder, &world, &url).await
}

#[tauri::command]
async fn worlds_toggle_datapack(
    state: State<'_, AppState>,
    id: String,
    world: String,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || {
        server::worlds::toggle_datapack(&folder, &world, &name, enabled)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_delete_datapack(
    state: State<'_, AppState>,
    id: String,
    world: String,
    name: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::worlds::delete_datapack(&folder, &world, &name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_settings(
    state: State<'_, AppState>,
    id: String,
    world: String,
) -> Result<server::world_config::WorldSettings, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::world_config::read_settings(&folder, &world))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn worlds_install_datapack_modrinth(
    state: State<'_, AppState>,
    id: String,
    world: String,
    project_id: String,
    mc: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    let mc = if mc.is_empty() || mc == ".*" {
        None
    } else {
        Some(mc)
    };
    server::worlds::install_modrinth_datapack(&folder, &world, &project_id, mc.as_deref()).await
}

#[tauri::command]
async fn worlds_datapack_search(
    query: String,
    mc: String,
    limit: i64,
) -> Result<Vec<api::modrinth::ModrinthProject>, String> {
    let mc = if mc.is_empty() || mc == ".*" {
        None
    } else {
        Some(mc)
    };
    let loaders = vec!["datapack".to_string()];
    api::modrinth::search(&query, "datapack", &loaders, mc.as_deref(), limit).await
}

// ---------------------------------------------------------------------------
// import
// ---------------------------------------------------------------------------

#[tauri::command]
async fn import_pack(
    app: AppHandle,
    state: State<'_, AppState>,
    req: importer::ImportRequest,
) -> Result<server::manager::ServerMeta, String> {
    let settings = state.settings.lock().await.clone();
    importer::import(&app, &settings, req).await
}

// ---------------------------------------------------------------------------
// files
// ---------------------------------------------------------------------------

#[tauri::command]
async fn files_list(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<Vec<server::files::FileEntry>, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::list(&folder, &path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_read(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::read_text(&folder, &path))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_read_data_url(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || {
        server::files::read_data_url(&folder, &path, 12 * 1024 * 1024)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_upload(
    state: State<'_, AppState>,
    id: String,
    path: String,
    sources: Vec<String>,
) -> Result<usize, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::upload_many(&folder, &path, &sources))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_write(
    state: State<'_, AppState>,
    id: String,
    path: String,
    content: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::write_text(&folder, &path, &content))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_create(
    state: State<'_, AppState>,
    id: String,
    path: String,
    is_dir: bool,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::create_entry(&folder, &path, is_dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_delete(
    state: State<'_, AppState>,
    id: String,
    paths: Vec<String>,
) -> Result<usize, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::delete_many(&folder, &paths))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_rename(
    state: State<'_, AppState>,
    id: String,
    path: String,
    new_name: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::rename(&folder, &path, &new_name))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_copy_move(
    state: State<'_, AppState>,
    id: String,
    sources: Vec<String>,
    dest_dir: String,
    is_move: bool,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || {
        server::files::copy_move(&folder, &sources, &dest_dir, is_move)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_zip(
    state: State<'_, AppState>,
    id: String,
    sources: Vec<String>,
    base_dir: String,
    name: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || {
        server::files::make_archive(&folder, &sources, &base_dir, name)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn files_unzip(
    state: State<'_, AppState>,
    id: String,
    path: String,
    dest_dir: String,
) -> Result<(), String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    tokio::task::spawn_blocking(move || server::files::extract_archive(&folder, &path, &dest_dir))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn export_server(
    state: State<'_, AppState>,
    id: String,
) -> Result<server::export::ExportResult, String> {
    let settings = state.settings.lock().await.clone();
    let meta = server::manager::find_server(&settings, &id)?;
    let folder = settings.server_folder(&id);
    let servers_dir = settings.servers_dir();
    tokio::task::spawn_blocking(move || server::export::export_mrpack(&folder, &meta, &servers_dir))
        .await
        .map_err(|e| e.to_string())?
}

// ---------------------------------------------------------------------------
// velocity
// ---------------------------------------------------------------------------

#[tauri::command]
async fn velocity_get(
    state: State<'_, AppState>,
    id: String,
) -> Result<server::velocity::VelocityConfig, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    server::velocity::read(&folder)
}

#[tauri::command]
async fn velocity_save(
    state: State<'_, AppState>,
    id: String,
    cfg: server::velocity::VelocityConfig,
) -> Result<server::velocity::VelocityConfig, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    server::velocity::write(&folder, &cfg)?;
    Ok(cfg)
}

#[tauri::command]
async fn velocity_generate_secret(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let settings = state.settings.lock().await.clone();
    let folder = settings.server_folder(&id);
    let mut cfg = server::velocity::read(&folder)?;
    cfg.forwarding_secret = server::velocity::generate_secret();
    server::velocity::write(&folder, &cfg)?;
    Ok(cfg.forwarding_secret.clone())
}

#[tauri::command]
async fn velocity_candidates(state: State<'_, AppState>) -> Result<Vec<VelocityCandidate>, String> {
    let settings = state.settings.lock().await.clone();
    let servers = server::manager::list_servers(&settings);
    let mut out = Vec::new();
    for meta in servers {
        if meta.is_proxy() {
            continue;
        }
        let folder = settings.server_folder(&meta.id);
        let props =
            utils::props::read_properties(&folder.join("server.properties")).unwrap_or_default();
        let port: u16 = props
            .get("server-port")
            .and_then(|p| p.parse().ok())
            .unwrap_or(25565);
        out.push(VelocityCandidate {
            id: meta.id.clone(),
            name: meta.name.clone(),
            port,
            address: format!("127.0.0.1:{port}"),
        });
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// misc
// ---------------------------------------------------------------------------

#[tauri::command]
async fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    use tauri_plugin_shell::ShellExt;
    app.shell().open(&url, None).map_err(|e| e.to_string())
}

/// Display/DPI diagnostics for the frontend: the GTK scale factor plus logical
/// and physical window sizes.
///
/// Note both `outer_size` and `inner_size` report *physical* pixels, so their
/// ratio is always ~1.0 and must never be used to derive a scale factor — the
/// previous frontend zoom heuristic did exactly that and shrank the UI on every
/// HiDPI display. The real factor is `scale_factor()`, and the logical size is
/// obtained by dividing the physical size by it.
#[tauri::command]
fn ui_metrics(window: tauri::WebviewWindow) -> Result<serde_json::Value, String> {
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    let inner = window.inner_size().map_err(|e| e.to_string())?;
    let scale = if scale.is_finite() && scale > 0.0 { scale } else { 1.0 };
    Ok(serde_json::json!({
        "scale": scale,
        "physical": { "width": inner.width, "height": inner.height },
        "logical": {
            "width": inner.width as f64 / scale,
            "height": inner.height as f64 / scale,
        },
    }))
}

#[tauri::command]
fn ui_log(msg: String) {
    println!("[ui-log] {msg}");
}

/// Creates the system-tray icon: left-click (or "Show" menu item) restores the
/// window, "Quit" shuts every server down gracefully then exits. Closing the
/// window hides to the tray when enabled, so running servers survive it.
///
/// Platform split:
///  * Linux (Wayland/X11): a ksni StatusNotifierItem is registered directly.
///    libappindicator (used by tauri's tray) never receives the status-notifier
///    `Activate` signal, so left-click could not open the window. ksni gives us
///    real click hooks (`activate`/`secondary_activate`).
///  * Windows/macOS: the first-class tauri tray handles clicks natively.
fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "linux")]
    {
        return tray_ksni::setup(app);
    }
    #[cfg(not(target_os = "linux"))]
    {
        use tauri::menu::{MenuBuilder, MenuItemBuilder};
        use tauri::tray::{MouseButton, TrayIconBuilder, TrayIconEvent};

        let show = MenuItemBuilder::with_id("show", "Open MC Server Manager").build(app)?;
        let quit = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
        let menu = MenuBuilder::new(app).items(&[&show, &quit]).build()?;
        let icon = app
            .default_window_icon()
            .cloned()
            .ok_or("no default window icon")?;

        let tray = TrayIconBuilder::new()
            .icon(icon)
            .menu(&menu)
            .on_tray_icon_event(|tray, event| {
                if let TrayIconEvent::Click {
                    button: MouseButton::Left,
                    ..
                } = event
                {
                    show_main_window(tray.app_handle());
                }
            })
            .on_menu_event(|app, event| match event.id().as_ref() {
                "show" => show_main_window(app),
                "quit" => {
                    let st = app.state::<AppState>();
                    st.quitting.store(true, Ordering::Relaxed);
                    let pm = st.process.clone();
                    let _ = tauri::async_runtime::block_on(server::process::stop_all(app, &pm));
                    app.exit(0);
                }
                _ => {}
            })
            .build(app)?;
        app.state::<AppState>().tray.lock().unwrap().replace(tray);
        Ok(())
    }
}

fn show_main_window(app: &AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
        app.state::<AppState>().ui_visible.store(true, Ordering::Relaxed);
    }
}

/// Gracefully stops all running servers. Used on app exit (and before the
/// final quit) so servers shut down cleanly instead of being orphaned/killed.
fn shutdown_servers(app: &AppHandle) {
    let st = app.state::<AppState>();
    let pm = st.process.clone();
    let _ = tauri::async_runtime::block_on(server::process::stop_all(app, &pm));
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let state = AppState {
        process: server::process::ProcessManager::default(),
        settings: tokio::sync::Mutex::new(settings::Settings::load()),
        system: Arc::new(tokio::sync::Mutex::new(sysinfo::System::new_all())),
        players: Default::default(),
        icons: Default::default(),
        tray: Default::default(),
        quitting: Arc::new(AtomicBool::new(false)),
        tray_ok: Arc::new(AtomicBool::new(false)),
        ui_visible: Arc::new(AtomicBool::new(true)),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state)
        .setup(|app| {
            let st = app.state::<AppState>();
            if let Ok(guard) = st.settings.try_lock() {
                std::fs::create_dir_all(guard.servers_dir()).ok();
            }
            // The tray is best-effort: a missing status-notifier host must not
            // prevent the app (and its servers) from running.
            if setup_tray(app.handle()).is_ok() {
                st.tray_ok.store(true, Ordering::Relaxed);
            } else {
                eprintln!("[tray] disabled");
            }
            // Auto-backups: hourly sweep (first tick runs immediately, so an
            // overdue backup is created shortly after startup).
            let hook_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(std::time::Duration::from_secs(60 * 60));
                loop {
                    interval.tick().await;
                    if let Err(e) = auto_backup_tick(hook_app.clone()).await {
                        eprintln!("[autobackup] {e}");
                    }
                }
            });
            // Scheduler: fires each server's start/stop rules. The loop runs
            // once a minute and remembers which (server, minute, rule) triples
            // already fired so a slow tick cannot trigger a rule twice.
            let sched_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let mut fired: std::collections::HashSet<String> = std::collections::HashSet::new();
                let mut last_minute = String::new();
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(20)).await;
                    let now = chrono::Local::now();
                    let minute_key = now.format("%Y-%m-%dT%H:%M").to_string();
                    if minute_key != last_minute {
                        last_minute = minute_key;
                        fired.clear();
                    }
                    if let Err(e) = scheduler_tick(sched_app.clone(), &now, &mut fired).await {
                        eprintln!("[scheduler] {e}");
                    }
                }
            });
            // Auto-start on app launch (opt-in per server, off by default).
            let boot_app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                boot_autostart(boot_app.clone()).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            set_settings,
            detect_java,
            list_servers,
            get_server,
            server_status,
            start_server,
            stop_server,
            restart_server,
            kill_server,
            console_send,
            console_tail,
            console_clear,
            server_logs,
            server_ping,
            create_server,
            delete_server,
            fetch_core_versions,
            fetch_loaders,
            server_properties_get,
            server_properties_set,
            get_players,
            run_command,
            server_icon_get,
            server_icon_set,
            modrinth_search,
            modrinth_icon,
            modrinth_project_versions,
            install_mod,
            list_mods,
            set_mod_enabled,
            install_local_mod,
            delete_mod,
            update_mod,
            mod_icon,
            curseforge_search,
            mods_gallery,
            worlds_list,
            worlds_backup,
            worlds_list_backups,
            worlds_restore,
            worlds_delete_backup,
            worlds_rename_backup,
            server_backup_config_set,
            server_set_java,
            server_java_resolve,
            server_update_config,
            server_launch_command,
            server_change_version,
            worlds_delete,
            worlds_datapacks,
            worlds_install_datapack,
            worlds_download_datapack,
            worlds_toggle_datapack,
            worlds_delete_datapack,
            worlds_settings,
            worlds_install_datapack_modrinth,
            worlds_datapack_search,
            import_pack,
            export_server,
            files_list,
            files_read,
            files_read_data_url,
            files_upload,
            files_write,
            files_create,
            files_delete,
            files_rename,
            files_copy_move,
            files_zip,
            files_unzip,
            velocity_get,
            velocity_save,
            velocity_generate_secret,
            velocity_candidates,
            open_url,
            ui_metrics,
            ui_log,
        ])
        // Close-to-tray: the window close must be intercepted HERE (on the
        // window itself) with `prevent_close()` + `hide()`. Handling the close
        // at the app level via RunEvent::ExitRequested is too late — by then
        // the GTK window is already destroyed, and a later tray "Open" has
        // nothing to show.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let st = window.state::<AppState>();
                let tray_enabled = st
                    .settings
                    .try_lock()
                    .map(|s| s.tray_enabled)
                    .unwrap_or(true);
                let tray_ok = st.tray_ok.load(Ordering::Relaxed);
                let quitting = st.quitting.load(Ordering::Relaxed);
                if tray_enabled && tray_ok && !quitting {
                    api.prevent_close();
                    st.ui_visible.store(false, Ordering::Relaxed);
                    let _ = window.hide();
                }
            } else if let tauri::WindowEvent::Focused(true) = event {
                // Re-shown (tray "Open", taskbar, etc.): resume live metrics.
                let st = window.state::<AppState>();
                st.ui_visible.store(true, Ordering::Relaxed);
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app_handle, event| match event {
            tauri::RunEvent::ExitRequested { api, .. } => {
                let st = app_handle.state::<AppState>();
                if st.quitting.load(Ordering::Relaxed) {
                    // Tray "Quit": servers already stopped, let it exit.
                    return;
                }
                let tray_enabled = st
                    .settings
                    .try_lock()
                    .map(|s| s.tray_enabled)
                    .unwrap_or(true);
                let tray_ok = st.tray_ok.load(Ordering::Relaxed);
                if tray_enabled && tray_ok {
                    api.prevent_exit();
                    if let Some(w) = app_handle.get_webview_window("main") {
                        let _ = w.hide();
                    }
                    return;
                }
                // Tray disabled: closing the window quits the app, but the
                // servers must shut down gracefully first.
                shutdown_servers(app_handle);
            }
            tauri::RunEvent::Exit => {
                shutdown_servers(app_handle);
            }
            _ => {}
        });
}
