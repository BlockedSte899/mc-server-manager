use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin};

use crate::server::manager::ServerMeta;
use crate::settings::Settings;
use crate::utils::RingBuffer;
use tauri::Manager;

pub type RunningMap = Arc<tokio::sync::Mutex<HashMap<String, RunningServer>>>;
pub type PlayersCache = Arc<tokio::sync::Mutex<HashMap<String, (Vec<String>, i32)>>>;

#[derive(Clone, Default)]
pub struct ProcessManager {
    pub running: RunningMap,
}

pub struct RunningServer {
    pub pid: u32,
    pub child: Child,
    pub stdin: Option<ChildStdin>,
    pub console: Arc<std::sync::Mutex<RingBuffer>>,
    pub started_at: Instant,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatusEvent {
    pub id: String,
    pub state: String,
    pub code: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct StatusInfo {
    pub state: String,
    pub pid: Option<u32>,
    pub uptime_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConsoleLine {
    pub line: String,
}

/// Emitted when a server process exits so the UI/notification layer can react
/// (crash highlighting, webhooks, etc). `expected` is false when the process
/// died without the user stopping it.
#[derive(Debug, Clone, Serialize)]
pub struct ExitEvent {
    pub id: String,
    pub code: Option<i32>,
    pub expected: bool,
}

/// Everything the watchdog needs to bring a crashed server back up.
pub struct RestartContext {
    pub settings: Settings,
    pub meta: ServerMeta,
    pub pm: ProcessManager,
    pub system: Arc<tokio::sync::Mutex<sysinfo::System>>,
    pub players: PlayersCache,
}

fn emit_status(app: &tauri::AppHandle, id: &str, state: &str, code: Option<i32>) {
    use tauri::Emitter;
    let _ = app.emit(
        "server-status",
        StatusEvent {
            id: id.to_string(),
            state: state.to_string(),
            code,
        },
    );
}

fn spawn_console_reader<R>(
    app: tauri::AppHandle,
    id: String,
    reader: R,
    console: Arc<std::sync::Mutex<RingBuffer>>,
    players: PlayersCache,
    is_proxy: bool,
) where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    use tauri::Emitter;
    tokio::spawn(async move {
        let mut lines = BufReader::new(reader).lines();
        loop {
            match lines.next_line().await {
                Ok(Some(line)) => {
                    if let Ok(mut c) = console.lock() {
                        c.push(line.clone() + "\n");
                    }
                    let _ = app.emit(
                        &format!("server:{id}:console"),
                        ConsoleLine { line: line.clone() },
                    );
                    if let Some((names, max)) =
                        crate::server::players::update_from_line(&players, &id, &line, is_proxy)
                            .await
                    {
                        let _ = app.emit(
                            "server-players",
                            crate::server::players::PlayersEvent {
                                id: id.clone(),
                                names,
                                max,
                            },
                        );
                    }
                }
                _ => {
                    let _ = app.emit(
                        &format!("server:{id}:console"),
                        ConsoleLine {
                            line: "\u{001b}[2J-- EOF --".into(),
                        },
                    );
                    break;
                }
            }
        }
    });
}

/// Watches a server process and emits its exit. When the process dies on its
/// own (not via [`stop`]/[`kill`], which remove it from the map first) the
/// exit counts as a crash: the UI is told, and — if the server opted in — the
/// process is brought back up after a short delay.
fn spawn_monitor(
    app: tauri::AppHandle,
    id: String,
    pm: ProcessManager,
    restart_ctx: RestartContext,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut guard = pm.running.lock().await;
            let Some(srv) = guard.get_mut(&id) else { break };
            match srv.child.try_wait() {
                Ok(Some(status)) => {
                    let code = status.code().or_else(|| {
                        #[cfg(unix)]
                        {
                            use std::os::unix::process::ExitStatusExt;
                            status.signal()
                        }
                        #[cfg(not(unix))]
                        {
                            None
                        }
                    });
                    guard.remove(&id);
                    drop(guard);
                    emit_status(&app, &id, "stopped", code);

                    use tauri::Emitter;
                    let _ = app.emit(
                        "server-exit",
                        ExitEvent {
                            id: id.clone(),
                            code,
                            expected: false,
                        },
                    );

                    let ctx = &restart_ctx;
                    {
                        let name = ctx.meta.name.clone();
                        let settings = ctx.settings.clone();
                        tauri::async_runtime::spawn(async move {
                            crate::notify::notify(&settings, &name, crate::notify::Event::Crash)
                                .await;
                        });
                    }
                    // Re-read the meta: the user may have flipped the
                    // watchdog switch while the server was running.
                    {
                        let meta = crate::server::manager::find_server(&ctx.settings, &id)
                            .unwrap_or_else(|_| ctx.meta.clone());
                        if meta.auto_restart_on_crash {
                            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                            let st = crate::settings::Settings::load();
                            let fresh = crate::server::manager::find_server(&st, &id)
                                .unwrap_or(meta.clone());
                            if !fresh.auto_restart_on_crash {
                                break;
                            }
                            eprintln!("[watchdog] restarting {id} after crash");
                            if let Err(e) = start(
                                &app,
                                &ctx.pm,
                                ctx.system.clone(),
                                &st,
                                &fresh,
                                ctx.players.clone(),
                            )
                            .await
                            {
                                eprintln!("[watchdog] restart of {id} failed: {e}");
                            }
                        }
                    }
                    break;
                }
                Err(_) => break,
                Ok(None) => {}
            }
        }
    });
}

