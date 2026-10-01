<script lang="ts">
  import { navigate } from "../lib/router.svelte";
  import { serversApi } from "../lib/api";
  import { statuses, toast, refreshServers, servers } from "../lib/store.svelte";
  import Button from "../lib/ui/Button.svelte";
  import Badge from "../lib/ui/Badge.svelte";
  import ConfirmDialog from "../lib/ui/ConfirmDialog.svelte";
  import { templateColor } from "../lib/types";
  import { ArrowLeft, Play, Square, RotateCw, Skull, Trash2 } from "lucide-svelte";
  import { onMount } from "svelte";
  import { t } from "../lib/i18n.svelte";

  import OverviewTab from "./tabs/OverviewTab.svelte";
  import LogsTab from "./tabs/LogsTab.svelte";
  import PlayersTab from "./tabs/PlayersTab.svelte";
  import ConfigTab from "./tabs/ConfigTab.svelte";
  import LaunchTab from "./tabs/LaunchTab.svelte";
  import ModsTab from "./tabs/ModsTab.svelte";
  import WorldsTab from "./tabs/WorldsTab.svelte";
  import FilesTab from "./tabs/FilesTab.svelte";
  import VelocityTab from "./tabs/VelocityTab.svelte";

  let id = $state("");
  let tab = $state("overview");
  let busy = $state(false);
  let confirmKill = $state(false);
  let confirmDelete = $state(false);

  function init() {
    const m = location.hash.match(/^#\/s\/([^/?]+)/);
    if (m) id = m[1];
  }
  init();

  const proxyTabs = [
    { key: "overview", label: "Overview" },
    { key: "launch", label: "Launch" },
    { key: "logs", label: "Logs" },
    { key: "config", label: "Config" },
    { key: "players", label: "Players" },
    { key: "files", label: "Files" },
  ];

  // Bukkit-family servers run plugins, mod loaders run mods, vanilla has neither.
  const PLUGIN_CORES = ["paper", "purpur", "spigot", "bukkit", "folia"];

  const baseGameTabs = [
    { key: "overview", label: "Overview" },
    { key: "launch", label: "Launch" },
    { key: "logs", label: "Logs" },
    { key: "players", label: "Players" },
    { key: "config", label: "Config" },
    { key: "worlds", label: "Worlds" },
    { key: "files", label: "Files" },
  ];

  const tabLabels: Record<string, string> = {
    overview: "Overview",
    launch: "Launch",
    logs: "Logs",
    players: "Players",
    config: "Config",
    worlds: "Worlds",
    files: "Files",
  };

  onMount(() => {
    document.documentElement.classList.add("dash-scroll-lock");
    refreshServers();
    return () => document.documentElement.classList.remove("dash-scroll-lock");
  });

  const meta = $derived(servers.find((s) => s.meta.id === id)?.meta ?? null);
  const isPluginCore = $derived(meta ? PLUGIN_CORES.includes(meta.core) : false);
  const visibleTabs = $derived.by(() => {
    if (!meta) return baseGameTabs;
    if (meta.is_proxy) return proxyTabs;
    const tabs = [...baseGameTabs];
    if (meta.core !== "vanilla") {
      tabs.splice(5, 0, { key: "mods", label: isPluginCore ? "Plugins" : "Mods" });
    }
    return tabs;
  });

  async function toggle() {
    if (busy) return;
    busy = true;
    try {
      const st = statuses[id!];
      if (st === "running" || st === "starting") await serversApi.stop(id!);
      else await serversApi.start(id!);
      await refreshServers();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      busy = false;
    }
  }

  async function doRestart() {
    try {
      await serversApi.restart(id!);
    } catch (e) {
      toast(t("Error restarting: {e}", { e: String(e) }), "error");
    }
  }

  async function doKill() {
    try {
      await serversApi.kill(id!);
      toast(t("Server killed"), "success");
    } catch (e) {
      toast(t("Error killing: {e}", { e: String(e) }), "error");
    }
  }

  async function doDelete() {
    try {
      await serversApi.delete(id!);
      toast(t("Server deleted"), "success");
      navigate("/");
    } catch (e) {
      toast(t("Error deleting: {e}", { e: String(e) }), "error");
    }
  }
</script>

{#if id && meta}
  <div class="max-w-7xl mx-auto px-5 py-6 flex flex-col" style="height: calc(100vh - 56px);">
    <div class="flex items-center justify-between mb-5 gap-3 flex-wrap">
      <div class="flex items-center gap-3 min-w-0">
        <Button
          variant="outline"
          size="icon"
          onclick={() => navigate("/")}
          title={t("Back to servers")}
          ariaLabel={t("Back to servers")}
        >
          <ArrowLeft size={18} />
        </Button>
        <div>
          <div class="flex items-center gap-2">
            <h1 class="text-xl font-bold text-slate-50 truncate">{meta.name}</h1>
            <Badge class={templateColor(meta.core)}>
              <span class="uppercase">{meta.core}</span>
            </Badge>
          </div>
          <p class="text-xs text-fg-dim">
            {#if !meta.is_proxy}{meta.mc_version}{/if}
            {#if meta.loader_version}
              · {meta.loader_version}
            {/if}
            {#if meta.build}
              · {t("build {build}", { build: meta.build })}
            {/if}
          </p>
        </div>
      </div>
      <div class="flex items-center gap-2 flex-wrap">
        <Button
          variant={statuses[id!] === "running" ? "subtle" : "primary"}
          size="sm"
          disabled={busy}
          onclick={toggle}
          title={statuses[id!] === "running" ? t("Stop the server") : t("Start the server")}
        >
          {#if statuses[id!] === "running" || statuses[id!] === "starting"}
            <Square size={14} /> {t("Stop")}
          {:else}
            <Play size={14} /> {t("Start")}
          {/if}
        </Button>
        <Button variant="outline" size="sm" onclick={doRestart} title={t("Gracefully restart the server")}>
          <RotateCw size={14} /> {t("Restart")}
        </Button>
        <Button variant="subtle" size="sm" onclick={() => (confirmKill = true)} title={t("Force-kill the process (no clean shutdown)")}>
          <Skull size={14} /> {t("Force kill")}
        </Button>
        <Button variant="danger" size="sm" onclick={() => (confirmDelete = true)} title={t("Delete this server and all its files")}>
          <Trash2 size={14} /> {t("Delete")}
        </Button>
      </div>
    </div>

    <div class="flex gap-1 border-b border-edge mb-5 overflow-x-auto">
      {#each visibleTabs as tb}
        <button
          class="px-3 py-2 text-sm cursor-pointer border-b-2 -mb-px transition-colors whitespace-nowrap {tab === tb.key ? 'border-brand-500 text-white font-medium' : 'border-transparent text-fg-dim hover:text-white'}"
          onclick={() => (tab = tb.key)}
        >
          {t(tabLabels[tb.key] ?? tb.label)}
        </button>
      {/each}
    </div>

    <div class="scroll-gutter flex-1 min-h-0 {tab === "files" || tab === "logs" ? "overflow-hidden" : "overflow-y-auto"}">
      {#if tab === "overview"}
        <OverviewTab {id} />
      {:else if tab === "logs"}
        <LogsTab {id} />
      {:else if tab === "launch"}
        <LaunchTab {id} />
      {:else if tab === "players"}
        <PlayersTab {id} isProxy={meta.is_proxy} />
      {:else if tab === "config" && !meta.is_proxy}
        <ConfigTab {id} />
      {:else if tab === "mods"}
        <ModsTab {id} />
      {:else if tab === "worlds"}
        <WorldsTab {id} />
      {:else if tab === "files"}
        <FilesTab {id} />
      {:else if tab === "config" && meta.is_proxy}
        <VelocityTab {id} />
      {/if}
    </div>
  </div>

  <ConfirmDialog
    title={t("Force kill server?")}
    message={t('Kill the process of "{name}" without a clean shutdown?\nRunning players may lose unsaved progress.', { name: meta.name })}
    confirmLabel={t("Force kill")}
    open={confirmKill}
    onConfirm={doKill}
    onClose={() => (confirmKill = false)}
  />

  <ConfirmDialog
    title={t("Delete server?")}
    message={isPluginCore
      ? t('Delete "{name}" and ALL of its files (worlds, configs, plugins)?\nThis cannot be undone.', { name: meta.name })
      : t('Delete "{name}" and ALL of its files (worlds, configs, mods)?\nThis cannot be undone.', { name: meta.name })}
    confirmLabel={t("Delete")}
    open={confirmDelete}
    onConfirm={doDelete}
    onClose={() => (confirmDelete = false)}
  />
{:else if id}
  <div class="max-w-7xl mx-auto px-5 py-12 text-center text-fg-dim text-sm">
    {t("Server not found.")}
    <button class="text-brand-500 underline cursor-pointer" onclick={() => navigate("/")}>{t("Go back")}</button>
  </div>
{/if}