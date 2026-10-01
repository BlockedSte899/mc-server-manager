/// CREATE_NO_WINDOW — instructs Windows to spawn the child without allocating a
/// console window. Without it every `Command::new` flashes a cmd.exe/PowerShell
/// window on Windows, which is very visible for the short-lived processes this
/// app spawns (java -version probes, installers, server start/stop).
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Applies the Windows-specific creation flags to a command.
///
/// On non-Windows platforms this is a no-op, so call sites stay portable.
#[cfg(windows)]
pub fn hide_console(cmd: &mut tokio::process::Command) -> &mut tokio::process::Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(windows))]
pub fn hide_console(cmd: &mut tokio::process::Command) -> &mut tokio::process::Command {
    cmd
}
