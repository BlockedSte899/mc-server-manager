use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Entry {
    pub name: String,
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlayersEvent {
    pub id: String,
    pub names: Vec<String>,
    pub max: i32,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct PlayersView {
    pub online: Vec<String>,
    pub max_online: i32,
    pub whitelist_enabled: bool,
    pub whitelist: Vec<Entry>,
    pub banned: Vec<Entry>,
    pub banned_ips: Vec<String>,
    pub ops: Vec<Entry>,
    pub usercache: Vec<Entry>,
}

fn read_json_entries(dir: &Path, file: &str) -> Vec<Entry> {
    let text = match std::fs::read_to_string(dir.join(file)) {
        Ok(t) => t,
        Err(_) => return Vec::new(),
    };
    let entries: Vec<Entry> = match serde_json::from_str(&text) {
        Ok(e) => e,
        Err(_) => Vec::new(),
    };
    entries
        .into_iter()
        .map(|mut e| {
            e.uuid = if e.uuid.contains('-') {
                e.uuid.clone()
            } else {
                add_uuid_dashes(&e.uuid)
            };
            e
        })
        .collect()
}

fn add_uuid_dashes(uuid: &str) -> String {
    if uuid.len() != 32 || uuid.contains('-') {
        return uuid.to_string();
    }
    format!(
        "{}-{}-{}-{}-{}",
        &uuid[0..8],
        &uuid[8..12],
        &uuid[12..16],
        &uuid[16..20],
        &uuid[20..32]
    )
}

/// Strips minecraft section-sign colour codes.
pub fn strip_color(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '\u{00a7}' && i + 1 < chars.len() {
            i += 2;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

/// Parses the most recent `/list` output to extract online players + max.
/// Only genuine, well-formed summary lines are accepted so that mangled
/// kick/ban feedback can never pollute the player cache.
pub fn parse_online(lines: &[String]) -> (Vec<String>, i32) {
    for raw in lines.iter().rev() {
        let line = strip_color(raw);
        if let Some((names, max)) = parse_summary_line(&line) {
            return (names, max);
        }
    }
    (Vec::new(), 0)
}

/// Extracts a strict `/list` summary: "<…> There are N of a max of M players
/// online: name1, name2 …". Returns `None` unless the surrounding text matches
/// the canonical shape and every parsed name is a valid player name, so merged
/// or corrupted lines (e.g. "Banned There are 1 of a max of 20 players
/// online: BlockedSte899 Bye!") are rejected outright.
fn parse_summary_line(line: &str) -> Option<(Vec<String>, i32)> {
    let lower = line.to_lowercase();
    let i = lower.find("players online")?;
    // Require a canonical "of a max of <M> " right before "players online".
    let before = &lower[..i];
    let marker = "of a max of ";
    let mi = before.rfind(marker)?;
    let after_marker = before[mi + marker.len()..].trim_start();
    let digits: String = after_marker
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    let raw = line[i + "players online".len()..].trim_start();
    let raw = raw.strip_prefix(':').unwrap_or(raw).trim_start();
    let names: Vec<String> = raw
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| is_valid_name(s))
        .collect();
    // Accept an empty list, or a list whose every entry is a valid player name.
    // Any leftover junk (e.g. a merged "BlockedSte899 Bye!") yields no valid
    // names and rejects the whole line so it can't pollute the cache.
    if raw.is_empty() || !names.is_empty() {
        let max = extract_max(line).unwrap_or(0);
        return Some((names, max));
    }
    None
}

/// Minecraft-style player name: `[A-Za-z0-9_]{1,16}`.
fn is_valid_name(s: &str) -> bool {
    !s.is_empty() && s.len() <= 16 && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn extract_max(line: &str) -> Option<i32> {
    // "There are 3 of a max of 20 players online: ..."
    let lower = line.to_lowercase();
    if let Some(idx) = lower.find("of a max of") {
        let rest = &lower[idx + "of a max of".len()..];
        let num: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        return num.parse().ok();
    }
    // "8/20 players"
    if let Some(idx) = lower.find('/') {
        let after = &lower[idx + 1..];
        let num: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        if !num.is_empty() {
            return num.parse().ok();
        }
    }
    None
}

/// Reads players from the on-disk console log as a fallback when the in-memory
/// console tail is empty (e.g. right after boot).
pub fn read_online_from_logs(dir: &Path) -> (Vec<String>, i32) {
    let logs_dir = dir.join("logs");
    let latest = logs_dir.join("latest.log");
    if let Ok(text) = std::fs::read_to_string(&latest) {
        let lines: Vec<String> = text.lines().map(String::from).collect();
        let (names, max) = parse_online(&lines);
        if !names.is_empty() || max > 0 {
            return (names, max);
        }
    }
    if let Ok(rd) = std::fs::read_dir(&logs_dir) {
        let mut files: Vec<(std::path::PathBuf, std::time::SystemTime)> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|x| x == "log").unwrap_or(false))
            .filter_map(|e| {
                e.metadata().ok().map(|m| {
                    (
                        e.path(),
                        m.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                    )
                })
            })
            .collect();
        files.sort_by_key(|(_, t)| *t);
        if let Some((path, _)) = files.last() {
            if let Ok(text) = std::fs::read_to_string(path) {
                let lines: Vec<String> = text.lines().map(String::from).collect();
                return parse_online(&lines);
            }
        }
    }
    (Vec::new(), 0)
}

/// Parses Velocity's `velocity list` output:
/// - "No players currently connected."
/// - "1 player(s) connected: [Alice]"
/// - "2 player(s) connected: [Alice, Bob]" (older, one bracket = all players)
/// - "2 player(s) connected: [Hub, Alice], [Hub2, Bob]" (newer, server prefix)
pub fn parse_online_velocity(lines: &[String]) -> (Vec<String>, i32) {
    for raw in lines.iter().rev() {
        let line = strip_color(raw);
        let lower = line.to_lowercase();
        if !lower.contains("player(s) connected") && !lower.contains("players currently connected")
        {
            continue;
        }
        if lower.contains("no players") {
            return (Vec::new(), 1);
        }
        // max: the number right before " player(s)"
        let mut max = 0;
        if let Some(idx) = lower.find(" player") {
            let digits: String = lower[..idx]
                .chars()
                .rev()
                .take_while(|c| c.is_ascii_digit())
                .collect::<Vec<char>>()
                .into_iter()
                .rev()
                .collect();
            if let Ok(n) = digits.parse::<i32>() {
                max = n;
            }
        }
        let mut names = Vec::new();
        let brackets: Vec<String> = line
            .match_indices('[')
            .filter_map(|(i, _)| line[i..].find(']').map(|e| line[i + 1..i + e].to_string()))
            .collect();
        let multi = brackets.len() > 1;
        for inner in brackets {
            let parts: Vec<String> = inner
                .split(',')
                .map(|p| p.trim().to_string())
                .filter(|p| !p.is_empty() && is_valid_name(p))
                .collect();
            if parts.is_empty() {
                continue;
            }
            let pick: Vec<String> = if multi {
                parts[parts.len() - 1..].to_vec()
            } else {
                parts
            };
            for n in pick {
                if !names.contains(&n) {
                    names.push(n);
                }
            }
        }
        return (names, max.max(1));
    }
    (Vec::new(), 0)
}

/// Derives the online-player snapshot from a single console line, mutating the
/// shared cache and returning the new `(names, max)` when it changed. No server
/// commands are ever sent from here.
///
/// Handled messages:
/// - `/list` summary (printed at boot): "There are N of a max of M players online: …"
/// - Java join/leave: "X joined the game" / "X left the game"
/// - Velocity connect/disconnect: "X has connected" / "X has disconnected"
pub async fn update_from_line(
    cache: &crate::server::process::PlayersCache,
    id: &str,
    raw: &str,
    is_proxy: bool,
) -> Option<(Vec<String>, i32)> {
    let line = strip_color(raw).trim().to_string();
    if line.is_empty() {
        return None;
    }

    let mut guard = cache.lock().await;
    let entry = guard.entry(id.to_string()).or_insert((Vec::new(), 0));
    let (names, max) = entry;

    // Authoritative snapshot from a well-formed `/list` summary (or the boot
    // line). Mangled feedback like "Banned There are 1 of a max of 20 players
    // online: BlockedSte899 Bye!" is rejected by parse_summary_line so it can
    // never put fake player names into the cache.
    if let Some((parsed, parsed_max)) = parse_summary_line(&line) {
        let changed = *names != parsed;
        if changed || parsed_max > 0 {
            *names = parsed;
            if parsed_max > 0 {
                *max = parsed_max;
            }
            return Some((names.clone(), *max));
        }
        return None;
    }

    let mutation = if is_proxy {
        matched_text_before(&line, "has connected")
            .filter(|u| is_valid_name(u))
            .map(|u| add_name(names, &u))
            .or_else(|| {
                matched_text_before(&line, "has disconnected")
                    .filter(|u| is_valid_name(u))
                    .map(|u| remove_name(names, &u))
            })
    } else {
        matched_text_before(&line, "joined the game")
            .filter(|u| is_valid_name(u))
            .map(|u| add_name(names, &u))
            .or_else(|| {
                matched_text_before(&line, "left the game")
                    .filter(|u| is_valid_name(u))
                    .map(|u| remove_name(names, &u))
            })
            .or_else(|| {
                matched_text_before(&line, "lost connection")
                    .filter(|u| is_valid_name(u))
                    .map(|u| remove_name(names, &u))
            })
    };
    match mutation {
        Some(updated) => {
            *names = updated;
            Some((names.clone(), *max))
        }
        None => None,
    }
}

/// If `needle` occurs in `line`, returns the trimmed word(s) right before it.
fn matched_text_before(line: &str, needle: &str) -> Option<String> {
    let idx = line.find(needle)?;
    let before = line[..idx].trim_end();
    let name = before.rsplit(' ').next().unwrap_or("").to_string();
    if name.is_empty() {
        return None;
    }
    Some(name)
}

fn add_name(names: &mut Vec<String>, name: &str) -> Vec<String> {
    if !names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
        names.push(name.to_string());
    }
    names.clone()
}

fn remove_name(names: &mut Vec<String>, name: &str) -> Vec<String> {
    names.retain(|n| !n.eq_ignore_ascii_case(name));
    names.clone()
}

/// Reads a `key=value` property from server.properties.
pub fn read_prop(dir: &Path, key: &str) -> Option<String> {
    std::fs::read_to_string(dir.join("server.properties"))
        .ok()?
        .lines()
        .find_map(|l| {
            l.split_once('=')
                .filter(|(k, _)| k.trim() == key)
                .map(|(_, v)| v.trim().to_string())
        })
}

/// Reads whitelist/ops/banned/user cache from a server directory.
pub fn read_view(dir: &Path) -> PlayersView {
    let whitelist_enabled = read_prop(dir, "white-list")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let banned_ips = std::fs::read_to_string(dir.join("banned-ips.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<Vec<serde_json::Value>>(&t).ok())
        .map(|v| {
            v.iter()
                .filter_map(|x| x.get("ip").and_then(|i| i.as_str()).map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let max_online = read_prop(dir, "max-players")
        .and_then(|v| v.parse::<i32>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(20);

    PlayersView {
        online: Vec::new(),
        max_online,
        whitelist_enabled,
        whitelist: read_json_entries(dir, "whitelist.json"),
        banned: read_json_entries(dir, "banned-players.json"),
        banned_ips,
        ops: read_json_entries(dir, "ops.json"),
        usercache: read_json_entries(dir, "usercache.json"),
    }
}
