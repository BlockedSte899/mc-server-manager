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
  let zoomMode: "webview" | "css" = $state("webview");

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
  // The factor is driven purely by the user's UI-scale setting. Any
  // "self-calibrating" correction is deliberately avoided: deriving a base
  // from window.innerWidth vs. the backend logical size breaks the moment the
  // window is resized without a scale change — logicalSize goes stale, the
  // bogus ratio sticks, and every applyZoom (e.g. after a theme switch)
  // re-applies it, shrinking the UI. The NixOS tiny-UI root cause (broken
  // xftDPI → garbage devicePixelRatio) is fixed at the source instead: the
  // flake points GSETTINGS_SCHEMA_DIR at the merged schemas directory.
  //
  // Linux scales via CSS `zoom`, which re-flows the layout at the new scale —
  // so the sticky header keeps working while scrolling (a transform would
  // break sticky and let the header slide down). Other platforms use the
  // native webview zoom, which is crisp and reliable there.
  function clamp(x: number, lo: number, hi: number) {
    return Math.min(hi, Math.max(lo, x));
  }

  /** GTK display scale factor reported by the backend (diagnostics only). */
  let systemScale = $state(1);
  /** True logical CSS size of the window (physical / GTK scale factor). */
  let logicalSize = $state({ width: 0, height: 0 });
  /** WebKitGTK's native zoom is unreliable on Linux (broken devicePixelRatio),
   *  so we scale via CSS there instead of webview.setZoom. */
  let isLinux = $state(false);

  async function detectEnvironment() {
    isLinux = /linux/i.test(navigator.platform || navigator.userAgent || "");
    try {
      const m = await miscApi.uiMetrics();
      systemScale = m.scale || 1;
      if (m.logical?.width > 0) {
        logicalSize.width = m.logical.width;
        logicalSize.height = m.logical.height;
      }
      // Diagnostics only — helps spot a broken WebKitGTK environment
      // (tiny/negative devicePixelRatio on NixOS without the schemas fix).
      // The zoom itself is driven purely by the user's UI-scale setting.
      void miscApi.uiLog(
        `env isLinux=${isLinux} scale=${systemScale} dpr=${window.devicePixelRatio || 1} ` +
          `inner=${window.innerWidth}x${window.innerHeight} ` +
          `logical=${Math.round(logicalSize.width)}x${Math.round(logicalSize.height)}`
      );
    } catch {
      systemScale = 1;
    }
  }

  async function applyZoom() {
    const factor = clamp((settings.ui_scale ?? 100) / 100, 0.5, 3);

    const wrap = document.getElementById("ui-root");
    const app = document.getElementById("app");
    if (!wrap) return;

    // Non-Linux: the native webview zoom is crisp and reliable, use it.
    if (!isLinux && zoomMode === "webview") {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        await getCurrentWebview().setZoom(factor);
        void miscApi.uiLog(
          `zoom webview factor=${factor} (ui_scale=${settings.ui_scale}%)`
        );
        return;
      } catch {
        zoomMode = "css";
        void miscApi.uiLog(`webview setZoom unavailable → css zoom`);
      }
    }

    // Linux / fallback: CSS `zoom` re-flows the layout at the new scale, so
    // the sticky header keeps sticking while the window scrolls (a transform
    // would break sticky and let the header slide down while scrolling).
    if (factor === 1) {
      wrap.style.zoom = "";
      if (app) app.style.overflow = "";
      return;
    }
    wrap.style.zoom = String(factor);
    if (app) app.style.overflow = "auto";
    void miscApi.uiLog(`zoom css factor=${factor} (ui_scale=${settings.ui_scale}%)`);
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