use java_properties::read;
use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

/// Reads a java .properties file into a map. Missing/empty files produce an empty map.
pub fn read_properties(path: &Path) -> Result<HashMap<String, String>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let map = read(reader).map_err(|e| e.to_string())?;
    Ok(map)
}

/// Escapes a key/value like java.util.Properties.store: backslash, control
/// chars and everything outside printable ASCII become `\uXXXX` with a full
/// four-digit hex — Java's Properties loader rejects shorter escapes.
fn escape_property(s: &str, is_key: bool) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for (i, c) in s.chars().enumerate() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{c}' => out.push_str("\\f"),
            ' ' if is_key && i == 0 => out.push_str("\\ "),
            '=' | ':' if is_key => {
                out.push('\\');
                out.push(c);
            }
            '#' | '!' if is_key => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}

/// Writes a java .properties file from a map.
///
/// We do NOT use the `java_properties` crate for writing: its `{:x}` escape
/// format drops the leading zero for characters below U+1000 (`\u446` instead
/// of `\u0446`), which Java's Properties loader rejects with "Malformed
/// \uxxxx encoding" and kills the server at boot. This writer follows
/// java.util.Properties.store: four-digit escapes, JDK-style key escaping.
pub fn write_properties(path: &Path, map: &HashMap<String, String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut out = std::io::BufWriter::new(file);
    // Deterministic order: sorted keys.
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    for key in keys {
        let value = map.get(key).cloned().unwrap_or_default();
        writeln!(
            out,
            "{}={}",
            escape_property(key, true),
            escape_property(&value, false)
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{read_properties, write_properties};
    use std::collections::HashMap;

    #[test]
    fn non_ascii_escapes_with_four_digits() {
        let mut map = HashMap::new();
        map.insert("motd".to_string(), "ццццц".to_string());
        let path =
            std::env::temp_dir().join(format!("mcsm-props-{}.properties", std::process::id()));
        write_properties(&path, &map).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        // Every \u escape must carry exactly four hex digits.
        assert!(
            text.contains("motd=\\u0446\\u0446\\u0446\\u0446\\u0446"),
            "bad escape: {text}"
        );
        // Round-trip through the reader.
        let back = read_properties(&path).unwrap();
        assert_eq!(back.get("motd").unwrap(), "ццццц");
        let _ = std::fs::remove_file(&path);
    }
}
