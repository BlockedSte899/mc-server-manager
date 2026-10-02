use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::api;
use crate::settings::Settings;
use crate::utils::download::download_file;
use crate::utils::paths::{now_iso, sane_filename};
use crate::utils::slug::{slugify, unique_folder};
use tauri::{AppHandle, Emitter};

pub const META_FILE: &str = "server.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMeta {
    pub name: String,
    pub id: String,
    pub java: u32,
    pub core: String,
    pub mc_version: String,
    pub loader_version: Option<String>,
    pub build: Option<String>,
    pub jar: String,
    pub min_ram: i64,
    pub max_ram: i64,
    pub created_at: String,
    pub is_proxy: bool,
    pub source_url: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    /// Auto-backup interval in days for all worlds of this server (0 = off).
    #[serde(default)]
    pub auto_backup_days: i64,
    /// Delete backups older than this many days (0 = keep forever).
    #[serde(default)]
    pub auto_backup_keep: i64,
    /// Explicit java executable for this server; falls back to [`ServerMeta::java`] when None.
    #[serde(default)]
    pub java_path: Option<String>,
    /// JVM flags preset for this server: "auto", "aikar", "vanilla", "custom".
    #[serde(default)]
    pub jvm_preset: String,
    /// Custom JVM arguments joined by spaces (when preset == "custom").
    #[serde(default)]
    pub jvm_args: String,
    /// Start this server automatically when the app launches (off by default).
    #[serde(default)]
    pub auto_start_on_boot: bool,
    /// Restart this server automatically if it dies unexpectedly (off by default).
    #[serde(default)]
    pub auto_restart_on_crash: bool,
    /// Time-based start/stop rules.
    #[serde(default)]
    pub schedule: Vec<ScheduleRule>,
}

/// A single "do X at HH:MM on <day>" rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleRule {
    /// Weekday short name (mon..sun) or "every".
    pub day: String,
    pub hour: u32,
    pub minute: u32,
    /// "start" or "stop".
    pub action: String,
}

impl ServerMeta {
    pub fn is_proxy(&self) -> bool {
        self.is_proxy || self.core == "velocity"
    }

