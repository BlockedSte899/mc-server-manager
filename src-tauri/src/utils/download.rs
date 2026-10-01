use futures_util::StreamExt;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::io::AsyncWriteExt;

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub key: String,
    pub received: u64,
    pub total: u64,
    pub percent: f64,
}

/// Downloads `url` to `dest`, emitting `download:progress` events tagged with `key`.
pub async fn download_file(
    client: &reqwest::Client,
    url: &str,
    dest: &Path,
    app: &AppHandle,
    key: &str,
) -> Result<PathBuf, String> {
    if url.is_empty() {
        return Err("empty download url".into());
    }
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("download request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} for {}", resp.status(), url));
    }
    let total = resp.content_length().unwrap_or(0);
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| e.to_string())?;
    let mut stream = resp.bytes_stream();
    let mut received: u64 = 0;
    let key = key.to_string();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| format!("stream error: {e}"))?;
        received += chunk.len() as u64;
        file.write_all(&chunk).await.map_err(|e| e.to_string())?;
        let percent = if total > 0 {
            received as f64 / total as f64 * 100.0
        } else {
            0.0
        };
        let _ = app.emit(
            "download:progress",
            DownloadProgress {
                key: key.clone(),
                received,
                total,
                percent,
            },
        );
    }
    file.flush().await.map_err(|e| e.to_string())?;
    let meta = tokio::fs::metadata(dest).await.map_err(|e| e.to_string())?;
    if meta.len() == 0 {
        let _ = tokio::fs::remove_file(dest).await;
        return Err(format!("downloaded file is empty: {url}"));
    }
    Ok(dest.to_path_buf())
}

/// Percent-encodes a string for use in a query string.
pub fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Simple sha1 hex helper.
pub fn sha1_hex(data: &[u8]) -> String {
    use sha1::Digest;
    let mut hasher = sha1::Sha1::new();
    hasher.update(data);
    format!("{:x}", hasher.finalize())
}

pub fn file_sha1(path: &Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    Ok(sha1_hex(&bytes))
}

/// Extracts all <!-- <tag>..</tag> --> values from a maven-metadata.xml string.
pub fn extract_xml_tags(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        if let Some(end) = after.find(&close) {
            out.push(after[..end].to_string());
            rest = &after[end..];
        } else {
            break;
        }
    }
    out
}
