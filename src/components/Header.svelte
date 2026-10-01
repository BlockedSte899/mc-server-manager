<script lang="ts">
  import { navigate, getPath } from "../lib/router.svelte";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { Server, Settings, DownloadCloud, Boxes, Minus, Square, X, Copy } from "lucide-svelte";
  import { t } from "../lib/i18n.svelte";
  import { onMount } from "svelte";

  const path = $derived(getPath());
  const appWindow = getCurrentWindow();

  let maximized = $state(false);
  let isTauri = $state(false);

  onMount(() => {
    (async () => {
      try {
        const m = await appWindow.isMaximized();
        maximized = m;
        isTauri = true;
        const un1 = await appWindow.onResized(() => {
          void appWindow.isMaximized().then((v) => (maximized = v));
        });
        return () => un1();
      } catch {
        isTauri = false;
      }
    })();
  });

  function isActive(prefix: string) {
    return path.startsWith(prefix);
  }
</script>

<header class="sticky top-0 z-40 border-b border-edge bg-surface-0">
  <div
    class="max-w-7xl mx-auto px-5 h-14 flex items-center gap-6"
    data-tauri-drag-region
  >
    <a
      class="flex items-center gap-2 cursor-pointer select-none"
      href="#/"
      onclick={(e) => {
        e.preventDefault();
        navigate("/");
      }}
    >
      <div
        class="w-8 h-8 rounded-lg bg-brand-500 border border-brand-400 flex items-center justify-center shadow-[0_0_12px_rgba(72,255,160,0.35)]"
        title={t("MC Server Manager logo")}
      >
        <Boxes size={18} class="text-white" />
      </div>
      <span class="font-bold tracking-tight text-slate-100">MC Server Manager</span>
    </a>

    <nav class="flex items-center gap-1 ml-4">
      <a
        href="#/"
        onclick={(e) => { e.preventDefault(); navigate("/"); }}
        class="px-3 py-1.5 rounded-lg text-sm cursor-pointer transition-colors {isActive('/') && path !== '/velocity' && path !== '/settings' && path !== '/import' ? 'bg-surface-2 text-white' : 'text-fg-dim hover:text-white'}">
        <span class="inline-flex items-center gap-2"><Server size={15} /> {t("Servers")}</span>
      </a>
      <a
        href="#/import"
        onclick={(e) => { e.preventDefault(); navigate("/import"); }}
        class="px-3 py-1.5 rounded-lg text-sm cursor-pointer transition-colors {isActive('/import') ? 'bg-surface-2 text-white' : 'text-fg-dim hover:text-white'}">
        <span class="inline-flex items-center gap-2"><DownloadCloud size={15} /> {t("Import")}</span>
      </a>
      <a
        href="#/settings"
        onclick={(e) => { e.preventDefault(); navigate("/settings"); }}
        class="px-3 py-1.5 rounded-lg text-sm cursor-pointer transition-colors {isActive('/settings') ? 'bg-surface-2 text-white' : 'text-fg-dim hover:text-white'}">
        <span class="inline-flex items-center gap-2"><Settings size={15} /> {t("Settings")}</span>
      </a>
    </nav>

    <div class="flex-1 self-stretch" data-tauri-drag-region></div>

    {#if isTauri}
      <div class="flex items-center gap-1">
        <button
          class="w-8 h-8 flex items-center justify-center rounded-md text-fg-dim hover:text-white hover:bg-surface-2 transition-colors"
          onclick={() => appWindow.minimize()}
          title={t("Minimize")}
          aria-label={t("Minimize window")}
        >
          <Minus size={15} />
        </button>
        <button
          class="w-8 h-8 flex items-center justify-center rounded-md text-fg-dim hover:text-white hover:bg-surface-2 transition-colors"
          onclick={() => appWindow.toggleMaximize()}
          title={maximized ? t("Restore") : t("Maximize")}
          aria-label={t("Toggle maximize")}
        >
          {#if maximized}
            <Copy size={13} />
          {:else}
            <Square size={13} />
          {/if}
        </button>
        <button
          class="w-8 h-8 flex items-center justify-center rounded-md text-fg-dim hover:text-white hover:bg-red-500/80 transition-colors"
          onclick={() => appWindow.close()}
          title={t("Close window")}
          aria-label={t("Close window")}
        >
          <X size={16} />
        </button>
      </div>
    {/if}
  </div>
</header>