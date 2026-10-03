{
  description = "MC Server Manager — Tauri v2 desktop app for managing Minecraft servers";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        lib = pkgs.lib;

        # Runtime + build dependencies shared by the dev shell and the package.
        tauriDeps = with pkgs; [
          glib
          gtk3
          webkitgtk_4_1
          libsoup_3
          cairo
          pango
          gdk-pixbuf
          librsvg
          openssl
          dbus
          at-spi2-core
          alsa-lib
          libxkbcommon
          libx11
          libxcursor
          libxi
          libxrandr
          libxcb
        ];

        version = "1.0.17";

        # GSettings schemas that GTK/WebKitGTK need at runtime. nixpkgs nests
        # each package's schemas under share/gsettings-schemas/<name>/…, which
        # GLib's XDG scan does not pick up, and GSETTINGS_SCHEMA_DIR accepts a
        # single extra schema *directory* (it appends /gschemas.compiled itself)
        # — so we merge the XML into one directory and point the env var at it.
        #
        # Getting this right is what fixes the NixOS "tiny UI" bug: without the
        # schemas, WebKitGTK reads the xftDPI = -1 sentinel and derives a
        # garbage negative devicePixelRatio (-1/96), which renders the whole UI
        # microscopic. With the directory set, dpr is a healthy 1 and the UI
        # renders at its natural scale (verified empirically via ui-log).
        gsettingsBundle = pkgs.runCommand "mcsm-gsettings-bundle" { } ''
          mkdir -p "$out/share/glib-2.0/schemas"
          for base in ${pkgs.gtk3} ${pkgs.gsettings-desktop-schemas} ${pkgs.glib}; do
            for xml in "$base"/share/gsettings-schemas/*/glib-2.0/schemas/*.xml; do
              [ -e "$xml" ] || continue
              ln -sfn "$xml" "$out/share/glib-2.0/schemas/$(basename "$xml")"
            done
          done
          ${pkgs.glib.dev}/bin/glib-compile-schemas "$out/share/glib-2.0/schemas"
        '';

        # The Tauri binary itself. Bundling (AppImage/deb/rpm) is deliberately
        # NOT done here: the AppImage bundler downloads linuxdeploy at build
        # time, which the Nix sandbox forbids. Release bundles are produced
        # outside the sandbox by `pnpm tauri build` (see README).
        #
        # `java` is a parameter rather than a fixed dependency: a full JDK adds
        # ~770 MiB to the runtime closure, which is half of the download. The app
        # detects Java itself (settings screen, PATH, JAVA_HOME, Nix store), so
        # the default package ships without one — anyone who wants it bundled
        # uses the `withJava` variant below.
        mcsmFor = java: pkgs.rustPlatform.buildRustPackage {
          pname = "mc-server-manager";
          inherit version;

          src = lib.cleanSourceWith {
            src = self;
            filter =
              path: type:
              let
                base = baseNameOf (toString path);
              in
              !(lib.hasPrefix "." base && base != ".github")
              && base != "node_modules"
              && base != "target"
              && base != "dist"
              && base != "result";
          };

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.nodejs_22
            pkgs.pnpm_10
            pkgs.sqlite
            pkgs.pnpmConfigHook
            pkgs.makeWrapper
          ];

          # libayatana-appindicator is dlopen'd at runtime by the tray icon, so
          # it has to stay reachable through RPATH at run time as well.
          buildInputs = tauriDeps ++ [ pkgs.libayatana-appindicator ];

          # The Svelte frontend is embedded into the binary by tauri-build, so it
          # has to exist at compile time. pnpm deps come from a fixed-output
          # derivation (network allowed only there); pnpmConfigHook unpacks that
          # store and runs the install offline.
          pnpmDeps = pkgs.fetchPnpmDeps {
            pname = "mc-server-manager";
            inherit version;
            src = ./.;
            pnpm = pkgs.pnpm_10;
            fetcherVersion = 4;
            hash = "sha256-imw/8HkNbQmngxgDDiH4ThpmGSvb8NMBwbFdMNf1vDI=";
          };

          preBuild = ''
            # The install hook skips package lifecycle scripts; esbuild's binary
            # is provisioned by its own install script, so run just that one.
            pnpm rebuild esbuild
            pnpm build
          '';

          buildFeatures = [ "custom-protocol" ];

          # nixpkgs' automatic RPATH patching handles the gtk/webkit link set;
          # the tray icon plugin dlopen()s libayatana-appindicator instead of
          # linking it, so that one is exposed via LD_LIBRARY_PATH below.

          postFixup = ''
            wrapProgram $out/bin/mc-server-manager \
              --prefix LD_LIBRARY_PATH : "${pkgs.libayatana-appindicator}/lib" \
              --set GSETTINGS_SCHEMA_DIR "${gsettingsBundle}/share/glib-2.0/schemas" \
              ${lib.optionalString (java != null) ''
                --prefix PATH : "${java}/bin" \
                --set JAVA_HOME "${java}"
              ''}

            # XDG desktop entry + icons so the app shows up in Wayland
            # launchers (niri + noctalia-shell, GNOME, KDE, …). The launcher
            # discovers it through XDG_DATA_DIRS ($out/share/applications is
            # picked up when the package is installed into a profile).
            mkdir -p $out/share/applications $out/share/icons/hicolor/{32x32,128x128,256x256,512x512}/apps
            install -Dm644 ${./src-tauri/icons/32x32.png} $out/share/icons/hicolor/32x32/apps/mc-server-manager.png
            install -Dm644 ${./src-tauri/icons/128x128.png} $out/share/icons/hicolor/128x128/apps/mc-server-manager.png
            install -Dm644 ${./src-tauri/icons/256x256.png} $out/share/icons/hicolor/256x256/apps/mc-server-manager.png
            install -Dm644 ${./src-tauri/icons/512x512.png} $out/share/icons/hicolor/512x512/apps/mc-server-manager.png
            install -Dm644 ${./src-tauri/icons/1024x1024.png} $out/share/icons/hicolor/1024x1024/apps/mc-server-manager.png
            cat > $out/share/applications/mc-server-manager.desktop <<EOF
[Desktop Entry]
Type=Application
Version=1.0
Name=MC Server Manager
Comment=Manage Minecraft servers: start, stop, console, mods, worlds, backups
Exec=mc-server-manager
Icon=mc-server-manager
Terminal=false
Categories=Game;
Keywords=minecraft;server;manager;
StartupWMClass=dev.mcsm.manager
EOF
          '';

          meta = {
            description = "MC Server Manager — Tauri v2 desktop app for managing Minecraft servers";
            longDescription = ''
              Desktop application for managing Minecraft servers: start, stop and
              watch servers, pick Java runtimes, schedule restarts and backups,
              browse worlds/mods/logs and watch the console — with a tray mode
              for background operation.
            '';
            homepage = "https://github.com/BlockedSte899/mc-server-manager";
            license = lib.licenses.mit;
            mainProgram = "mc-server-manager";
            platforms = lib.platforms.linux;
          };
        };

        # Default: no bundled JDK, ~1/3 smaller download and closure.
        mcsm = mcsmFor null;

        # Opt-in: ships an OpenJDK 17 and puts it on PATH for the app.
        mcsmWithJava = mcsmFor pkgs.openjdk17;
      in
      {
        packages = {
          default = mcsm;
          mc-server-manager = mcsm;
          withJava = mcsmWithJava;
        };

        apps.default = {
          type = "app";
          program = "${mcsm}/bin/mc-server-manager";
        };

        devShells.default = pkgs.mkShell {
          name = "mc-server-manager-dev";
          inputsFrom = [ mcsm ];
          buildInputs = tauriDeps ++ (with pkgs; [
            clippy
            rustfmt
            cargo-tauri
            python3
          ]);
          nativeBuildInputs = with pkgs; [
            pkg-config
            rustc
            cargo
            nodejs_22
            pnpm_10
          ];
          shellHook = ''
            export RUST_BACKTRACE=1
            export GSETTINGS_SCHEMA_DIR="${gsettingsBundle}/share/glib-2.0/schemas"
            export GTK_IM_MODULE=gtk-im-context-simple
            # System-tray icon: libappindicator is dlopen'd at runtime by
            # tray-icon, so its .so must be findable via LD_LIBRARY_PATH.
            export LD_LIBRARY_PATH="${pkgs.libayatana-appindicator}/lib''${LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}"
            echo "mc-server-manager devShell ready (cargo + node + tauri CLI)"
          '';
        };

        checks = {
          build = mcsm;
        };

        formatter = pkgs.nixpkgs-fmt;
      }
    );
}
