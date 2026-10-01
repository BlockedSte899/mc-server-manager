pub mod cfzip;
pub mod loader;
pub mod mrpack;

use crate::server::manager::ServerMeta;
use crate::settings::Settings;
use tauri::AppHandle;

/// Options for importing a .mrpack (Modrinth) or .zip (CurseForge) pack.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ImportRequest {
    pub path: String,
    pub name: Option<String>,
    pub java: u32,
    pub min_ram: i64,
    pub max_ram: i64,
    pub accept_eula: bool,
}

pub async fn import(
    app: &AppHandle,
    settings: &Settings,
    req: ImportRequest,
) -> Result<ServerMeta, String> {
    let path = std::path::Path::new(&req.path);
    if !path.exists() {
        return Err("pack file not found".into());
    }
    if !req.accept_eula {
        return Err("You must accept the Minecraft EULA first".into());
    }
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("invalid archive: {e}"))?;
    let has_mr = archive.by_name("modrinth.index.json").is_ok();
    let has_mf = archive.by_name("manifest.json").is_ok();
    drop(archive);

    if has_mr && !has_mf {
        mrpack::import(app, settings, req).await
    } else if has_mf {
        cfzip::import(app, settings, req).await
    } else {
        Err(
            "Unrecognized pack format: expected a Modrinth (.mrpack) or CurseForge (.zip) archive"
                .into(),
        )
    }
}
