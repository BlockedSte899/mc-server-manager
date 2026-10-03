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
      # inputs.mc-server-manager.url = "github:BlockedSte899/mc-server-manager/v1.0.20";
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

Building the Nix package needs no network access; pnpm dependencies are fetched by a
fixed-output derivation, so the build is fully sandboxed and reproducible.

#### Java is not bundled

The default package deliberately ships **without** a JDK: a full OpenJDK adds roughly
770 MiB to the runtime closure, which is about a third of the total download. The app
finds Java itself — check Settings → Java, which scans `JAVA_HOME`, `PATH`,
`/usr/lib/jvm`, the Nix store and Prism Launcher.

If you would rather have it bundled (and are happy to pay the download), use the
`withJava` variant, which puts OpenJDK 17 on `PATH`:

```nix
environment.systemPackages = [ inputs.mc-server-manager.packages.${pkgs.system}.withJava ];
```

#### First build vs. binary cache

Until a cache is configured, `nix build` compiles the whole Rust/Tauri app from
source, which takes roughly ten minutes on a normal machine. To download a ready
binary instead, add the project's binary cache as a substituter:

```nix
# /etc/nix/nix.conf  (or nix.settings.substituters in NixOS)
substituters = https://cache.nixos.org https://mc-server-manager.cachix.org
trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY= mc-server-manager.cachix.org-1:<key from the cache>
```

then verify and build:

```bash
nix store verify --check-contents --repair   # only if needed
nix build github:BlockedSte899/mc-server-manager
```

CI publishes every build to that cache, so once a release has been through CI the
package is fetched instead of compiled.

### AppImage

Grab the `.AppImage` from [Releases](https://github.com/BlockedSte899/mc-server-manager/releases):

```bash
chmod +x MC_Server_Manager_1.0.20_amd64.AppImage
./MC_Server_Manager_1.0.20_amd64.AppImage
```

The AppImage bundles its own libraries, so it runs on Ubuntu, Fedora, Arch and friends
without installing dependencies. Some distributions (Fedora, some Arch setups) require FUSE:

```bash
sudo apt install libfuse2      # Debian / Ubuntu
./MC_Server_Manager_1.0.20_amd64.AppImage --appimage-extract-and-run   # no FUSE
```

### .deb / .rpm

```bash
# Debian / Ubuntu
sudo apt install ./mc-server-manager_1.0.20_amd64.deb

# Fedora / openSUSE
sudo dnf install ./mc-server-manager-1.0.20-1.x86_64.rpm
```

### Windows

Download the NSIS installer (`MC Server Manager_1.0.20_x64-setup.exe`) from
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
git tag v1.0.20
git push origin v1.0.20
```

## License

MIT
