<script lang="ts">
  import type { ServerListItem } from "../lib/types";
  import { statuses, metrics, fmtBytes } from "../lib/store.svelte";
  import { serversApi, propertiesApi } from "../lib/api";
  import { navigate } from "../lib/router.svelte";
  import { toast } from "../lib/store.svelte";
  import { isTauri } from "../lib/env";
  import { t, pl } from "../lib/i18n.svelte";
  import { onMount } from "svelte";
  import { Play, Square, Settings, TerminalSquare, RotateCw, Trash2, Users, Cpu, MemoryStick } from "lucide-svelte";
  import Button from "../lib/ui/Button.svelte";
  import ConfirmDialog from "../lib/ui/ConfirmDialog.svelte";

  let { server }: { server: ServerListItem } = $props();

  const statusColors: Record<string, string> = {
    running: "bg-emerald-500",
    starting: "bg-brand-500 animate-pulse",
    stopping: "bg-amber-400 animate-pulse",
    stopped: "bg-slate-600",
    error: "bg-red-500",
  };
  const statusLabels: Record<string, string> = {
    running: "Running",
    starting: "Starting…",
    stopping: "Stopping…",
    stopped: "Stopped",
    error: "Error",
  };
  const statusLabel = $derived(statusLabels[statuses[server.meta.id]] ?? "Stopped");

  let busy = $state(false);
  let confirmRestart = $state(false);
  let confirmDelete = $state(false);
  let icon = $state<string | null>(null);

  onMount(() => {
    if (!isTauri() || server.meta.is_proxy) return;
    void propertiesApi
      .iconGet(server.meta.id)
      .then((d) => (icon = d))
      .catch(() => (icon = null));
  });

  const m = $derived(metrics[server.meta.id]);
  const running = $derived(
    statuses[server.meta.id] === "running" || statuses[server.meta.id] === "starting"
  );

  function metricsMeta() {
    if (!running || !m) return { server_cpu: null, server_mem: null };
    return {
      server_cpu: m.cpu_percent != null ? m.cpu_percent : null,
      server_mem: m.mem_bytes != null ? m.mem_bytes : null,
    };
  }

  async function toggle() {
    if (busy) return;
    busy = true;
    try {
      const st = statuses[server.meta.id];
      if (st === "running" || st === "starting") {
        await serversApi.stop(server.meta.id);
      } else {
        await serversApi.start(server.meta.id);
      }
    } catch (e) {
      toast(String(e), "error");
    } finally {
      busy = false;
    }
  }

  async function doRestart() {
    try {
      await serversApi.restart(server.meta.id);
      toast(t("Restarting…"), "success");
    } catch (e) {
      toast(t("Error restarting: {e}", { e: String(e) }), "error");
    }
  }

  async function doDelete() {
    try {
      await serversApi.delete(server.meta.id);
      toast(t("Server deleted"), "success");
    } catch (e) {
      toast(t("Error deleting: {e}", { e: String(e) }), "error");
    }
  }
</script>

<div class="rounded-xl border border-edge bg-surface-1 hover:border-brand-500/40 transition-colors overflow-hidden">
  <button
    class="w-full text-left p-4 cursor-pointer"
    onclick={() => navigate(`/s/${server.meta.id}`)}
  >
    <div class="flex items-center justify-between">
      <div class="flex items-center gap-3 min-w-0">
        {#if icon}
          <img src={icon} alt="" class="w-9 h-9 rounded-lg bg-surface-2 border border-brand-500/20 object-cover shrink-0" loading="lazy" />
        {:else}
          <div class="w-9 h-9 rounded-lg bg-brand-500/10 border border-brand-500/20 flex items-center justify-center shrink-0 text-brand-500 font-bold">
            {server.meta.name.slice(0, 1).toUpperCase()}
          </div>
        {/if}
        <div class="min-w-0">
          <div class="font-semibold text-slate-100 truncate">{server.meta.name}</div>
          <div class="text-xs text-fg-dim truncate">
            {#if server.meta.is_proxy}
              velocity
            {/if}
            {#if !server.meta.is_proxy && server.meta.mc_version}
              mc {server.meta.mc_version}
            {/if}
            {#if server.meta.loader_version}
              · {server.meta.loader_version}
            {/if}
          </div>
        </div>
      </div>
      <span class="inline-flex items-center gap-1.5 shrink-0">
        <span class={`w-2 h-2 rounded-full ${statusColors[statuses[server.meta.id]] ?? "bg-slate-600"}`}></span>
        <span class="text-xs text-fg-dim">{t(statusLabel)}</span>
      </span>
    </div>

    <div class="flex items-center gap-4 mt-3 flex-wrap">
      <span class="inline-flex items-center gap-1.5 text-xs text-fg-dim" title={t("Players online")}>
        <Users size={13} />
        {#if running && server.players != null}
          {pl(server.players, ["{n} игрок", "{n} игрока", "{n} игроков"], "{n} online")}
        {:else}
          0 {t("online")}
        {/if}
      </span>
      <span class="inline-flex items-center gap-1.5 text-xs text-fg-dim" title={t("Server process CPU")}>
        <Cpu size={13} />
        {metricsMeta().server_cpu != null ? `${metricsMeta().server_cpu!.toFixed(1)}%` : "—"}
      </span>
      <span class="inline-flex items-center gap-1.5 text-xs text-fg-dim" title={t("Server process RAM")}>
        <MemoryStick size={13} />
        {metricsMeta().server_mem != null
          ? `${fmtBytes(metricsMeta().server_mem!)} / ${fmtBytes((server.meta.max_ram ?? 0) * 1024 * 1024)}`
          : "—"}
      </span>
    </div>
  </button>

  <div class="flex items-center gap-1 px-3 py-2 border-t border-edge bg-surface-2/50">
    <Button
      variant={statuses[server.meta.id] === "running" ? "subtle" : "primary"}
      size="sm"
      disabled={busy}
      onclick={toggle}
    >
      {#if statuses[server.meta.id] === "running" || statuses[server.meta.id] === "starting"}
        <Square size={13} /> {t("Stop")}
      {:else}
        <Play size={13} /> {t("Start")}
      {/if}
    </Button>
    <Button variant="ghost" size="sm" onclick={() => (confirmRestart = true)} title={t("Restart (graceful)")}>
      <RotateCw size={14} />
    </Button>
    <div class="flex-1"></div>
    <Button
      variant="ghost"
      size="sm"
      onclick={() => navigate(`/s/${server.meta.id}`)}
      title={t("Console")}
    >
      <TerminalSquare size={15} />
    </Button>
    <Button
      variant="ghost"
      size="sm"
      onclick={() => navigate(`/s/${server.meta.id}`)}
      title={t("Manage")}
    >
      <Settings size={15} />
    </Button>
    <Button variant="ghost" size="sm" onclick={() => (confirmDelete = true)} title={t("Delete server")} class="!text-red-400 hover:!text-red-300">
      <Trash2 size={14} />
    </Button>
  </div>
</div>

<ConfirmDialog
  title={t("Restart server?")}
  message={t('Gracefully restart "{name}"?', { name: server.meta.name })}
  confirmLabel={t("Restart")}
  danger={false}
  open={confirmRestart}
  onConfirm={doRestart}
  onClose={() => (confirmRestart = false)}
/>

<ConfirmDialog
  title={t("Delete server?")}
  message={t('Delete "{name}" and all of its files?\nThis cannot be undone.', { name: server.meta.name })}
  confirmLabel={t("Delete")}
  open={confirmDelete}
  onConfirm={doDelete}
  onClose={() => (confirmDelete = false)}
/>