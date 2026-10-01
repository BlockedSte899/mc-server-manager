use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomTheme {
    pub surface0: String,
    pub surface1: String,
    pub surface2: String,
    pub surface3: String,
    pub edge: String,
    pub fg: String,
    pub fg_dim: String,
    pub brand: String,
    pub accent: String,
    pub magenta: String,
}

impl Default for CustomTheme {
    fn default() -> Self {
        Self {
            surface0: "#0b1220".into(),
            surface1: "#111a2e".into(),
            surface2: "#182338".into(),
            surface3: "#22304a".into(),
            edge: "#2a3a58".into(),
            fg: "#e6edf7".into(),
            fg_dim: "#8fa3bf".into(),
            brand: "#27c985".into(),
            accent: "#38bdf8".into(),
            magenta: "#c084fc".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub servers_dir: String,
    pub java_overrides: BTreeMap<u32, String>,
    pub curseforge_api_key: Option<String>,
    pub min_ram_default: i64,
    pub max_ram_default: i64,
    pub console_max_lines: usize,
    pub ui_scale: i64,
    pub theme: String,
    pub tray_enabled: bool,
    pub lang: String,
    pub custom_theme: Option<CustomTheme>,
    pub notify_desktop: bool,
    pub notify_on_start: bool,
    pub notify_on_stop: bool,
    pub notify_on_crash: bool,
    pub notify_on_backup: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            servers_dir: crate::utils::paths::default_servers_dir()
                .to_string_lossy()
                .to_string(),
            java_overrides: BTreeMap::new(),
            curseforge_api_key: None,
            min_ram_default: 2048,
            max_ram_default: 4096,
            console_max_lines: 2000,
            ui_scale: 100,
            theme: "dark".to_string(),
            tray_enabled: true,
            lang: "en".to_string(),
            custom_theme: None,
            notify_desktop: true,
            notify_on_start: true,
            notify_on_stop: false,
            notify_on_crash: true,
            notify_on_backup: false,
        }
    }
}

impl Settings {
    pub fn load() -> Self {
        let path = crate::utils::paths::settings_file();
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let path = crate::utils::paths::settings_file();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())
    }

    pub fn servers_dir(&self) -> PathBuf {
        PathBuf::from(&self.servers_dir)
    }

    pub fn server_folder(&self, id: &str) -> PathBuf {
        self.servers_dir().join(id)
    }
}
