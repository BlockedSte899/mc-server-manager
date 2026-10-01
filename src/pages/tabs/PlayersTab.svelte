<script lang="ts">
  import { playersApi } from "../../lib/api";
  import { toast, windowHidden } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Input from "../../lib/ui/Input.svelte";
  import Badge from "../../lib/ui/Badge.svelte";
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import type { PlayersView } from "../../lib/types";
  import { UserPlus, UserMinus, Hammer, TerminalSquare, RefreshCw } from "lucide-svelte";
  import { t, pl } from "../../lib/i18n.svelte";

  let { id, isProxy = false }: { id: string; isProxy?: boolean } = $props();

  let view: PlayersView = $state({
    online: [],
    max_online: 0,
    whitelist_enabled: false,
    whitelist: [],
    banned: [],
    banned_ips: [],
    ops: [],
    usercache: [],
  });

  let whitelistName = $state("");
  let opName = $state("");
  let banName = $state("");
  let command = $state("");
  let headFailed = $state<Record<string, boolean>>({});
  let lastListed = $state(-1);
  let syncing = $state(false);

  const headUrl = (name: string) => `https://mc-heads.net/avatar/${encodeURIComponent(name)}/16`;

  const strip = (s: string) => s.replace(/^\/*/, "");

  async function refresh(silent = false) {
    try {
      view = await playersApi.get(id);
    } catch (e) {
      if (!silent) toast(String(e), "error");
    }
  }

  /** Sends `list` once — only while this tab is mounted — then re-reads names. */
  async function requestList(silent = false) {
    if (syncing) return;
    syncing = true;
    lastListed = view.online.length;
    try {
      await playersApi.command(id, "list");
      await new Promise((r) => setTimeout(r, 1600));
      await refresh(silent);
    } catch (e) {
      if (!silent) toast(String(e), "error");
    } finally {
      syncing = false;
    }
  }

  function maybeRequestList() {
    if (!syncing && view.online.length !== lastListed) void requestList(true);
  }

  async function run(cmd: string) {
    try {
      await playersApi.command(id, strip(cmd));
      toast(t("Sent: {cmd}", { cmd: strip(cmd) }), "success");
      setTimeout(refresh, 1500);
    } catch (e) {
      toast(t("Error running command: {e}", { e: String(e) }), "error");
    }
  }

  async function sendCmd() {
    const c = command.trim();
    if (!c) return;
    command = "";
    await run(c);
  }

onMount(() => {
    refresh(true);
    setTimeout(() => requestList(true), 400);
    const int = setInterval(() => {
      if (windowHidden()) return;
      refresh(true);
      maybeRequestList();
    }, 5000);
    let un: UnlistenFn | undefined;
    void listen("server-players", (ev) => {
      const p = ev.payload as { id: string; names: string[]; max: number };
      if (p.id === id) {
        view.online = p.names;
        view.max_online = p.max;
        maybeRequestList();
      }
    }).then((fn) => (un = fn));
    return () => {
      clearInterval(int);
      un?.();
    };
  });
</script>

<div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
  <Card
    title={t("Online Players")}
    subtitle={view.online.length > 0 ? t("Connected right now") : t("Nobody online")}
  >
    {#snippet actions()}
      <Button
        variant="ghost"
        size="xs"
        onclick={() => requestList()}
        disabled={syncing}
        title={t("Refresh player list")}
      >
        <RefreshCw size={13} class={syncing ? "animate-spin" : ""} />
      </Button>
      <Badge class="bg-brand-500/15 text-brand-300 border-brand-500/30">
        {view.online.length}/{view.max_online || 20}
      </Badge>
    {/snippet}
    {#if view.online.length === 0}
      <p class="text-sm text-fg-dim">{t("No players connected.")}</p>
    {:else}
      <ul class="flex flex-col gap-1.5">
        {#each view.online as name}
          <li class="flex items-center justify-between px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
            <span class="flex items-center gap-2 text-sm font-medium text-slate-100">
              {#if !headFailed[name]}
                <img
                  src={headUrl(name)}
                  alt=""
                  width="16"
                  height="16"
                  loading="lazy"
                  class="rounded-sm ring-1 ring-surface-3/60 bg-surface-2"
                  onerror={() => (headFailed[name] = true)}
                />
              {/if}
              <span class="w-1.5 h-1.5 rounded-full bg-emerald-400/80"></span>
              {name}
            </span>
            <div class="flex gap-1">
              {#if !isProxy}
                <Button variant="ghost" size="xs" onclick={() => run(`kick ${name} Bye!`)} title={t("Kick")}>
                  <UserMinus size={13} />
                </Button>
                <Button variant="ghost" size="xs" onclick={() => run(`ban ${name}`)} title={t("Ban")} class="!text-red-400">
                  <Hammer size={13} />
                </Button>
                <Button variant="ghost" size="xs" onclick={() => run(`op ${name}`)} title={t("Op")}>
                  <UserPlus size={13} />
                </Button>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>

  {#if isProxy}
    <Card title={t("Proxy Commands")} subtitle={t("Run Velocity console commands")}>
      <div class="flex gap-2">
        <div class="flex-1">
          <Input label="" bind:value={command} placeholder={t("e.g. velocity kick Alice Too slow!")} onkeydown={(e) => { if (e.key === "Enter") sendCmd(); }} />
        </div>
        <div class="flex items-end">
          <Button onclick={sendCmd}><TerminalSquare size={14} /> {t("Send")}</Button>
        </div>
      </div>
      <p class="text-xs text-fg-dim mt-2">
        {t("Online players are polled from {code} automatically.", { code: "velocity list" })}
      </p>
    </Card>
  {:else}
  <Card title={t("Whitelist")} subtitle={view.whitelist_enabled ? t("Whitelist is enabled") : t("Whitelist is disabled")}>
    <div class="flex gap-2 mb-3">
      <div class="flex-1">
        <Input label="" bind:value={whitelistName} placeholder={t("Username")} />
      </div>
      <div class="flex items-end">
        <Button onclick={() => { run(`whitelist add ${whitelistName.trim()}`); whitelistName = ""; }}>
          {t("Add")}
        </Button>
      </div>
    </div>
    {#if view.whitelist.length === 0}
      <p class="text-sm text-fg-dim">{t("Whitelist is empty.")}</p>
    {:else}
      <ul class="flex flex-col gap-1 max-h-56 overflow-y-auto">
        {#each view.whitelist as e}
          <li class="flex items-center justify-between px-3 py-1.5 rounded-lg bg-surface-2/60 border border-edge">
            <span class="text-sm">{e.name}</span>
            <button class="text-xs text-red-400 hover:text-red-300 cursor-pointer" onclick={() => run(`whitelist remove ${e.name}`)}>
              {t("remove")}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>

  <Card title={t("Operators")} subtitle={pl(view.ops.length, ["{n} оператор", "{n} оператора", "{n} операторов"], "{n} ops")}>
    <div class="flex gap-2 mb-3">
      <div class="flex-1"><Input label="" bind:value={opName} placeholder={t("Username")} /></div>
      <div class="flex items-end">
        <Button onclick={() => { run(`op ${opName.trim()}`); opName = ""; }}>{t("Op")}</Button>
      </div>
    </div>
    {#if view.ops.length === 0}
      <p class="text-sm text-fg-dim">{t("No ops.")}</p>
    {:else}
      <ul class="flex flex-col gap-1 max-h-56 overflow-y-auto">
        {#each view.ops as e}
          <li class="flex items-center justify-between px-3 py-1.5 rounded-lg bg-surface-2/60 border border-edge">
            <span class="text-sm">{e.name}</span>
            <button class="text-xs text-red-400 hover:text-red-300 cursor-pointer" onclick={() => run(`deop ${e.name}`)}>{t("deop")}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>

  <Card title={t("Bans")} subtitle={t("{a} player bans · {b} IP bans", { a: view.banned.length, b: view.banned_ips.length })}>
    <div class="flex gap-2 mb-3">
      <div class="flex-1"><Input label="" bind:value={banName} placeholder={t("Username or IP")} /></div>
      <div class="flex items-end">
        <Button onclick={() => { run(`ban ${banName.trim()}`); banName = ""; }}>{t("Ban")}</Button>
      </div>
    </div>
    {#if view.banned.length === 0 && view.banned_ips.length === 0}
      <p class="text-sm text-fg-dim">{t("No bans.")}</p>
    {:else}
      <ul class="flex flex-col gap-1 max-h-56 overflow-y-auto">
        {#each view.banned as e}
          <li class="flex items-center justify-between px-3 py-1.5 rounded-lg bg-surface-2/60 border border-edge">
            <span class="text-sm">{e.name}</span>
            <button class="text-xs text-red-400 hover:text-red-300 cursor-pointer" onclick={() => run(`pardon ${e.name}`)}>{t("pardon")}</button>
          </li>
        {/each}
        {#each view.banned_ips as ip}
          <li class="flex items-center justify-between px-3 py-1.5 rounded-lg bg-surface-2/60 border border-edge">
            <span class="text-sm">{ip}</span>
            <button class="text-xs text-red-400 hover:text-red-300 cursor-pointer" onclick={() => run(`pardon-ip ${ip}`)}>{t("pardon")}</button>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>
  {/if}
</div>