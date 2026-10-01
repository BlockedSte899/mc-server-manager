use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VelocityServer {
    pub address: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Default)]
pub struct VelocityServers {
    pub servers: HashMap<String, VelocityServer>,
    pub try_servers: Vec<String>,
}

impl VelocityServers {
    pub fn from_value(v: &toml::Value) -> Self {
        let mut out = VelocityServers::default();
        let Some(table) = v.as_table() else {
            return out;
        };
        for (k, val) in table {
            if k == "try" {
                out.try_servers = val
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    })
                    .unwrap_or_default();
            } else if let Ok(ss) = val.clone().try_into::<VelocityServer>() {
                out.servers.insert(k.clone(), ss);
            }
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct VelocityConfig {
    pub bind: String,
    pub motd: String,
    pub show_max_players: i64,
    pub online_mode: bool,
    pub force_key_authentication: bool,
    pub prevent_client_proxy_connections: bool,
    pub player_info_forwarding_mode: String,
    pub forwarding_secret: String,
    pub servers: VelocityServers,
    pub forced_hosts: HashMap<String, String>,
}

impl Default for VelocityConfig {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0:25577".to_string(),
            motd: "&3A Velocity Proxy".to_string(),
            show_max_players: 500,
            online_mode: true,
            force_key_authentication: true,
            prevent_client_proxy_connections: false,
            player_info_forwarding_mode: "modern".to_string(),
            forwarding_secret: generate_secret(),
            servers: VelocityServers::default(),
            forced_hosts: HashMap::new(),
        }
    }
}

/// JSON shape for the frontend: `servers` stays a plain map, `try` (if any) is
/// exposed as a top-level key so the UI never confuses it with a registered server.
impl Serialize for VelocityConfig {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let mut map = s.serialize_map(Some(
            9 + if self.servers.try_servers.is_empty() {
                0
            } else {
                1
            },
        ))?;
        map.serialize_entry("bind", &self.bind)?;
        map.serialize_entry("motd", &self.motd)?;
        map.serialize_entry("show_max_players", &self.show_max_players)?;
        map.serialize_entry("online_mode", &self.online_mode)?;
        map.serialize_entry("force_key_authentication", &self.force_key_authentication)?;
        map.serialize_entry(
            "prevent_client_proxy_connections",
            &self.prevent_client_proxy_connections,
        )?;
        map.serialize_entry(
            "player_info_forwarding_mode",
            &self.player_info_forwarding_mode,
        )?;
        map.serialize_entry("forwarding-secret", &self.forwarding_secret)?;
        map.serialize_entry("servers", &self.servers.servers)?;
        if !self.servers.try_servers.is_empty() {
            map.serialize_entry("try", &self.servers.try_servers)?;
        }
        map.serialize_entry("forced_hosts", &self.forced_hosts)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for VelocityConfig {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let raw = toml::Value::deserialize(d)?;
        let table = raw
            .as_table()
            .ok_or_else(|| serde::de::Error::custom("velocity.toml: expected a table"))?;
        let mut cfg = VelocityConfig::default();
        for (k, v) in table {
            match k.as_str() {
                "bind" => cfg.bind = v.as_str().unwrap_or_default().to_string(),
                "motd" => cfg.motd = v.as_str().unwrap_or_default().to_string(),
                "show_max_players" => {
                    cfg.show_max_players = v.as_integer().unwrap_or(cfg.show_max_players)
                }
                "online_mode" => cfg.online_mode = v.as_bool().unwrap_or(cfg.online_mode),
                "force_key_authentication" => {
                    cfg.force_key_authentication =
                        v.as_bool().unwrap_or(cfg.force_key_authentication)
                }
                "prevent_client_proxy_connections" => {
                    cfg.prevent_client_proxy_connections =
                        v.as_bool().unwrap_or(cfg.prevent_client_proxy_connections)
                }
                "player_info_forwarding_mode" => {
                    cfg.player_info_forwarding_mode = v.as_str().unwrap_or_default().to_string()
                }
                "forwarding-secret" => {
                    cfg.forwarding_secret = v.as_str().unwrap_or_default().to_string()
                }
                "servers" => cfg.servers = VelocityServers::from_value(v),
                "try" => {
                    cfg.servers.try_servers = v
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default()
                }
                "forced_hosts" => {
                    if let Some(t) = v.as_table() {
                        for (hk, hv) in t {
                            if let Some(s) = hv.as_str() {
                                cfg.forced_hosts.insert(hk.clone(), s.to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(cfg)
    }
}

pub const VELOCITY_FILE: &str = "velocity.toml";

pub fn generate_secret() -> String {
    let a = uuid::Uuid::new_v4().simple().to_string();
    let b = uuid::Uuid::new_v4().simple().to_string();
    format!("{a}{b}")
}

pub fn read(dir: &Path) -> Result<VelocityConfig, String> {
    let path = dir.join(VELOCITY_FILE);
    let text = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    toml::from_str::<VelocityConfig>(&text).map_err(|e| format!("invalid velocity.toml: {e}"))
}

/// Writes velocity.toml with the `try` list placed *inside* the `[servers]`
/// table (as modern Velocity expects), matching what the parser accepts.
pub fn write(dir: &Path, cfg: &VelocityConfig) -> Result<(), String> {
    let path = dir.join(VELOCITY_FILE);
    let mut servers_table = toml::map::Map::new();
    for (name, srv) in &cfg.servers.servers {
        let mut t = toml::map::Map::new();
        t.insert(
            "address".to_string(),
            toml::Value::String(srv.address.clone()),
        );
        t.insert("enabled".to_string(), toml::Value::Boolean(srv.enabled));
        servers_table.insert(name.clone(), toml::Value::Table(t));
    }
    if !cfg.servers.try_servers.is_empty() {
        servers_table.insert(
            "try".to_string(),
            toml::Value::Array(
                cfg.servers
                    .try_servers
                    .iter()
                    .map(|s| toml::Value::String(s.clone()))
                    .collect(),
            ),
        );
    }
    let mut root = toml::map::Map::new();
    root.insert("bind".to_string(), toml::Value::String(cfg.bind.clone()));
    root.insert("motd".to_string(), toml::Value::String(cfg.motd.clone()));
    root.insert(
        "show_max_players".to_string(),
        toml::Value::Integer(cfg.show_max_players),
    );
    root.insert(
        "online_mode".to_string(),
        toml::Value::Boolean(cfg.online_mode),
    );
    root.insert(
        "force_key_authentication".to_string(),
        toml::Value::Boolean(cfg.force_key_authentication),
    );
    root.insert(
        "prevent_client_proxy_connections".to_string(),
        toml::Value::Boolean(cfg.prevent_client_proxy_connections),
    );
    root.insert(
        "player_info_forwarding_mode".to_string(),
        toml::Value::String(cfg.player_info_forwarding_mode.clone()),
    );
    root.insert(
        "forwarding-secret".to_string(),
        toml::Value::String(cfg.forwarding_secret.clone()),
    );
    root.insert("servers".to_string(), toml::Value::Table(servers_table));
    let mut forced = toml::map::Map::new();
    for (k, v) in &cfg.forced_hosts {
        forced.insert(k.clone(), toml::Value::String(v.clone()));
    }
    root.insert("forced_hosts".to_string(), toml::Value::Table(forced));
    let text = toml::to_string(&toml::Value::Table(root)).map_err(|e| e.to_string())?;
    std::fs::write(&path, text).map_err(|e| e.to_string())
}