fn spawn_metrics(
    app: tauri::AppHandle,
    id: String,
    pid: u32,
    pm: ProcessManager,
    system: Arc<tokio::sync::Mutex<sysinfo::System>>,
    ui_visible: Arc<std::sync::atomic::AtomicBool>,
) {
    tokio::spawn(async move {
        loop {
            {
                let guard = pm.running.lock().await;
                if !guard.contains_key(&id) {
                    break;
                }
            }
            // When the window is hidden (trayed out) nobody is watching the
            // charts, so poll far less often to save CPU.
            let hidden = !ui_visible.load(std::sync::atomic::Ordering::Relaxed);
            if hidden {
                tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                continue;
            }
            let mut sys = system.lock().await;
            // Targeted refresh only: refresh_all() also walks disks/networks
            // every tick, which is wasted work for a couple of counters.
            sys.refresh_cpu_usage();
            sys.refresh_memory();
            let proc_pid = sysinfo::Pid::from_u32(pid);
            let (proc_cpu, proc_mem) = match sys.process(proc_pid) {
                Some(p) => (Some(p.cpu_usage()), Some(p.memory())),
                None => (None, None),
            };
            use tauri::Emitter;
            let _ = app.emit(
                "server-metrics",
                crate::server::metrics::ServerMetrics {
                    id: id.clone(),
                    cpu_percent: proc_cpu,
                    mem_bytes: proc_mem,
                    system_cpu_percent: sys.global_cpu_info().cpu_usage(),
                    system_mem_used: sys.used_memory(),
                    system_mem_total: sys.total_memory(),
                },
            );
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    });
}

/// Builds the java command-line for a server.
fn java_command(java: &Path, meta: &ServerMeta, server_dir: &Path) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new(java);
    crate::utils::console::hide_console(&mut cmd);
    cmd.arg(format!("-Xms{}M", meta.min_ram))
        .arg(format!("-Xmx{}M", meta.max_ram));
    cmd.args(crate::server::jvm::extra_args(meta));
    cmd.arg("-jar")
        .arg(server_dir.join(&meta.jar))
        .arg("nogui")
        .current_dir(server_dir)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());
    cmd
}

pub async fn start(
    app: &tauri::AppHandle,
    pm: &ProcessManager,
    system: Arc<tokio::sync::Mutex<sysinfo::System>>,
    settings: &Settings,
    meta: &ServerMeta,
    players_cache: PlayersCache,
) -> Result<(), String> {
    {
        let guard = pm.running.lock().await;
        if guard.contains_key(&meta.id) {
            return Err("server is already running".into());
        }
    }

    let server_dir = settings.server_folder(&meta.id);
    let jar = server_dir.join(&meta.jar);
    if !jar.exists() {
        return Err(format!("server jar not found: {}", jar.display()));
    }
    let java =
        crate::server::java::server_java(settings, meta.java_path.as_deref(), meta.java).await?;

    let mut cmd = java_command(&java, meta, &server_dir);
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("failed to start java: {e}"))?;
    let pid = child.id().unwrap_or(0);
    let stdin = child.stdin.take().ok_or("could not take stdin")?;
    let stdout = child.stdout.take().ok_or("could not take stdout")?;
    let stderr = child.stderr.take().ok_or("could not take stderr")?;

    let console = Arc::new(std::sync::Mutex::new(RingBuffer::new(
        settings.console_max_lines,
    )));
    let running = RunningServer {
        pid,
        child,
        stdin: Some(stdin),
        console: console.clone(),
        started_at: Instant::now(),
    };
    pm.running.lock().await.insert(meta.id.clone(), running);

    let pm2 = pm.clone();
    let pm3 = pm.clone();
    let restart_ctx = RestartContext {
        settings: settings.clone(),
        meta: meta.clone(),
        pm: pm.clone(),
        system: system.clone(),
        players: players_cache.clone(),
    };
    spawn_console_reader(
        app.clone(),
        meta.id.clone(),
        stdout,
        console.clone(),
        players_cache.clone(),
        meta.is_proxy(),
    );
    spawn_console_reader(
        app.clone(),
        meta.id.clone(),
        stderr,
        console,
        players_cache.clone(),
        meta.is_proxy(),
    );
    spawn_monitor(app.clone(), meta.id.clone(), pm2, restart_ctx);
    let ui_visible = app.state::<crate::AppState>().ui_visible.clone();
    spawn_metrics(app.clone(), meta.id.clone(), pid, pm3, system, ui_visible);

    emit_status(app, &meta.id, "running", None);
    let name = meta.name.clone();
    let settings = settings.clone();
    tauri::async_runtime::spawn(async move {
        crate::notify::notify(&settings, &name, crate::notify::Event::Start).await;
    });
    Ok(())
}

