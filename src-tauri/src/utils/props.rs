use java_properties::{read, write};
use std::collections::HashMap;
use std::path::Path;

/// Reads a java .properties file into a map. Missing/empty files produce an empty map.
pub fn read_properties(path: &Path) -> Result<HashMap<String, String>, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = std::io::BufReader::new(file);
    let map = read(reader).map_err(|e| e.to_string())?;
    Ok(map)
}

/// Writes a java .properties file from a map.
pub fn write_properties(path: &Path, map: &HashMap<String, String>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let writer = std::io::BufWriter::new(file);
    write(writer, map).map_err(|e| e.to_string())
}