    pub fn mod_folder(&self) -> &'static str {
        match self.core.as_str() {
            "paper" | "purpur" | "spigot" | "bukkit" | "velocity" => "plugins",
            _ => "mods",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateServerRequest {
    pub name: String,
    pub java: u32,
    #[serde(default)]
    pub java_path: Option<String>,
    pub core: String,
    pub mc_version: String,
    pub loader_version: Option<String>,
    pub min_ram: i64,
    pub max_ram: i64,
    pub accept_eula: bool,
    #[serde(default)]
    pub jvm_preset: Option<String>,
    #[serde(default)]
    pub jvm_args: Option<String>,
}

pub fn meta_path(dir: &Path) -> PathBuf {
    dir.join(META_FILE)
}

pub fn load_meta(dir: &Path) -> Option<ServerMeta> {
    let text = std::fs::read_to_string(meta_path(dir)).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_meta(dir: &Path, meta: &ServerMeta) -> Result<(), String> {
    let json = serde_json::to_string_pretty(meta).map_err(|e| e.to_string())?;
    std::fs::write(meta_path(dir), json).map_err(|e| e.to_string())
}

pub fn list_servers(settings: &Settings) -> Vec<ServerMeta> {
    let dir = settings.servers_dir();
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if entry.path().is_dir() {
                if let Some(meta) = load_meta(&entry.path()) {
                    out.push(meta);
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

pub fn find_server(settings: &Settings, id: &str) -> Result<ServerMeta, String> {
    let dir = settings.server_folder(id);
    load_meta(&dir).ok_or_else(|| format!("Server '{id}' not found"))
}

pub fn default_server_properties(name: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert(
        "motd".to_string(),
        if name.is_empty() {
            "A Minecraft Server".to_string()
        } else {
            name.to_string()
        },
    );
    map.insert("server-port".to_string(), "25565".to_string());
    map.insert("online-mode".to_string(), "true".to_string());
    map.insert("level-name".to_string(), "world".to_string());
    map.insert("level-seed".to_string(), String::new());
    map.insert("gamemode".to_string(), "survival".to_string());
    map.insert("difficulty".to_string(), "easy".to_string());
    map.insert("max-players".to_string(), "20".to_string());
    map.insert("view-distance".to_string(), "10".to_string());
    map.insert("spawn-protection".to_string(), "16".to_string());
    map.insert("pvp".to_string(), "true".to_string());
    map.insert("allow-nether".to_string(), "true".to_string());
    map.insert("enable-command-block".to_string(), "false".to_string());
    map.insert("white-list".to_string(), "false".to_string());
    map.insert(
        "network-compression-threshold".to_string(),
        "256".to_string(),
    );
    map.insert("max-tick-time".to_string(), "60000".to_string());
    map.insert("require-resource-pack".to_string(), "false".to_string());
    map.insert("enable-query".to_string(), "false".to_string());
    map.insert("query.port".to_string(), "25565".to_string());
    map.insert("enable-rcon".to_string(), "false".to_string());
    map.insert("rcon.port".to_string(), "25575".to_string());
    map.insert("rcon.password".to_string(), String::new());
    map.insert("enable-status".to_string(), "true".to_string());
    map
}

/// Core download resolution.
enum Resolution {
    /// Straight jar download.
    Direct { url: String, filename: String },
    /// Download an installer then run it into the server dir.
    Installer { url: String, filename: String },
}

async fn resolve_core(
    _client: &reqwest::Client,
    core: &str,
    mc: &str,
    loader: &Option<String>,
) -> Result<Resolution, String> {
    match core {
        "paper" => {
            let dl = api::paper::latest_build("paper", mc).await?;
            Ok(Resolution::Direct {
                url: dl.url,
                filename: dl.name,
            })
        }
        "velocity" => {
            let versions = api::paper::versions("velocity").await?;
            let ver = versions.first().ok_or("no velocity version")?;
            let dl = api::paper::latest_build("velocity", ver).await?;
            Ok(Resolution::Direct {
                url: dl.url,
                filename: dl.name,
            })
        }
        "vanilla" => {
            let url = api::vanilla::server_url(mc).await?;
            Ok(Resolution::Direct {
                url,
                filename: "server.jar".to_string(),
            })
        }
        "purpur" => {
            let build = api::purpur::latest_build(mc).await?;
            Ok(Resolution::Direct {
                url: api::purpur::download_url(mc, build),
                filename: format!("purpur-{mc}-{build}.jar"),
            })
        }
        "spigot" => Ok(Resolution::Direct {
            url: api::bukkit::spigot_url(mc),
            filename: format!("spigot-{mc}.jar"),
        }),
        "bukkit" => Ok(Resolution::Direct {
            url: api::bukkit::craftbukkit_url(mc),
            filename: format!("craftbukkit-{mc}.jar"),
        }),
        "forge" => {
            let loader = loader.clone().ok_or("forge loader version required")?;
            Ok(Resolution::Installer {
                url: api::forge::installer_url(mc, &loader),
                filename: format!("forge-{mc}-{loader}-installer.jar"),
            })
        }
        "neoforge" => {
            let loader = loader.clone().ok_or("neoforge loader version required")?;
            Ok(Resolution::Installer {
                url: api::forge::neoforge_installer_url(&loader),
                filename: format!("neoforge-{loader}-installer.jar"),
            })
        }
        "fabric" => {
            if loader.is_none() {
                return Err("fabric loader version required".into());
            }
            let iv = api::fabric::fabric_installer_latest().await?;
            Ok(Resolution::Installer {
                url: api::fabric::fabric_installer_url(&iv),
                filename: format!("fabric-installer-{iv}.jar"),
            })
        }
        "quilt" => {
            if loader.is_none() {
                return Err("quilt loader version required".into());
            }
            let iv = api::fabric::quilt_installer_latest().await?;
            Ok(Resolution::Installer {
                url: api::fabric::quilt_installer_url(&iv),
                filename: format!("quilt-installer-{iv}.jar"),
            })
        }
        other => Err(format!("unsupported core: {other}")),
    }
}

async fn emit_stage(app: &AppHandle, id: &str, stage: &str) {
    use serde::Serialize;
    #[derive(Clone, Serialize)]
    struct Stage {
        id: String,
        stage: String,
    }
    let _ = app.emit(
        "server-stage",
        Stage {
            id: id.to_string(),
            stage: stage.to_string(),
        },
    );
}

/// Runs the Forge/NeoForge installer for a directory and returns the server jar name.
pub async fn install_forge(
    app: &AppHandle,
    java: &Path,
    server_dir: &Path,
    core: &str,
    mc: &str,
    loader: &str,
    installer_filename: &str,
) -> Result<Option<String>, String> {
    let installer = server_dir.join(installer_filename);
    if !installer.exists() {
        return Err("installer.jar missing".into());
    }
    emit_stage(app, "install", "installing-loader").await;
    let mut cmd = tokio::process::Command::new(java);
    crate::utils::console::hide_console(&mut cmd);
    cmd.arg("-jar")
        .arg(&installer)
        .arg("--installServer")
        .current_dir(server_dir)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    let status = tokio::time::timeout(std::time::Duration::from_secs(900), cmd.status())
        .await
        .map_err(|_| format!("{core} installer timed out"))?
        .map_err(|e| format!("failed to launch {core} installer: {e}"))?;
    if !status.success() {
        return Err(format!("{core} installer exited with {status}"));
    }
    let _ = std::fs::remove_file(&installer);
    Ok(crate::server::installers::detect_server_jar(
        server_dir, core,
    ))
}

/// Swaps a server to a different Minecraft version (and loader build when the
/// core needs one) in place. The world, configs and plugins/mods are left
/// untouched; only the server jar is replaced and the metadata updated.
pub async fn change_version(
    app: &AppHandle,
    settings: &Settings,
    meta: &ServerMeta,
    mc_version: &str,
    loader_version: &Option<String>,
) -> Result<ServerMeta, String> {
    let client = api::client();
    let core = meta.core.to_lowercase();
    let mc = mc_version.trim().to_string();
    if mc.is_empty() {
        return Err("a Minecraft version is required".into());
    }
    if mc == meta.mc_version && core != "velocity" {
        if let (Some(new), Some(old)) = (loader_version.as_ref(), meta.loader_version.as_ref()) {
            if new == old {
                return Err("the server is already on that version".into());
            }
        } else {
            return Err("the server is already on that version".into());
        }
    }

    let dir = settings.server_folder(&meta.id);
    let old_jar = meta.jar.clone();
    let stage_key = format!("server:{}:core", meta.id);
    emit_stage(app, &meta.id, "downloading-core").await;
    let resolution = resolve_core(&client, &core, &mc, loader_version).await?;

    let mut updated = meta.clone();
    let new_jar = match resolution {
        Resolution::Direct { url, filename } => {
            let jar = sane_filename(&filename);
            download_file(&client, &url, &dir.join(&jar), app, &stage_key).await?;
            jar
        }
        Resolution::Installer { url, filename } => {
            download_file(&client, &url, &dir.join(&filename), app, &stage_key).await?;
            let java =
                crate::server::java::server_java(settings, meta.java_path.as_deref(), meta.java)
                    .await?;
            let loader = loader_version.as_deref().unwrap_or_default();
            let j = match core.as_str() {
                "forge" | "neoforge" => {
                    install_forge(app, &java, &dir, &core, &mc, loader, &filename).await?
                }
                "fabric" => Some(
                    crate::server::installers::install_fabric(
                        &java,
                        &dir,
                        &dir.join(&filename),
                        &mc,
                        loader,
                    )
                    .await?,
                ),
                "quilt" => Some(
                    crate::server::installers::install_quilt(
                        &java,
                        &dir,
                        &dir.join(&filename),
                        &mc,
                        loader,
                    )
                    .await?,
                ),
                _ => None,
            };
            j.ok_or("unable to locate installed server jar")?
        }
    };

    // Only drop the previous jar once the new one is safely on disk.
    if !old_jar.is_empty() && old_jar != new_jar {
        let old_path = dir.join(&old_jar);
        if old_path.is_file() {
            let _ = std::fs::remove_file(old_path);
        }
    }

    updated.mc_version = mc;
    updated.loader_version = loader_version.clone();
    updated.jar = new_jar;
    updated.build = None;
    save_meta(&dir, &updated)?;
    emit_stage(app, &meta.id, "done").await;
    Ok(updated)
}

/// Creates a server folder, downloads the core, writes eula/properties/meta.
pub async fn create_server(
    app: &AppHandle,
    settings: &Settings,
    req: CreateServerRequest,
) -> Result<ServerMeta, String> {
    if !req.accept_eula {
        return Err("You must accept the Minecraft EULA first".into());
    }
    if req.name.trim().is_empty() {
        return Err("Server name is required".into());
    }

    let client = api::client();
    let core = req.core.to_lowercase();
    let mc = req.mc_version.trim().to_string();

    let (id, dir) = unique_folder(&settings.servers_dir(), &slugify(&req.name));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    emit_stage(app, &id, "downloading-core").await;
    let resolution = resolve_core(&client, &core, &mc, &req.loader_version).await?;

    let mut jar = String::new();
    let mut loader_version = req.loader_version.clone();
    match resolution {
        Resolution::Direct { url, filename } => {
            jar = filename;
            download_file(
                &client,
                &url,
                &dir.join(sane_filename(&jar)),
                app,
                &format!("server:{id}:core"),
            )
            .await?;
        }
        Resolution::Installer { url, filename } => {
            download_file(
                &client,
                &url,
                &dir.join(&filename),
                app,
                &format!("server:{id}:core"),
            )
            .await?;
            let java =
                crate::server::java::server_java(settings, req.java_path.as_deref(), req.java)
                    .await?;
            let loader = loader_version.as_deref().unwrap_or_default();
            let j = match core.as_str() {
                "forge" | "neoforge" => {
                    install_forge(app, &java, &dir, &core, &mc, loader, &filename).await?
                }
                "fabric" => Some(
                    crate::server::installers::install_fabric(
                        &java,
                        &dir,
                        &dir.join(&filename),
                        &mc,
                        loader,
                    )
                    .await?,
                ),
                "quilt" => Some(
                    crate::server::installers::install_quilt(
                        &java,
                        &dir,
                        &dir.join(&filename),
                        &mc,
                        loader,
                    )
                    .await?,
                ),
                _ => None,
            };
            if let Some(j) = j {
                jar = j;
            } else {
                return Err("unable to locate installed server jar".into());
            }
        }
    }
    if jar.is_empty() {
        return Err("failed to determine server jar".into());
    }

    // eula
    std::fs::write(dir.join("eula.txt"), "eula=true\n").map_err(|e| e.to_string())?;

    let is_proxy = core == "velocity";
    if is_proxy {
        let vcfg = crate::server::velocity::VelocityConfig::default();
        crate::server::velocity::write(&dir, &vcfg)?;
    } else {
        let props = default_server_properties(&req.name);
        crate::utils::props::write_properties(&dir.join("server.properties"), &props)?;
    }

    let meta = ServerMeta {
        name: req.name.trim().to_string(),
        id,
        java: req.java,
        core: core.clone(),
        mc_version: mc.clone(),
        loader_version: loader_version.take(),
        build: None,
        jar,
        min_ram: req.min_ram,
        max_ram: req.max_ram,
        created_at: now_iso(),
        is_proxy,
        source_url: None,
        warnings: Vec::new(),
        auto_backup_days: 0,
        auto_backup_keep: 0,
        java_path: req.java_path.clone(),
        jvm_preset: req.jvm_preset.clone().unwrap_or_else(|| "auto".to_string()),
        jvm_args: req.jvm_args.clone().unwrap_or_default(),
        auto_start_on_boot: false,
        auto_restart_on_crash: false,
        schedule: Vec::new(),
    };
    save_meta(&dir, &meta)?;
    emit_stage(app, &meta.id, "done").await;
    Ok(meta)
}
