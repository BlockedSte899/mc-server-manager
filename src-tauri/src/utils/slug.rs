use std::path::PathBuf;

/// Turns an arbitrary name into a url/folder friendly slug.
pub fn slugify(name: &str) -> String {
    let cleaned: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '-'
            }
        })
        .collect();
    let parts: Vec<&str> = cleaned.split('-').filter(|s| !s.is_empty()).collect();
    let base = if parts.is_empty() {
        "server".to_string()
    } else {
        parts.join("-")
    };
    base
}

/// Returns a unique folder name and its path under servers_dir.
pub fn unique_folder(servers_dir: &std::path::Path, slug: &str) -> (String, PathBuf) {
    let mut candidate = slug.to_string();
    let mut n = 2;
    let mut path = servers_dir.join(&candidate);
    while path.exists() {
        candidate = format!("{slug}-{n}");
        path = servers_dir.join(&candidate);
        n += 1;
    }
    (candidate, path)
}
