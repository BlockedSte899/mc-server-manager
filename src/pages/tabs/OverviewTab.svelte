<script lang="ts">
  import { serversApi, settingsApi } from "../../lib/api";
  import { statuses, metrics, fmtBytes, fmtDuration, toast, servers, refreshServers, patchServerMeta, windowHidden } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Badge from "../../lib/ui/Badge.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Dialog from "../../lib/ui/Dialog.svelte";
  import Spinner from "../../lib/ui/Spinner.svelte";
  import Console from "../../components/Console.svelte";
  import { t } from "../../lib/i18n.svelte";
  import { onMount } from "svelte";
  import type { ServerPing, JavaInstall } from "../../lib/types";
  import { Users, Cpu, MemoryStick, Clock, Coins, Check, X } from "lucide-svelte";

  let { id }: { id: string } = $props();

  let uptime = $state<number | null>(null);
  let ping: ServerPing | null = $state(null);
  let detected: JavaInstall[] = $state([]);
  let detectedLoading = $state(false);
  let javaOpen = $state(false);
  let resolving = $state(false);
  let appliedPath = $state<string | null>(null);

  onMount(() => {
    void refreshResolved();
    const poll = setInterval(async () => {
      if (windowHidden()) return;
      try {
        const s = await serversApi.status(id);
        uptime = s.uptime_secs;
        if (s.state === "running") {
          ping = await serversApi.ping(id);
        } else {
          ping = null;
        }
      } catch {
        /* ignore */
      }
    }, 3000);
    return () => clearInterval(poll);
  });

  const m = $derived(metrics[id]);
  const meta = $derived(servers.find((s) => s.meta.id === id)?.meta ?? null);

  let running = $derived(
    statuses[id] === "running" || statuses[id] === "starting" || statuses[id] === "stopping"
  );

  const statusLabel = $derived(
    statuses[id] === "running"
      ? "Running"
      : statuses[id] === "starting"
        ? "Starting…"
        : statuses[id] === "stopping"
          ? "Stopping…"
          : statuses[id] === "error"
            ? "Error"
            : "Stopped"
  );

  const statusColors: Record<string, string> = {
    running: "bg-emerald-500",
    starting: "bg-brand-500 animate-pulse",
    stopping: "bg-amber-400 animate-pulse",
    stopped: "bg-slate-600",
    error: "bg-red-500",
  };

  const memSysPct = $derived(
    running && m && m.system_mem_total > 0 ? Math.min(100, (m.system_mem_used / m.system_mem_total) * 100) : 0
  );
  const heapLimitBytes = $derived((meta?.max_ram ?? 0) * 1024 * 1024);
  const memServerPct = $derived(
    running && m && m.mem_bytes != null
      ? Math.min(100, (m.mem_bytes / Math.max(1, heapLimitBytes)) * 100)
      : 0
  );

  const currentChoice = $derived(
    meta?.java_path
      ? t("Custom path: {path}", { path: meta.java_path })
      : meta
        ? t("Auto (Java {major})", { major: meta.java })
        : ""
  );

  async function openJava() {
    javaOpen = true;
    detected = [];
    detectedLoading = true;
    try {
      const list = await settingsApi.detectJava();
      list.sort((a, b) => a.major - b.major);
      detected = list;
    } catch (e) {
      detected = [];
      toast(t("Error loading Java list: {e}", { e: String(e) }), "error");
    } finally {
      detectedLoading = false;
    }
    void refreshResolved();
  }

  async function refreshResolved() {
    resolving = true;
    try {
      appliedPath = await serversApi.resolveJava(id);
    } catch {
      appliedPath = null;
    } finally {
      resolving = false;
    }
  }

  async function setJava(path: string | null) {
    try {
      const updated = await serversApi.setJava(id, path);
      patchServerMeta(id, updated);
      void refreshServers();
      if (!path) {
        appliedPath = null;
        try {
          appliedPath = await serversApi.resolveJava(id);
        } catch {
          /* ignore */
        }
      } else {
        appliedPath = path;
        try {
          appliedPath = await serversApi.resolveJava(id);
        } catch {
          /* ignore */
        }
      }
      toast(t("Java updated"), "success");
      javaOpen = false;
    } catch (e) {
      toast(t("Error setting Java: {e}", { e: String(e) }), "error");
    }
  }
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-4">
  <Card title={t("Status")} subtitle={t("Server state, uptime and players")}>
    <div class="flex items-center justify-between mb-3">
      <div class="flex items-center gap-2">
        <span class={`w-2 h-2 rounded-full ${statusColors[statuses[id]] ?? "bg-slate-600"}`}></span>
        <span class="font-medium text-green-400 text-sm">{t(statusLabel)}</span>
      </div>
      {#if running}
        <span class="inline-flex items-center gap-1.5 text-xs text-fg-dim">
          <Clock size={13} /> {t("Up {dur}", { dur: fmtDuration(uptime) })}
        </span>
      {/if}
    </div>
    {#if running}
      <div class="flex items-center gap-2 text-sm text-slate-100 mb-2">
        <Users size={15} class="text-fg-dim" />
        <span>{ping ? t("{n} players online", { n: ping.online ?? 0 }) : "…"}</span>
      </div>
      {#if ping?.version_name}
        <Badge class="bg-surface-2 text-slate-200">{ping.version_name}</Badge>
      {/if}
    {:else}
      <p class="text-sm text-fg-dim">{t("Server is offline. Press Start to launch it.")}</p>
    {/if}
  </Card>

  <Card title={t("Memory")} subtitle={t("Used vs heap limit, then system RAM")}>
    <div class="flex flex-col gap-3">
      <div class="flex items-center justify-between text-sm">
        <span class="inline-flex items-center gap-1.5 text-fg-dim"><MemoryStick size={14} /> {t("Server (RSS)")}</span>
        <span class="text-slate-100 font-medium">
          {running && m != null && m.mem_bytes != null
            ? `${fmtBytes(m.mem_bytes)} / ${fmtBytes(heapLimitBytes)}`
            : `— / ${fmtBytes(heapLimitBytes)}`}
        </span>
      </div>
      <div class="h-1.5 rounded-full bg-surface-3/60 overflow-hidden">
        <div class="h-full rounded-full bg-brand-500" style="width:{memServerPct}%"></div>
      </div>
      <div class="flex items-center justify-between text-sm">
        <span class="inline-flex items-center gap-1.5 text-fg-dim"><MemoryStick size={14} /> {t("System")}</span>
        <span class="text-slate-100 font-medium">{running && m ? `${fmtBytes(m.system_mem_used)} / ${fmtBytes(m.system_mem_total)}` : "—"}</span>
      </div>
      <div class="h-1.5 rounded-full bg-surface-3/60 overflow-hidden">
        <div class="h-full rounded-full bg-accent" style="width:{memSysPct}%"></div>
      </div>
    </div>
  </Card>

  <Card title={t("CPU")} subtitle={t("Server process vs system")}>
    <div class="flex flex-col gap-3">
      <div class="flex items-center justify-between text-sm">
        <span class="inline-flex items-center gap-1.5 text-fg-dim"><Cpu size={14} /> {t("Server")}</span>
        <span class="text-slate-100 font-medium">{running && m != null && m.cpu_percent != null ? `${m.cpu_percent.toFixed(1)}%` : "—"}</span>
      </div>
      <div class="h-1.5 rounded-full bg-surface-3/60 overflow-hidden">
        <div class="h-full rounded-full bg-brand-500" style="width:{running ? Math.min(100, m?.cpu_percent ?? 0) : 0}%"></div>
      </div>
      <div class="flex items-center justify-between text-sm">
        <span class="inline-flex items-center gap-1.5 text-fg-dim"><Cpu size={14} /> {t("System")}</span>
        <span class="text-slate-100 font-medium">{running && m ? `${m.system_cpu_percent.toFixed(1)}%` : "—"}</span>
      </div>
      <div class="h-1.5 rounded-full bg-surface-3/60 overflow-hidden">
        <div class="h-full rounded-full bg-accent" style="width:{running ? Math.min(100, m?.system_cpu_percent ?? 0) : 0}%"></div>
      </div>
    </div>
  </Card>

  <Card title={t("Java")} subtitle={t("Java runtime used by this server")}>
    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-between gap-2">
        <span class="inline-flex items-center gap-1.5 text-sm text-slate-100 font-medium">
          <Coins size={14} class="text-fg-dim" /> {currentChoice}
        </span>
        <Button variant="outline" size="xs" onclick={openJava}>{t("Change")}</Button>
      </div>
      <div class="text-xs text-fg-dim break-all">
        {#if resolving}
          <Spinner class="w-3 h-3 inline" />
        {:else if appliedPath}
          {appliedPath}
        {:else}
          {t("Could not resolve")}
        {/if}
      </div>
    </div>
  </Card>
</div>

{#if javaOpen}
  <Dialog title={t("Java")} wide open={javaOpen} onClose={() => (javaOpen = false)}>
    <div class="flex flex-col gap-2">
      <div class="text-[11px] uppercase tracking-wider text-fg-dim mt-1">{t("Automatic")}</div>
      <button
        class={`w-full text-left px-3 py-2.5 rounded-lg border transition-colors flex items-center justify-between gap-2 ${meta?.java_path == null ? 'border-brand-500 bg-brand-500/10' : 'border-edge bg-surface-2 hover:bg-surface-3'}`}
        onclick={() => void setJava(null)}
      >
        <span class="text-sm text-slate-100">{t("Auto (Java {major})", { major: meta?.java ?? "" })}</span>
        <span class="text-xs text-fg-dim">{meta ? `Java ${meta.java}` : ""}</span>
        {#if meta?.java_path == null}
          <Check size={15} class="text-brand-400 shrink-0" />
        {/if}
      </button>

      <div class="text-[11px] uppercase tracking-wider text-fg-dim mt-2">{t("Installed Java")}</div>
      {#if detectedLoading}
        <p class="text-sm text-fg-dim py-4 text-center">{t("Scanning for Java…")}</p>
      {:else if detected.length === 0}
        <p class="text-sm text-fg-dim py-4 text-center">{t("No Java found — scan in Settings.")}</p>
      {:else}
        {#each detected as java (java.path)}
          {@const chosen = meta?.java_path === java.path}
          <button
            class={`w-full text-left px-3 py-2.5 rounded-lg border transition-colors ${chosen ? 'border-brand-500 bg-brand-500/10' : 'border-edge bg-surface-2 hover:bg-surface-3'}`}
            onclick={() => void setJava(java.path)}
          >
            <div class="flex items-center justify-between gap-2">
              <span class="text-sm text-slate-100">{t("Java {major} — {vendor}", { major: java.major, vendor: java.vendor })}</span>
              {#if chosen}
                <Check size={15} class="text-brand-400 shrink-0" />
              {/if}
            </div>
            <div class="text-xs text-fg-dim break-all mt-0.5">{java.path}</div>
            <div class="text-xs text-fg-dim/60 mt-0.5">{java.source}</div>
          </button>
        {/each}
      {/if}

      <div class="flex items-center justify-end mt-2">
        <Button variant="ghost" size="sm" onclick={() => (javaOpen = false)}>
          <X size={14} /> {t("Cancel")}
        </Button>
      </div>
    </div>
  </Dialog>
{/if}

<div class="mt-4">
  <Console serverId={id} height="520px" />
</div>