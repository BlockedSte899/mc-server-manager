<script lang="ts">
  import { worldsApi, playersApi, serversApi } from "../../lib/api";
  import { toast, fmtBytes, servers, windowHidden } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Badge from "../../lib/ui/Badge.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import Dialog from "../../lib/ui/Dialog.svelte";
  import Input from "../../lib/ui/Input.svelte";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import type { WorldInfo, DatapackInfo, WorldSettings, ModrinthProject } from "../../lib/types";
  import {
    Archive,
    Trash2,
    History,
    Loader2,
    RotateCcw,
    Package,
    Download,
    Upload,
    Settings2,
    Search,
    Plus,
    Pencil,
    Save,
  } from "lucide-svelte";
  import { t } from "../../lib/i18n.svelte";

  let { id }: { id: string } = $props();

  let worlds = $state<WorldInfo[]>([]);
  let backups = $state<string[]>([]);
  let busyWorld = $state<string | null>(null);
  let restoring = $state<string | null>(null);
  let deletingBackup = $state<string | null>(null);
  let showBackups = $state(false);
  let autoDays = $state("0");
  let autoKeep = $state("0");
  let savingAuto = $state(false);
  let renameTarget = $state<string | null>(null);
  let renameName = $state("");

  let selectedWorld = $state<string | null>(null);
  let datapacks = $state<DatapackInfo[]>([]);
  let settings = $state<WorldSettings | null>(null);
  let dcBusy = $state<Record<string, string>>({});
  let installing = $state(false);
  let dlUrl = $state("");
  let applyingRule = $state<string | null>(null);
  let applyingDiff = $state(false);

  let showModrinth = $state(false);
  let mrQuery = $state("");
  let mrSearching = $state(false);
  let mrResults = $state<ModrinthProject[]>([]);
  let mrInstalling = $state<Record<string, boolean>>({});
  let mrIcons = $state<Record<string, string | null>>({});

  async function loadMrIcons(results: ModrinthProject[]) {
    for (const r of results) {
      if (!r.icon_url || r.project_id in mrIcons) continue;
      worldsApi
        .modrinthIcon(r.icon_url)
        .then((data) => {
          mrIcons = { ...mrIcons, [r.project_id]: data };
        })
        .catch(() => {
          mrIcons = { ...mrIcons, [r.project_id]: null };
        });
    }
  }
  const serverVersion = $derived(servers.find((s) => s.meta.id === id)?.meta.mc_version ?? "");

  async function mrSearch() {
    if (!mrQuery.trim()) return;
    mrSearching = true;
    try {
      const res = await worldsApi.searchModrinth(mrQuery.trim(), serverVersion);
      mrResults = res;
      void loadMrIcons(res);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      mrSearching = false;
    }
  }

  async function mrInstall(pid: string) {
    if (!selectedWorld) return;
    mrInstalling = { ...mrInstalling, [pid]: true };
    try {
      const name = await worldsApi.installDatapackModrinth(id, selectedWorld, pid, serverVersion);
      toast(t("Installed {name}", { name }), "success");
      await loadDatapacks(selectedWorld);
    } catch (e) {
      toast(t("Install failed: {e}", { e: String(e) }), "error");
    } finally {
      mrInstalling = { ...mrInstalling, [pid]: false };
    }
  }

  async function refresh() {
    try {
      worlds = await worldsApi.list(id);
      backups = await worldsApi.listBackups(id);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function loadDatapacks(world: string) {
    try {
      datapacks = await worldsApi.datapacks(id, world);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function loadSettings(world: string) {
    try {
      settings = await worldsApi.settings(id, world);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function selectWorld(name: string) {
    if (selectedWorld === name) {
      selectedWorld = null;
      return;
    }
    selectedWorld = name;
    await Promise.all([loadDatapacks(name), loadSettings(name)]);
  }

  async function installLocal() {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: "Datapack ZIP", extensions: ["zip"] }],
    });
    if (!selectedWorld || typeof picked !== "string") return;
    installing = true;
    try {
      const name = await worldsApi.installDatapack(id, selectedWorld, picked);
      toast(t("Installed {name}", { name }), "success");
      await loadDatapacks(selectedWorld);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      installing = false;
    }
  }

  async function downloadPack() {
    const url = dlUrl.trim();
    if (!selectedWorld || !url) return;
    installing = true;
    try {
      const name = await worldsApi.downloadDatapack(id, selectedWorld, url);
      toast(t("Downloaded {name}", { name }), "success");
      dlUrl = "";
      await loadDatapacks(selectedWorld);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      installing = false;
    }
  }

  async function togglePack(name: string, enabled: boolean) {
    if (!selectedWorld) return;
    dcBusy[name] = "toggle";
    try {
      await worldsApi.toggleDatapack(id, selectedWorld, name, enabled);
      await loadDatapacks(selectedWorld);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      delete dcBusy[name];
    }
  }

  async function delPack(name: string) {
    if (!selectedWorld || !confirm(t('Delete datapack "{name}"?', { name }))) return;
    dcBusy[name] = "del";
    try {
      await worldsApi.deleteDatapack(id, selectedWorld, name);
      await loadDatapacks(selectedWorld);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      delete dcBusy[name];
    }
  }

  async function applyGamerule(name: string) {
    const rule = settings?.gamerules.find((g) => g.name === name);
    if (!rule) return;
    applyingRule = name;
    try {
      await playersApi.command(id, `/gamerule ${rule.name} ${rule.value}`);
      toast(t("Sent /gamerule {rule} {value}", { rule: rule.name, value: rule.value }), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      applyingRule = null;
    }
  }

  async function applyDifficulty() {
    if (!settings) return;
    applyingDiff = true;
    try {
      await playersApi.command(id, `/difficulty ${settings.difficulty}`);
      toast(t("Sent /difficulty {difficulty}", { difficulty: settings.difficulty }), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      applyingDiff = false;
    }
  }

  function fmtDownloads(n: number): string {
    if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
    if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
    return String(n);
  }

  async function backup(world: string) {
    busyWorld = world;
    try {
      const name = await worldsApi.backup(id, world);
      toast(t("Backup saved: {name}", { name }), "success");
      await refresh();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      busyWorld = null;
    }
  }

  async function restore(zipfile: string) {
    if (!confirm(t('Restore backup "{zipfile}"? Existing world folders will be overwritten.', { zipfile }))) return;
    restoring = zipfile;
    try {
      await worldsApi.restore(id, zipfile);
      toast(t("Backup restored"), "success");
      await refresh();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      restoring = null;
    }
  }

  async function deleteBackup(zipfile: string) {
    if (!confirm(t('Delete backup "{zipfile}"?', { zipfile }))) return;
    deletingBackup = zipfile;
    try {
      await worldsApi.deleteBackup(id, zipfile);
      toast(t("Deleted {zipfile}", { zipfile }), "success");
      await refresh();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      deletingBackup = null;
    }
  }

  async function openBackups() {
    showBackups = true;
    try {
      const meta = await serversApi.get(id);
      autoDays = String(meta.auto_backup_days || 0);
      autoKeep = String(meta.auto_backup_keep || 0);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function saveAutoCfg() {
    savingAuto = true;
    try {
      const days = Math.max(0, Math.trunc(Number(autoDays)) || 0);
      const keep = Math.max(0, Math.trunc(Number(autoKeep)) || 0);
      await serversApi.setBackupConfig(id, days, keep);
      autoDays = String(days);
      autoKeep = String(keep);
      toast(t("Auto-backup settings saved"), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      savingAuto = false;
    }
  }

  async function doRename() {
    const name = renameName.trim();
    if (!renameTarget || !name) return;
    try {
      const newName = await worldsApi.renameBackup(id, renameTarget, name);
      toast(t("Renamed to {newName}", { newName }), "success");
      renameTarget = null;
      await refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function del(world: string) {
    if (!confirm(t('Delete world "{world}" and all its data permanently?', { world }))) return;
    try {
      await worldsApi.delete(id, world);
      toast(t("Deleted {world}", { world }), "success");
      if (selectedWorld === world) {
        selectedWorld = null;
        datapacks = [];
        settings = null;
      }
      await refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  onMount(() => {
    refresh();
    const int = setInterval(() => {
      if (!windowHidden()) refresh();
    }, 10000);
    return () => clearInterval(int);
  });
</script>

<div class="flex flex-col gap-4">
  <div class="flex items-center justify-between">
    <p class="text-sm text-fg-dim">
      {@html t("Worlds are detected by {code} / region folders inside the server directory.", { code: "<code class=\"text-brand-300\">level.dat</code>" })}
    </p>
    <Button variant="subtle" size="sm" onclick={openBackups}>
      <History size={14} /> {t("Backups ({n})", { n: backups.length })}
    </Button>
  </div>

  {#if worlds.length === 0}
    <div class="rounded-xl border border-dashed border-edge p-12 text-center bg-surface-1">
      <p class="text-fg-dim text-sm">{t("No worlds found yet. Start the server once to generate one.")}</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
      {#each worlds as w (w.name)}
        <div class="rounded-xl border border-edge bg-surface-1 p-4">
          <div class="flex items-center justify-between">
            <div>
              <div class="font-semibold text-slate-100">{w.name}</div>
              <div class="text-xs text-fg-dim mt-0.5">{fmtBytes(w.size_bytes)}</div>
            </div>
            {#if w.backup}
              <Badge class="bg-emerald-500/15 text-emerald-300 border-emerald-500/30">{t("backup ready")}</Badge>
            {/if}
          </div>
          <div class="flex items-center gap-2 mt-4 border-t border-edge pt-3">
            <Button
              variant="outline"
              size="sm"
              disabled={busyWorld === w.name}
              onclick={() => backup(w.name)}
            >
              {#if busyWorld === w.name}<Loader2 size={13} class="animate-spin" />{:else}<Archive size={13} />{/if}
              {t("Backup")}
            </Button>
            <div class="flex-1"></div>
            <Button
              variant={selectedWorld === w.name ? "primary" : "ghost"}
              size="sm"
              title={t("Datapacks and world settings")}
              onclick={() => selectWorld(w.name)}
            >
              <Settings2 size={14} />
              {t("Manage")}
            </Button>
            <Button variant="ghost" size="sm" title={t("Delete world")} class="!text-red-400" onclick={() => del(w.name)}>
              <Trash2 size={14} />
            </Button>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if selectedWorld}
    <Card>
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2">
          <Package size={15} class="text-brand-300" />
          <span class="font-semibold text-slate-100">{t("Datapacks")}</span>
          <span class="text-fg-dim">· {selectedWorld}</span>
        </div>
        <div class="flex items-center gap-2">
          <input
            bind:value={dlUrl}
            placeholder="https://… datapack.zip"
            class="w-64 rounded-lg border border-edge bg-surface-2 px-3 py-1.5 text-sm text-slate-100 placeholder:text-fg-dim focus:border-brand-400 focus:outline-none"
          />
          <Button variant="outline" size="sm" title={t("Search datapacks on Modrinth")} onclick={() => (showModrinth = true)}>
            <Search size={13} /> {t("Modrinth…")}
          </Button>
          <Button variant="outline" size="sm" disabled={installing || !dlUrl.trim()} onclick={downloadPack}>
            {#if installing}<Loader2 size={13} class="animate-spin" />{:else}<Download size={13} />{/if}
            {t("Download")}
          </Button>
          <Button variant="outline" size="sm" disabled={installing} onclick={installLocal}>
            <Upload size={13} /> {t("Install…")}
          </Button>
        </div>
      </div>
      <div class="mt-3">
        {#if datapacks.length === 0}
          <p class="text-sm text-fg-dim">{t("No datapacks in this world. Install a .zip or paste a download URL.")}</p>
        {:else}
          <ul class="flex flex-col gap-1.5">
            {#each datapacks as dp (dp.name)}
              <li class="flex items-center gap-3 px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
                {#if dp.enabled}
                  <Badge class="bg-emerald-500/15 text-emerald-300 border-emerald-500/30">{t("on")}</Badge>
                {:else}
                  <Badge class="bg-slate-500/15 text-fg-dim border-edge">{t("off")}</Badge>
                {/if}
                <span class="flex-1 text-sm font-mono text-slate-100 truncate">{dp.name}</span>
                <span class="text-xs text-fg-dim">{dp.kind}</span>
                <Button
                  variant="outline"
                  size="sm"
                  disabled={!!dcBusy[dp.name]}
                  onclick={() => togglePack(dp.name, !dp.enabled)}
                >
                  {#if dcBusy[dp.name] === "toggle"}<Loader2 size={13} class="animate-spin" />{:else}{dp.enabled ? t("Disable") : t("Enable")}{/if}
                </Button>
                <button
                  class="shrink-0 cursor-pointer text-fg-dim hover:text-red-400"
                  title={t("Delete datapack")}
                  disabled={!!dcBusy[dp.name]}
                  onclick={() => delPack(dp.name)}
                >
                  {#if dcBusy[dp.name] === "del"}<Loader2 size={14} class="animate-spin" />{:else}<Trash2 size={14} />{/if}
                </button>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </Card>

    <Card>
      <div class="flex items-center gap-2">
        <Settings2 size={15} class="text-brand-300" />
        <span class="font-semibold text-slate-100">{t("World config")}</span>
        <span class="text-fg-dim">· {selectedWorld}</span>
        <span class="flex-1"></span>
        <span class="text-xs text-fg-dim">{t("changes apply via the server console")}</span>
      </div>
      {#if settings}
        <div class="mt-4 grid grid-cols-1 md:grid-cols-3 gap-4">
          <div class="flex flex-col gap-1">
            <span class="text-xs text-fg-dim">{t("Seed")}</span>
            <span class="font-mono text-sm text-slate-100">
              {settings.seed !== null && settings.seed !== undefined ? settings.seed : t("unknown / not generated")}
            </span>
          </div>
          <div class="flex flex-col gap-1">
            <span class="text-xs text-fg-dim">{t("Difficulty")}</span>
            <div class="flex items-center gap-2">
              <Select
                value={settings.difficulty}
                options={[
                  { value: "peaceful", label: t("Peaceful") },
                  { value: "easy", label: t("Easy") },
                  { value: "normal", label: t("Normal") },
                  { value: "hard", label: t("Hard") },
                ]}
                onChange={(v) => { if (settings) settings.difficulty = v; }}
              />
              <Button variant="outline" size="sm" disabled={applyingDiff} onclick={applyDifficulty}>
                {#if applyingDiff}<Loader2 size={13} class="animate-spin" />{:else}{t("Apply")}{/if}
              </Button>
            </div>
          </div>
          <div class="flex flex-col gap-1">
            <span class="text-xs text-fg-dim">{t("Flags")}</span>
            <div class="flex items-center gap-4 text-sm text-slate-100">
              <span class="flex items-center gap-1.5">
                <input type="checkbox" disabled checked={settings.hardcore} class="accent-emerald-500" />
                {t("Hardcore")}
              </span>
              <span class="flex items-center gap-1.5">
                <input type="checkbox" disabled checked={settings.allow_cheats} class="accent-emerald-500" />
                {t("Cheats on")}
              </span>
            </div>
          </div>
        </div>
        <div class="mt-5 border-t border-edge pt-3">
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm text-fg-dim">{t("Gamerules")}</span>
            <span class="text-xs text-fg-dim">{t("edit + Apply sends /gamerule (server must be running)")}</span>
          </div>
          {#if settings.gamerules.length === 0}
            <p class="text-sm text-fg-dim">{t("No gamerules found in level.dat.")}</p>
          {:else}
            <div class="max-h-64 overflow-y-auto rounded-lg border border-edge">
              <ul class="divide-y divide-edge">
                {#each settings.gamerules as rule (rule.name)}
                  <li class="flex items-center gap-3 px-3 py-1.5 bg-surface-2/60">
                    <span class="flex-1 text-sm font-mono text-slate-100 truncate">{rule.name}</span>
                    <input
                      bind:value={rule.value}
                      class="w-44 rounded-lg border border-edge bg-surface-1 px-2 py-1 text-sm text-slate-100 focus:border-brand-400 focus:outline-none"
                    />
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={applyingRule === rule.name}
                      onclick={() => applyGamerule(rule.name)}
                    >
                      {#if applyingRule === rule.name}<Loader2 size={13} class="animate-spin" />{:else}{t("Apply")}{/if}
                    </Button>
                  </li>
                {/each}
              </ul>
            </div>
          {/if}
        </div>
      {:else}
        <p class="mt-3 text-sm text-fg-dim">{t("No level.dat yet — start the server once to generate it.")}</p>
      {/if}
    </Card>
  {/if}

  <Dialog title={t("Modrinth datapacks · {world}", { world: selectedWorld ?? "" })} open={showModrinth} onClose={() => (showModrinth = false)} wide>
    <div class="flex items-center gap-2 mb-3">
      <input
        bind:value={mrQuery}
        placeholder={t("Search datapacks…")}
        class="flex-1 rounded-lg border border-edge bg-surface-2 px-3 py-1.5 text-sm text-slate-100 placeholder:text-fg-dim focus:border-brand-400 focus:outline-none"
        onkeydown={(e) => {
          if (e.key === "Enter") void mrSearch();
        }}
      />
      <Button variant="outline" size="sm" disabled={mrSearching || !mrQuery.trim()} onclick={mrSearch}>
        {#if mrSearching}<Loader2 size={13} class="animate-spin" />{:else}<Search size={13} />{/if}
        {t("Search")}
      </Button>
    </div>
    {#if mrResults.length === 0 && !mrSearching}
      <p class="text-sm text-fg-dim">
        {serverVersion ? t("Showing datapacks compatible with MC {v}.", { v: serverVersion }) : t("Search Modrinth for datapacks.")}
      </p>
    {/if}
    <ul class="flex flex-col gap-1.5 max-h-[60vh] overflow-y-auto">
      {#each mrResults as r (r.project_id)}
        <li class="flex items-center gap-3 px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
          {#if mrIcons[r.project_id]}
            <img
              src={mrIcons[r.project_id]}
              alt=""
              class="w-8 h-8 rounded-md object-cover bg-surface-1 border border-edge"
            />
          {/if}
          {#if !mrIcons[r.project_id]}
            <div class="w-8 h-8 rounded-md bg-surface-1 border border-edge flex items-center justify-center text-slate-300 text-sm font-semibold uppercase">
              {r.title.slice(0, 1)}
            </div>
          {/if}
          <div class="flex-1 min-w-0">
            <div class="text-sm font-medium text-slate-100 truncate">{r.title}</div>
            <div class="text-xs text-fg-dim truncate">{r.description ?? ""}</div>
          </div>
          <span class="text-xs text-fg-dim shrink-0">{fmtDownloads(r.downloads)}</span>
          <Button
            variant="outline"
            size="sm"
            disabled={!!mrInstalling[r.project_id]}
            onclick={() => mrInstall(r.project_id)}
          >
            {#if mrInstalling[r.project_id]}<Loader2 size={13} class="animate-spin" />{:else}<Plus size={13} />{/if}
            {t("Install")}
          </Button>
        </li>
      {/each}
    </ul>
  </Dialog>

  <Dialog title={t("Backups")} open={showBackups} onClose={() => (showBackups = false)} wide>
    <div class="mb-4 rounded-lg border border-edge bg-surface-1 p-3">
      <div class="flex items-center gap-2 mb-3">
        <Settings2 size={14} class="text-brand-300" />
        <span class="text-sm font-medium text-slate-100">{t("Auto-backup")}</span>
        <span class="text-xs text-fg-dim">{t("applies to all worlds of this server")}</span>
      </div>
      <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
        <div>
          <label for="auto-days" class="text-xs font-medium text-fg-dim uppercase tracking-wide">{t("Create every (days)")}</label>
          <input
            id="auto-days"
            type="number"
            min="0"
            step="1"
            bind:value={autoDays}
            class="mt-1.5 w-full rounded-lg border border-edge bg-surface-2 px-3 py-2 text-sm text-slate-100 outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/40"
          />
          <p class="text-xs text-fg-dim mt-1">{t("Auto-backup days hint")}</p>
        </div>
        <div>
          <label for="auto-keep" class="text-xs font-medium text-fg-dim uppercase tracking-wide">{t("Delete after (days)")}</label>
          <input
            id="auto-keep"
            type="number"
            min="0"
            step="1"
            bind:value={autoKeep}
            class="mt-1.5 w-full rounded-lg border border-edge bg-surface-2 px-3 py-2 text-sm text-slate-100 outline-none focus:border-brand-500 focus:ring-1 focus:ring-brand-500/40"
          />
          <p class="text-xs text-fg-dim mt-1">{t("Auto-backup keep hint")}</p>
        </div>
      </div>
      <div class="mt-3 flex justify-end">
        <Button variant="outline" size="sm" disabled={savingAuto} onclick={saveAutoCfg}>
          {#if savingAuto}<Loader2 size={13} class="animate-spin" />{:else}<Save size={13} />{/if}
          {t("Save")}
        </Button>
      </div>
    </div>

    {#if backups.length === 0}
      <p class="text-sm text-fg-dim">{t("No backups yet. Use the Backup button on a world.")}</p>
    {:else}
      <ul class="flex flex-col gap-1.5">
        {#each backups as zipfile (zipfile)}
          <li class="flex items-center gap-3 px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
            <span class="flex-1 text-sm font-mono text-slate-100 truncate">{zipfile}</span>
            <button
              class="shrink-0 cursor-pointer text-fg-dim hover:text-brand-300"
              title={t("Rename backup")}
              disabled={deletingBackup === zipfile}
              onclick={() => {
                renameTarget = zipfile;
                renameName = zipfile.replace(/\.zip$/, "");
              }}
            >
              <Pencil size={14} />
            </button>
            <Button
              variant="outline"
              size="sm"
              disabled={restoring === zipfile || deletingBackup === zipfile}
              onclick={() => restore(zipfile)}
            >
              {#if restoring === zipfile}<Loader2 size={13} class="animate-spin" />{:else}<RotateCcw size={13} />{/if}
              {t("Restore")}
            </Button>
            <button
              class="shrink-0 cursor-pointer text-fg-dim hover:text-red-400"
              title={t("Delete backup")}
              disabled={deletingBackup === zipfile}
              onclick={() => deleteBackup(zipfile)}
            >
              {#if deletingBackup === zipfile}<Loader2 size={14} class="animate-spin" />{:else}<Trash2 size={14} />{/if}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </Dialog>

  {#if renameTarget}
    <Dialog title={t("Rename backup: {name}", { name: renameTarget })} open={true} onClose={() => (renameTarget = null)}>
      <Input
        label={t("New name (.zip is added automatically)")}
        bind:value={renameName}
        onkeydown={(e) => {
          if (e.key === "Enter") void doRename();
        }}
      />
      <div class="flex justify-end gap-2 mt-4">
        <Button variant="outline" size="sm" onclick={() => (renameTarget = null)}>{t("Cancel")}</Button>
        <Button variant="primary" size="sm" disabled={!renameName.trim()} onclick={doRename}>{t("Rename")}</Button>
      </div>
    </Dialog>
  {/if}
</div>