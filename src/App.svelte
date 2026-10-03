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
      // Diagnostics only. `devicePixelRatio` is unreliable on WebKitGTK/NixOS
      // (can be tiny/negative), so it is NOT used to derive the zoom — instead
      // applyZoom compares window.innerWidth against the backend logical size.
      void miscApi.uiLog(
        `env isLinux=${isLinux} scale=${systemScale} dpr=${window.devicePixelRatio || 1} ` +
          `inner=${window.innerWidth}x${window.innerHeight} ` +
          `logical=${Math.round(logicalSize.width)}x${Math.round(logicalSize.height)}`
      );
    } catch {
      systemScale = 1;
    }
  }

  /**
   * How far WebKitGTK's effective scale is off from the intended one.
   *
   * The window's true logical CSS width is physical / GTK-scale (from the
   * backend). `window.innerWidth` is what the webview *thinks* it has, which is
   * inflated when devicePixelRatio is under-reported (the NixOS WebKitGTK bug):
   * a 1280-logical window may report innerWidth=2560. So innerWidth/logical is
   * exactly the correction needed — it is 1 on healthy displays (nothing to do)
   * and >1 when WebKitGTK failed to apply the display scale.
   */
  function computeBase(): number {
    if (logicalSize.width > 0 && window.innerWidth > 0) {
      const r = window.innerWidth / logicalSize.width;
      if (Number.isFinite(r) && r > 0.2 && r < 5) return r;
    }
    return 1;
  }

  async function applyZoom() {
    const user = clamp((settings.ui_scale ?? 100) / 100, 0.5, 3);
    const base = computeBase();
    const factor = clamp(user * base, 0.5, 3);

    const wrap = document.getElementById("ui-root");
    const app = document.getElementById("app");
    if (!wrap) return;

    // Non-Linux: the native webview zoom is crisp and reliable, use it.
    if (!isLinux && zoomMode === "webview") {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        await getCurrentWebview().setZoom(factor);
        void miscApi.uiLog(
          `zoom webview factor=${factor} (ui_scale=${settings.ui_scale}% base=${base} systemScale=${systemScale})`
        );
        return;
      } catch {
        zoomMode = "transform";
        void miscApi.uiLog(`webview setZoom unavailable → transform`);
      }
    }

    // Linux / fallback: scale the wrapping div so the whole UI fills the
    // window regardless of WebKitGTK's broken devicePixelRatio. We use a
    // transform + narrower width (rather than CSS `zoom`) so the scaled content
    // fits the window exactly instead of overflowing into scrollbars.
    if (factor === 1) {
      wrap.style.transform = "";
      wrap.style.transformOrigin = "";
      wrap.style.width = "";
      if (app) app.style.overflow = "";
      return;
    }
    wrap.style.transform = `scale(${factor})`;
    wrap.style.transformOrigin = "top left";
    wrap.style.width = `${100 / factor}%`;
    if (app) app.style.overflow = "auto";
    void miscApi.uiLog(
      `zoom css factor=${factor} (ui_scale=${settings.ui_scale}% base=${base} systemScale=${systemScale} logical=${Math.round(logicalSize.width)})`
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