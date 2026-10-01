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
  let baseCorrection = $state(1);

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

  // WebKitGTK often fails to honour display scaling (tiny UI on HiDPI / scaled
  // desktops). We detect the mismatch between the GTK window scale and the
  // webview's own devicePixelRatio and compensate automatically. Zoom is applied
  // with the webview's native zoom-level (crisp re-layout), transforming the
  // #ui-root wrapper as a last-resort fallback.
  function clamp(x: number, lo: number, hi: number) {
    return Math.min(hi, Math.max(lo, x));
  }

  async function detectEnvironment() {
    try {
      const m = await miscApi.uiMetrics();
      const gtkScale = m.inner.width > 0 ? m.physical.width / m.inner.width : 1;
      let dpr = window.devicePixelRatio || 1;
      // WebKitGTK (in some Wayland/fractional-scale setups) reports a bogus
      // devicePixelRatio (e.g. -1/96). Anything outside a sane range is treated
      // as 1 so the automatic correction never distorts the UI.
      if (!Number.isFinite(dpr) || dpr < 0.2 || dpr > 8) dpr = 1;
      const corr = gtkScale / dpr;
      baseCorrection = Number.isFinite(corr) ? clamp(corr, 0.5, 2) : 1;
      void miscApi.uiLog(
        `env dpr=${dpr} gtk=${m.scale} phys=${m.physical.width}x${m.physical.height} ` +
          `inner=${m.inner.width}x${m.inner.height} corr=${corr.toFixed(3)}`
      );
    } catch {
      baseCorrection = 1;
    }
  }

  async function applyZoom() {
    const user = clamp((settings.ui_scale ?? 100) / 100, 0.4, 3);
    const factor = clamp(user * baseCorrection, 0.4, 3);
    const wrap = document.getElementById("ui-root");
    const app = document.getElementById("app");
    if (zoomMode === "webview") {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        await getCurrentWebview().setZoom(factor);
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