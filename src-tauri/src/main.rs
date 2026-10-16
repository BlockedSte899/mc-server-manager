#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround for WebKitGTK rendering glitches on some GPUs (see tauri-apps/tauri#9394).
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        fix_xkb_data_path();
    }
    mc_server_manager_lib::run();
}

/// Generic (distro-built) binaries — e.g. the AppImage — carry an Ubuntu
/// libxkbcommon that looks for its keymaps in /usr/share/X11/xkb, which does
/// not exist on NixOS. Point XKB_CONFIG_ROOT at the system's
/// xkeyboard-config data instead (must happen before GTK initializes).
#[cfg(target_os = "linux")]
fn fix_xkb_data_path() {
    use std::path::Path;
    if std::env::var_os("XKB_CONFIG_ROOT").is_some() {
        return;
    }
    let candidates = [
        "/run/current-system/sw/share/X11/xkb".to_string(), // NixOS system path
        {
            // Newest xkeyboard-config in the Nix store, if any.
            let mut best: Option<String> = None;
            if let Ok(entries) = std::fs::read_dir("/nix/store") {
                for e in entries.flatten() {
                    let name = e.file_name().to_string_lossy().to_string();
                    if name.starts_with("xkeyboard-config-") {
                        let p = e.path().join("share/X11/xkb");
                        if p.is_dir() {
                            let s = p.to_string_lossy().to_string();
                            if best.as_ref().is_none_or(|b: &String| s > *b) {
                                best = Some(s);
                            }
                        }
                    }
                }
            }
            best.unwrap_or_default()
        },
    ];
    for c in candidates {
        if !c.is_empty() && Path::new(&c).exists() {
            std::env::set_var("XKB_CONFIG_ROOT", c);
            break;
        }
    }
}
