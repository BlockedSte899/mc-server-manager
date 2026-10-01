<script lang="ts">
  import { velocityApi } from "../../lib/api";
  import { toast } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Input from "../../lib/ui/Input.svelte";
  import Switch from "../../lib/ui/Switch.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import Badge from "../../lib/ui/Badge.svelte";
  import Dialog from "../../lib/ui/Dialog.svelte";
  import { onMount } from "svelte";
  import type { VelocityConfig, VelocityCandidate } from "../../lib/types";
  import { Save, KeyRound, Plus, Trash2 } from "lucide-svelte";
  import { t } from "../../lib/i18n.svelte";

  let { id }: { id: string } = $props();

  let cfg: VelocityConfig = $state({
    bind: "0.0.0.0:25577",
    motd: "A Velocity Server",
    show_max_players: 500,
    online_mode: true,
    force_key_authentication: true,
    prevent_client_proxy_connections: false,
    player_info_forwarding_mode: "legacy",
    forwarding_secret: "",
    servers: {},
    forced_hosts: {},
  });
  let candidates: VelocityCandidate[] = $state([]);
  let loaded = $state(false);
  let saving = $state(false);
  let addOpen = $state(false);

  async function load() {
    loaded = false;
    try {
      cfg = await velocityApi.get(id);
      loaded = true;
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function save() {
    saving = true;
    try {
      cfg = await velocityApi.save(id, cfg);
      toast(t("Velocity config saved"), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      saving = false;
    }
  }

  async function regenSecret() {
    try {
      cfg = { ...cfg, forwarding_secret: await velocityApi.generateSecret(id) };
      toast(t("New forwarding secret generated"), "success");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function loadCandidates() {
    try {
      candidates = await velocityApi.candidates();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function addServer(v: VelocityCandidate) {
    const key = v.name;
    if (Object.prototype.hasOwnProperty.call(cfg.servers, key)) {
      toast(t("Server already registered"), "error");
      addOpen = false;
      return;
    }
    cfg = { ...cfg, servers: { ...cfg.servers, [key]: { address: v.address, enabled: true } } };
    candidates = candidates.filter((c) => c.id !== v.id);
    addOpen = false;
  }

  function removeServer(key: string) {
    const { [key]: _drop, ...rest } = cfg.servers;
    cfg = { ...cfg, servers: rest };
  }

  function setServerAddress(key: string, address: string) {
    const s = cfg.servers[key];
    if (!s) return;
    cfg = { ...cfg, servers: { ...cfg.servers, [key]: { ...s, address } } };
  }

  function setServerEnabled(key: string, enabled: boolean) {
    const s = cfg.servers[key];
    if (!s) return;
    cfg = { ...cfg, servers: { ...cfg.servers, [key]: { ...s, enabled } } };
  }

  onMount(load);
</script>

{#if loaded}
  <div class="flex flex-col gap-4">
    <Card title={t("Bind")} subtitle={t("Where the proxy listens")}>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="md:col-span-1">
          <Input label={t("Bind address")} value={cfg.bind} onChange={(v) => (cfg = { ...cfg, bind: v })} placeholder="0.0.0.0:25577" />
        </div>
        <div class="md:col-span-2">
          <Input label="MOTD" value={cfg.motd} onChange={(v) => (cfg = { ...cfg, motd: v })} />
        </div>
      </div>
    </Card>

    <Card title={t("Auth & Forwarding")}>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <Switch label={t("Online mode")} hint={t("Authenticate players with Mojang")} checked={cfg.online_mode} onChange={(v) => (cfg = { ...cfg, online_mode: v })} />
        <Switch label={t("Force key authentication")} checked={cfg.force_key_authentication} onChange={(v) => (cfg = { ...cfg, force_key_authentication: v })} />
        <Switch label={t("Prevent client proxy connections")} checked={cfg.prevent_client_proxy_connections} onChange={(v) => (cfg = { ...cfg, prevent_client_proxy_connections: v })} />
        <Select
          label={t("Player info forwarding mode")}
          value={cfg.player_info_forwarding_mode}
          options={[
            { value: "none", label: "none" },
            { value: "legacy", label: "legacy" },
            { value: "bungeeguard", label: "bungeeguard" },
            { value: "modern", label: "modern" },
          ]}
          onChange={(v) => (cfg = { ...cfg, player_info_forwarding_mode: v })}
        />
      </div>
      <div class="mt-4 flex flex-wrap items-end gap-3">
        <div class="flex-1 min-w-[260px]">
          <Input label={t("Forwarding secret")} type="text" value={cfg.forwarding_secret} onChange={(v) => (cfg = { ...cfg, forwarding_secret: v })} />
        </div>
        <div class="flex items-center gap-2">
          <Button variant="outline" size="sm" onclick={regenSecret} title={t("Generate random secret")}>
            <KeyRound size={14} /> {t("Generate")}
          </Button>
        </div>
      </div>
      <p class="text-xs text-fg-dim mt-2">
        {@html t("Velocity modern hint")}
      </p>
    </Card>

    <Card title={t("Registered Servers")} subtitle={t("Backend servers visible under this proxy")}>
      {#snippet actions()}
        <Button
          variant="outline"
          size="sm"
          onclick={async () => {
            await loadCandidates();
            addOpen = true;
          }}
        >
          <Plus size={14} /> {t("Add Server")}
        </Button>
      {/snippet}
      {#if Object.keys(cfg.servers).length === 0}
        <p class="text-sm text-fg-dim">{t("No backend servers registered.")}</p>
      {:else}
        <ul class="flex flex-col gap-1.5">
          {#each Object.entries(cfg.servers) as [key, srv]}
            <li class="flex flex-wrap items-center gap-3 px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
              <button
                class="shrink-0 text-xs cursor-pointer {srv.enabled ? 'text-emerald-400' : 'text-fg-dim'}"
                onclick={() => setServerEnabled(key, !srv.enabled)}
              >
                {srv.enabled ? t("ON") : t("OFF")}
              </button>
              <span class="font-medium text-sm text-slate-100 w-36 truncate">{key}</span>
              <input
                class="flex-1 min-w-[140px] font-mono text-xs bg-surface-2 border border-edge rounded-md px-2 py-1 text-slate-100 outline-none focus:border-brand-500"
                value={srv.address}
                onchange={(e) => setServerAddress(key, (e.currentTarget as HTMLInputElement).value)}
              />
              <button class="text-fg-dim hover:text-red-400 cursor-pointer" onclick={() => removeServer(key)}>
                <Trash2 size={14} />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </Card>

    <div class="flex items-center gap-3">
      <Button onclick={save} disabled={saving}>
        <Save size={16} /> {t("Save Config")}
      </Button>
      {#if cfg.player_info_forwarding_mode === "modern" && !cfg.forwarding_secret}
        <Badge class="bg-amber-500/15 text-amber-300 border-amber-500/30">{t("forwarding secret missing")}</Badge>
      {/if}
    </div>
  </div>

  <Dialog title={t("Add Backend Server")} open={addOpen} onClose={() => (addOpen = false)}>
    {#if candidates.length === 0}
      <p class="text-sm text-fg-dim">{t("No other servers available to register.")}</p>
    {:else}
      <ul class="flex flex-col gap-1.5">
        {#each candidates as c (c.id)}
          <li class="flex items-center justify-between px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
            <div>
              <div class="text-sm font-medium text-slate-100">{c.name}</div>
              <div class="text-xs font-mono text-fg-dim">{c.address}</div>
            </div>
            <Button variant="outline" size="sm" onclick={() => addServer(c)}>
              {Object.prototype.hasOwnProperty.call(cfg.servers, c.name) ? t("Added") : t("Add")}
            </Button>
          </li>
        {/each}
      </ul>
    {/if}
  </Dialog>
{:else}
  <p class="text-sm text-fg-dim">{t("Loading Velocity config…")}</p>
{/if}