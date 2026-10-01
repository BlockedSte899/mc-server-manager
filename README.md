# MC Server Manager

Desktop application for managing Minecraft servers, built with **Tauri v2**, **Svelte 5**,
**Tailwind CSS 4** and **Rust**. It runs servers, watches them, picks Java runtimes,
schedules restarts and backups, streams the console, and can stay resident in the system
tray.

## Features

- Multiple Minecraft (and Velocity) servers, each with its own settings
- Start / stop / restart with a configurable watchdog
- Java runtime discovery: `JAVA_HOME`, `PATH`, `/usr/lib/jvm`, Nix store, Prism Launcher
- Memory limits, JVM presets and custom arguments
- Scheduled start/stop, automatic startup and automatic backups
- Live console (xterm.js) with log search, file browsing and file management
- Worlds, mods, players and backups management
- Server version download and switching
- System tray mode with notifications for start, stop, crash and backup events
- Launch at login (OS autostart)

## Installation

### NixOS (flake)

The flake exposes `packages.<system>.default`, so the app can be installed system-wide or
per-user straight from your configuration.

Add it as an input — in your `flake.nix`:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    mc-server-manager = {
      url = "github:BlockedSte899/mc-server-manager";
      # pin to a release tag once you have one:
      # inputs.mc-server-manager.url = "github:BlockedSte899/mc-server-manager/v0.1.0";
    };
  };

  outputs = { nixpkgs, mc-server-manager, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        ({ ... }: {
          environment.systemPackages = [ mc-server-manager.packages.x86_64-linux.default ];
        })
      ];
    };
  };
}
```

Then rebuild:

```bash
sudo nixos-rebuild switch --flake
```

`mc-server-manager` is available as `mc-server-manager` afterwards:

```bash
mc-server-manager          # launch
mc-server-manager --help   # CLI options
```

To try it without adding it to your configuration:

```bash
nix run github:BlockedSte899/mc-server-manager
```

#### With Home Manager

```nix
home.packages = [ inputs.mc-server-manager.packages.${pkgs.system}.default ];
```

#### Non-NixOS Linux with flakes

```bash
# run without installing
nix run github:BlockedSte899/mc-server-manager

# or build the store path
nix build github:BlockedSte899/mc-server-manager
./result/bin/mc-server-manager
```

The package is Linux-only. Windows and macOS users should take the installers from
[Releases](https://github.com/BlockedSte899/mc-server-manager/releases).

#### What the flake packages

The Nix package wraps the binary so it works on a clean NixOS install:

- `GSETTINGS_SCHEMA_DIR` points at a merged, compiled bundle of GTK3 + GLib +
  desktop GSettings schemas, which GTK file dialogs require
- `LD_LIBRARY_PATH` includes `libayatana-appindicator`, which the tray icon `dlopen()`s
- `JAVA_HOME` and `PATH` include OpenJDK 17, so servers can be launched out of the box
  (override with the app's own Java settings)

Building the Nix package needs no network access; pnpm dependencies are fetched by a
fixed-output derivation, so the build is fully sandboxed and reproducible.

### AppImage

Grab the `.AppImage` from [Releases](https://github.com/BlockedSte899/mc-server-manager/releases):

```bash
chmod +x MC_Server_Manager_0.1.0_amd64.AppImage
./MC_Server_Manager_0.1.0_amd64.AppImage
```

The AppImage bundles its own libraries, so it runs on Ubuntu, Fedora, Arch and friends
without installing dependencies. Some distributions (Fedora, some Arch setups) require FUSE:

```bash
sudo apt install libfuse2      # Debian / Ubuntu
./MC_Server_Manager_0.1.0_amd64.AppImage --appimage-extract-and-run   # no FUSE
```

### .deb / .rpm

```bash
# Debian / Ubuntu
sudo apt install ./mc-server-manager_0.1.0_amd64.deb

# Fedora / openSUSE
sudo dnf install ./mc-server-manager-0.1.0-1.x86_64.rpm
```

### Windows

Download the NSIS installer (`MC Server Manager_0.1.0_x64-setup.exe`) from
[Releases](https://github.com/BlockedSte899/mc-server-manager/releases). It installs per
user, needs no separate runtime, and ships English and Russian. An MSI is provided too.

Windows users need a Java runtime to launch servers — either install Temurin/OpenJDK
yourself or point the app at a runtime from Settings → Java. The app also discovers
runtimes bundled with Prism Launcher and MultiMC.

## Development

```bash
nix develop          # dev shell with cargo, node, pnpm and the Tauri CLI
pnpm install
pnpm tauri dev
```

Other useful commands:

```bash
pnpm check           # svelte-check / TypeScript
nix develop --command cargo test --workspace
nix develop --command cargo clippy --workspace
```

Build release bundles (outside the Nix sandbox, because the AppImage bundler downloads
`linuxdeploy` at build time):

```bash
# Linux: appimage + deb + rpm
# Note: AppImage bundling resolves libraries through distro pkg-config, so it needs
# a real distro system (Ubuntu/Fedora) rather than a Nix store dev shell. deb and
# rpm build fine inside `nix develop`.
pnpm tauri build --bundles appimage deb rpm

# Windows (on windows-latest): nsis + msi
pnpm tauri build
```

Release binaries are produced by [`.github/workflows/build.yml`](.github/workflows/build.yml)
on tag pushes and are published to GitHub Releases automatically:

```bash
git tag v0.1.0
git push origin v0.1.0
```

## License

MIT
