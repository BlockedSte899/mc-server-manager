use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

use crate::AppState;

/// StatusNotifierItem-based tray used on Linux (Wayland).
///
/// Tauri's `tray-icon` on Linux is backed by libappindicator, which only
/// exposes a context menu: the host never forwards the status-notifier
/// `Activate` signal (left-click) to it, so a plain left-click on the tray
/// icon cannot open the window. `ksni` implements the protocol directly and
/// gives us the `activate`/`secondary_activate` hooks, so single-click can
/// show the window again.
///
/// The icon ([`Icon`], ARGB32 network byte order) comes from
/// `src-tauri/tray.argb` — a 64x64 render of the app logo. Regenerate it
/// together with the PNG icons if the logo ever changes.
pub struct McsmTray {
    pub app: AppHandle,
}

impl ksni::Tray for McsmTray {
    fn id(&self) -> String {
        "mc-server-manager".into()
    }

    fn title(&self) -> String {
        "MC Server Manager".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: 64,
            height: 64,
            data: include_bytes!("../tray.argb").to_vec(),
        }]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: "MC Server Manager".into(),
            ..Default::default()
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        crate::show_main_window(&self.app);
    }

    fn secondary_activate(&mut self, _x: i32, _y: i32) {
        crate::show_main_window(&self.app);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;

        let show = StandardItem {
            label: "Open MC Server Manager".into(),
            activate: Box::new(|this: &mut Self| crate::show_main_window(&this.app)),
            ..Default::default()
        };

        let quit = StandardItem {
            label: "Quit".into(),
            activate: Box::new(|this: &mut Self| {
                let app = this.app.clone();
                app.state::<AppState>()
                    .quitting
                    .store(true, Ordering::Relaxed);
                // Closing the window with `quitting` set runs the normal
                // shutdown path (graceful server stop on exit) on the main
                // thread — no blocking call is needed from this D-Bus thread.
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.close();
                } else {
                    app.exit(0);
                }
            }),
            ..Default::default()
        };

        vec![show.into(), ksni::MenuItem::Separator, quit.into()]
    }
}

/// Registers the tray icon and returns once the service thread is running.
/// Best-effort: if no StatusNotifierWatcher is available the thread quietly
/// fails and the rest of the app keeps working.
pub fn setup(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let service = ksni::TrayService::new(McsmTray { app: app.clone() });
    service.spawn();
    Ok(())
}
