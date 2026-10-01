<script lang="ts">
  import { onMount } from "svelte";
  import { logApi } from "../../lib/api";
  import { cleanConsoleLine, highlightLogHtml } from "../../lib/consoleLines";
  import { statuses, toast, windowHidden } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import { RefreshCw, ScrollText, Loader2, Clipboard } from "lucide-svelte";
  import { t, pl } from "../../lib/i18n.svelte";

  let { id }: { id: string } = $props();

  let lines = $state<string[]>([]);
  let path = $state("");
  let total = $state(0);
  let truncated = $state(false);
  let loading = $state(false);
  let following = $state(true);
  let boxEl: HTMLDivElement | undefined = $state();

  async function load() {
    loading = true;
    try {
      const view = await logApi.get(id);
      lines = view.lines.flatMap(cleanConsoleLine);
      path = view.path;
      total = view.total_lines;
      truncated = view.truncated;
    } catch (e) {
      lines = [t("Error reading logs: {e}", { e: String(e) })];
      path = "";
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (lines.length === 0) return;
    if (following && boxEl) {
      boxEl.scrollTop = boxEl.scrollHeight;
    }
  });

  async function copyLogs() {
    try {
      await navigator.clipboard.writeText(lines.join("\n"));
      toast(t("Logs copied"), "success");
    } catch {
      toast(t("Copy failed"), "error");
    }
  }

  function onScroll() {
    if (!boxEl) return;
    const atBottom = boxEl.scrollHeight - boxEl.scrollTop - boxEl.clientHeight < 40;
    if (!following && atBottom) following = true;
    if (!atBottom && following) following = false;
  }

  const running = $derived(
    statuses[id] === "running" || statuses[id] === "starting" || statuses[id] === "stopping"
  );

  onMount(() => {
    load();
    const int = setInterval(() => {
      if (running && !windowHidden()) load();
    }, 2000);
    return () => clearInterval(int);
  });
</script>

<Card
  title={t("Server Logs")}
  subtitle={path ? t("{path} · {total} lines total", { path, total }) : t("No log file found yet — start the server first")}
  class="h-full flex flex-col"
  bodyClass="flex-1 flex flex-col min-h-0"
>
  {#snippet actions()}
    <Button
      variant={following ? "primary" : "outline"}
      size="sm"
      title={t("Auto-scroll to the newest lines")}
      onclick={() => (following = !following)}
    >
      <ScrollText size={14} /> {following ? t("Following") : t("Follow")}
    </Button>
    <Button variant="outline" size="sm" title={t("Copy logs")} onclick={copyLogs} disabled={lines.length === 0}>
      <Clipboard size={14} /> {t("Copy logs")}
    </Button>
    <Button variant="ghost" size="sm" title={t("Refresh")} onclick={load} disabled={loading}>
      {#if loading}
        <Loader2 size={14} class="animate-spin" />
      {:else}
        <RefreshCw size={14} />
      {/if}
    </Button>
  {/snippet}

  <div class="flex items-center gap-2 text-[11px] text-fg-dim mb-2">
    {#if truncated}
      <span class="text-amber-400">{t("Showing last {n} lines", { n: lines.length })}</span>
    {:else if lines.length === 0}
      <span>{t("Empty log")}</span>
    {:else}
      <span>{pl(lines.length, ["{n} строка", "{n} строки", "{n} строк"], "{n} lines")}</span>
    {/if}
    {#if running}
      <span class="text-emerald-400">· {t("live")}</span>
    {/if}
  </div>

  <div
    bind:this={boxEl}
    onscroll={onScroll}
    class="scroll-gutter flex-1 min-h-0 overflow-y-auto rounded-lg border border-edge bg-surface-0/60 p-3 font-mono text-sm leading-relaxed"
  >
    {#each lines as l, i (i)}
      <div class="whitespace-pre-wrap break-words text-slate-300">{@html highlightLogHtml(l)}</div>
    {/each}
  </div>
</Card>