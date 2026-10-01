use std::path::PathBuf;

pub const APP_DIR_NAME: &str = "mc-server-manager";

pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_DIR_NAME)
}

pub fn default_servers_dir() -> PathBuf {
    config_dir().join("servers")
}

pub fn settings_file() -> PathBuf {
    config_dir().join("settings.json")
}

/// Sanitizes a string into a filename-safe form.
pub fn sane_filename(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else if c.is_alphanumeric() {
                c
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim_matches(['.', '-', '_']);
    if cleaned.is_empty() {
        "file".to_string()
    } else {
        cleaned.to_string()
    }
}

pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}
