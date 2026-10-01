use crate::utils::props;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::Path;

/// A curated ordering + label map used by the frontend to render the config form.
pub const PROPERTY_ORDER: &[&str] = &[
    "server-port",
    "online-mode",
    "motd",
    "gamemode",
    "difficulty",
    "max-players",
    "white-list",
    "view-distance",
    "spawn-protection",
    "pvp",
    "allow-nether",
    "level-name",
    "level-seed",
    "enable-command-block",
    "enable-rcon",
    "rcon.port",
    "rcon.password",
    "network-compression-threshold",
    "max-tick-time",
    "enable-query",
    "query.port",
    "enable-status",
    "require-resource-pack",
];

pub fn read(path: &Path) -> Result<Map<String, Value>, String> {
    let map = props::read_properties(path)?;
    let mut out = Map::new();
    // emit in canonical order, then any extras
    let mut known: Vec<(String, String)> = Vec::new();
    let mut extras: Vec<(String, String)> = Vec::new();
    for key in PROPERTY_ORDER {
        if let Some(v) = map.get(*key) {
            known.push(((*key).to_string(), v.clone()));
        }
    }
    for (k, v) in &map {
        if !PROPERTY_ORDER.contains(&k.as_str()) {
            extras.push((k.clone(), v.clone()));
        }
    }
    for (k, v) in known.into_iter().chain(extras) {
        out.insert(k, Value::String(v));
    }
    Ok(out)
}

pub fn write(path: &Path, values: &Map<String, Value>) -> Result<(), String> {
    let mut map: HashMap<String, String> = HashMap::new();
    for (k, v) in values {
        let s = match v {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            _ => v.to_string(),
        };
        map.insert(k.clone(), s);
    }
    props::write_properties(path, &map)
}
