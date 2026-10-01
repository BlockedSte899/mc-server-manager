use crate::settings::Settings;

/// Which events fire a notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Start,
    Stop,
    Crash,
    Backup,
}

impl Event {
    fn title(self) -> &'static str {
        match self {
            Event::Start => "Server started",
            Event::Stop => "Server stopped",
            Event::Crash => "Server crashed",
            Event::Backup => "Auto-backup finished",
        }
    }
}

fn enabled(settings: &Settings, event: Event) -> bool {
    match event {
        Event::Start => settings.notify_on_start,
        Event::Stop => settings.notify_on_stop,
        Event::Crash => settings.notify_on_crash,
        Event::Backup => settings.notify_on_backup,
    }
}

/// Shows a desktop notification through the system daemon. Best-effort: a
/// missing D-Bus session must never disturb the app, so failures are ignored.
pub async fn notify(settings: &Settings, server: &str, event: Event) {
    if !settings.notify_desktop || !enabled(settings, event) {
        return;
    }
    let title = event.title().to_string();
    let body = server.to_string();
    // libnotify is a blocking FFI call, so keep it off the async runtime.
    let _ = tokio::task::spawn_blocking(move || {
        let _ = notify_rust::Notification::new()
            .summary(&title)
            .body(&body)
            .appname("MC Server Manager")
            .show();
    })
    .await;
}