pub async fn stop(app: &tauri::AppHandle, pm: &ProcessManager, id: &str) -> Result<(), String> {
    let mut guard = pm.running.lock().await;
    let Some(srv) = guard.get_mut(id) else {
        return Ok(());
    };
    emit_status(app, id, "stopping", None);
    // graceful: write "stop" then wait with timeout, then hard kill
    let write_result = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        if let Some(stdin) = srv.stdin.as_mut() {
            let _ = stdin.write_all(b"stop\n").await;
            let _ = stdin.flush().await;
        }
    })
    .await;
    let _ = write_result;

    let graceful = tokio::time::timeout(std::time::Duration::from_secs(30), srv.child.wait())
        .await
        .ok()
        .and_then(|r| r.ok());
    if graceful.is_none() {
        let _ = srv.child.kill().await;
        let _ = tokio::time::timeout(std::time::Duration::from_secs(10), srv.child.wait()).await;
    }
    let code = graceful.and_then(|st| st.code());
    guard.remove(id);
    drop(guard);
    emit_status(app, id, "stopped", code);
    Ok(())
}

/// Gracefully stops every running server: sends `stop` to all of them, waits a
/// generous budget for a clean shutdown, then hard-kills any that linger.
pub async fn stop_all(app: &tauri::AppHandle, pm: &ProcessManager) {
    let ids: Vec<String> = {
        let guard = pm.running.lock().await;
        guard.keys().cloned().collect()
    };
    if ids.is_empty() {
        return;
    }
    for id in &ids {
        let _ = send_input(pm, id, "stop").await;
    }
    let deadline = Instant::now() + std::time::Duration::from_secs(20);
    loop {
        {
            let guard = pm.running.lock().await;
            if guard.is_empty() {
                break;
            }
        }
        if Instant::now() >= deadline {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
    for id in &ids {
        let _ = kill(app, pm, id).await;
    }
}

pub async fn kill(app: &tauri::AppHandle, pm: &ProcessManager, id: &str) -> Result<(), String> {
    let mut guard = pm.running.lock().await;
    let Some(srv) = guard.get_mut(id) else {
        return Ok(());
    };
    emit_status(app, id, "stopping", None);
    let _ = srv.child.kill().await;
    let exited = tokio::time::timeout(std::time::Duration::from_secs(10), srv.child.wait())
        .await
        .ok()
        .and_then(|r| r.ok());
    let code = exited.and_then(|st| st.code());
    guard.remove(id);
    drop(guard);
    emit_status(app, id, "stopped", code);
    Ok(())
}

pub async fn send_input(pm: &ProcessManager, id: &str, line: &str) -> Result<(), String> {
    // server consoles (vanilla/paper/proxy) accept commands without a leading `/`
    let line = line.trim_start_matches('/');
    let mut guard = pm.running.lock().await;
    let srv = guard.get_mut(id).ok_or("server is not running")?;
    let mut stdin = srv.stdin.take().ok_or("stdin unavailable")?;
    let res = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let _ = stdin.write_all(format!("{line}\n").as_bytes()).await;
        let _ = stdin.flush().await;
    })
    .await
    .map_err(|_| "write timed out".to_string());
    srv.stdin = Some(stdin);
    res
}

pub async fn status(pm: &ProcessManager, id: &str) -> StatusInfo {
    let guard = pm.running.lock().await;
    match guard.get(id) {
        Some(srv) => StatusInfo {
            state: "running".into(),
            pid: Some(srv.pid),
            uptime_secs: Some(srv.started_at.elapsed().as_secs()),
        },
        None => StatusInfo::default(),
    }
}

pub async fn console_tail(pm: &ProcessManager, id: &str) -> Vec<String> {
    let guard = pm.running.lock().await;
    match guard.get(id) {
        Some(srv) => srv.console.lock().map(|c| c.to_vec()).unwrap_or_default(),
        None => Vec::new(),
    }
}

pub async fn clear_console(pm: &ProcessManager, id: &str) {
    let guard = pm.running.lock().await;
    if let Some(srv) = guard.get(id) {
        if let Ok(mut c) = srv.console.lock() {
            c.clear();
        }
    }
}
