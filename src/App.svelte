<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  import { getPath } from "./lib/router.svelte";
  import { setStatus, loadSettings, refreshServers, metrics, downloads, settings, windowHidden } from "./lib/store.svelte";
  import { miscApi } from "./lib/api";
  import { THEMES, CUSTOM_THEME_ID, DEFAULT_VARS, applyThemeVars } from "./lib/themes";
  import type { StatusEvent, ServerMetrics, DownloadProgress } from "./lib/types";

  const path = $derived(getPath());

  let zoomReady = $state(false);
  let zoomMode: "webview" | "transform" = $state("webview");

  $effect(() => {
    if (!zoomReady) return;
    const _scaleDep = settings.ui_scale; // keep zoom reactive to UI-scale changes
    const themeId = settings.theme || "dark";
    document.documentElement.dataset.theme = themeId;
    document.documentElement.lang = settings.lang || "en";
    const vars =
      themeId === CUSTOM_THEME_ID
        ? settings.custom_theme ?? DEFAULT_VARS
        : (THEMES.find((t) => t.id === themeId)?.vars ?? DEFAULT_VARS);
    applyThemeVars(vars, document.documentElement);
    void applyZoom();
  });

  // Zoom handling.
  //
  // GTK/WebKitGTK already apply the display scale factor and the
  // org.gnome.desktop.interface text-scaling-factor on their own, so the app
  // must NOT try to "correct" anything: a previous heuristic derived a scale
  // from outer_size()/inner_size(), but both of those report physical pixels,
  // so the ratio was always ~1.0 and the real factor was 1/devicePixelRatio.
  // On a display with dpr=2 that produced a 0.5 factor, which shrank the whole
  // UI and made the UI-scale setting unable to reach 100% again
  // (150% * 0.5 = 0.75). Zoom is now driven purely by the user's choice.
  function clamp(x: number, lo: number, hi: number) {
    return Math.min(hi, Math.max(lo, x));
  }

  /** GTK display scale factor reported by the backend (reliable on every
   *  platform). On healthy systems WebKitGTK already applies it and the page's
   *  devicePixelRatio matches it; on NixOS WebKitGTK sometimes fails to apply
   *  the display scale (broken GSettings -> bogus devicePixelRatio), which is
   *  what makes the whole UI render tiny there. We use this as the *reference*
   *  value to correct that, never as a multiplier on top of a working scale. */
  let systemScale = $state(1);

  async function detectEnvironment() {
    try {
      const m = await miscApi.uiMetrics();
      systemScale = m.scale || 1;
      // Diagnostics only. Note that WebKitGTK on some Wayland setups reports
      // nonsense here (devicePixelRatio came back as -1/96 and layout metrics
      // as huge/negative numbers), which is why zoom is never derived from
      // these values directly — see applyZoom below.
      void miscApi.uiLog(
        `env scale=${systemScale} dpr=${window.devicePixelRatio || 1} ` +
          `phys=${m.physical.width}x${m.physical.height} ` +
          `logical=${Math.round(m.logical.width)}x${Math.round(m.logical.height)}`
      );
    } catch {
      systemScale = 1;
    }
  }

  async function applyZoom() {
    const user = clamp((settings.ui_scale ?? 100) / 100, 0.5, 3);

    // Correct for WebKitGTK failing to apply the display scale on NixOS.
    //
    // On a healthy display `devicePixelRatio` equals the GTK scale factor and
    // WebKitGTK already scales by itself, so base = sys/dpr = 1 and we only
    // apply the user's UI-scale choice (this is what the old heuristic got
    // wrong: it used outer/inner size, both in physical pixels, so it read ~1
    // and ended up dividing by dpr -> a 0.5 shrink on every HiDPI display).
    //
    // When WebKitGTK under-scales (NixOS: dpr is reported too low or even
    // negative while the GTK scale factor is correct), base > 1 and we zoom up
    // to compensate. The ratio is clamped so a pathological dpr can never blow
    // the UI up.
    const dpr = window.devicePixelRatio || 1;
    let base = 1;
    if (systemScale > 0 && Number.isFinite(systemScale) && Number.isFinite(dpr) && dpr > 0) {
      base = systemScale / dpr;
    }
    const factor = clamp(user * clamp(base, 0.5, 3), 0.5, 3);

    const wrap = document.getElementById("ui-root");
    const app = document.getElementById("app");
    if (zoomMode === "webview") {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        await getCurrentWebview().setZoom(factor);
        void miscApi.uiLog(
          `zoom webview factor=${factor} (ui_scale=${settings.ui_scale}% base=${base} systemScale=${systemScale} dpr=${dpr})`
        );
        return;
      } catch {
        zoomMode = "transform";
        void miscApi.uiLog(`webview setZoom unavailable → transform`);
      }
    }
    // Fallback: scale the wrapping div (blurry, but works everywhere).
    if (!wrap || !app) return;
    if (factor === 1) {
      wrap.style.transform = "";
      wrap.style.transformOrigin = "";
      wrap.style.width = "";
      app.style.overflow = "";
      return;
    }
    wrap.style.transform = `scale(${factor})`;
    wrap.style.transformOrigin = "top left";
    wrap.style.width = `${100 / factor}%`;
    app.style.overflow = "auto";
    void miscApi.uiLog(
      `zoom transform factor=${factor} (ui_scale=${settings.ui_scale}% base=${base} systemScale=${systemScale} dpr=${dpr})`
    );
  }

  import Header from "./components/Header.svelte";
  import Toasts from "./components/Toasts.svelte";
  import ServersPage from "./pages/ServersPage.svelte";
  import WizardPage from "./pages/WizardPage.svelte";
  import ServerPage from "./pages/ServerPage.svelte";
  import ImportPage from "./pages/ImportPage.svelte";
  import SettingsPage from "./pages/SettingsPage.svelte";

  let unlisteners: UnlistenFn[] = [];

  function isServerPath(p: string) {
    return /^\/s\/[^/]+/.test(p);
  }

  onMount(() => {
    loadSettings();
    refreshServers();
    const int = setInterval(() => {
      if (!windowHidden()) void refreshServers();
    }, 6000);

    let disposed = false;
    (async () => {
      try {
        await detectEnvironment();
        const { getCurrentWindow } = await import("@tauri-apps/api/window");
        const un = await getCurrentWindow().onScaleChanged(() => {
          void detectEnvironment().then(() => applyZoom());
        });
        if (disposed) un();
        else unlisteners.push(un);
      } catch {
        /* non-tauri (plain browser) */
      }
      zoomReady = true;
    })();

    (async () => {
      const l1 = await listen<StatusEvent>("server-status", (ev) => {
        setStatus(ev.payload.id, ev.payload.state);
      });
      const l2 = await listen<ServerMetrics>("server-metrics", (ev) => {
        metrics[ev.payload.id] = ev.payload;
      });
      const l3 = await listen<DownloadProgress>("download:progress", (ev) => {
        downloads[ev.payload.key] = ev.payload;
      });
      if (disposed) {
        l1();
        l2();
        l3();
      } else {
        unlisteners.push(l1, l2, l3);
      }
    })();

    return () => {
      disposed = true;
      clearInterval(int);
      unlisteners.forEach((u) => u());
    };
  });
</script>

<div id="ui-root">
  <Header />

  <main class="min-h-[calc(100vh-56px)]">
    {#if path === "/" || path === ""}
      <ServersPage />
    {:else if path === "/wizard" || path?.startsWith("/wizard")}
      <WizardPage />
    {:else if isServerPath(path ?? "")}
      <ServerPage />
    {:else if path === "/import"}
      <ImportPage />
    {:else if path === "/settings"}
      <SettingsPage />
    {:else}
      <ServersPage />
    {/if}
  </main>

  <Toasts />
</div>