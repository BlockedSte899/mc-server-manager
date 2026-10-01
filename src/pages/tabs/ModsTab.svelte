<script lang="ts">
  import { modsApi } from "../../lib/api";
  import { toast, fmtBytes, servers, windowHidden } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Badge from "../../lib/ui/Badge.svelte";
  import Input from "../../lib/ui/Input.svelte";
  import ConfirmDialog from "../../lib/ui/ConfirmDialog.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import type { FileItem, ModrinthProject, CfMod } from "../../lib/types";
  import { Search, RefreshCw, Upload, Loader2, Download, Power, Trash2, ArrowUpCircle } from "lucide-svelte";
  import { isTauri } from "../../lib/env";
  import { t } from "../../lib/i18n.svelte";

  let { id }: { id: string } = $props();

  let mods = $state<FileItem[]>([]);
  let query = $state("");
  let results: ModrinthProject[] = $state([]);
  let searching = $state(false);
  let cfResults: CfMod[] = $state([]);
  let cfSearching = $state(false);
  let installing = $state<Record<string, boolean>>({});
  let updating = $state<Record<string, boolean>>({});
  let confirmDelete: FileItem | null = $state(null);
  let mc = $state("");
  let imgFailed = $state<Record<string, boolean>>({});
  let icons = $state<Record<string, string>>({});
  let iconPending = $state<Record<string, boolean>>({});
  let iconFailed = $state<Record<string, boolean>>({});

  const core = $derived(servers.find((s) => s.meta.id === id)?.meta.core ?? "paper");
  const serverVersion = $derived(servers.find((s) => s.meta.id === id)?.meta.mc_version ?? "");

  const PLUGIN_CORES = ["paper", "purpur", "spigot", "bukkit", "folia"];
  const isPlugins = $derived(PLUGIN_CORES.includes(core));

  async function refresh() {
    try {
      mods = await modsApi.list(id);
      if (!mc) mc = serverVersion;
      if (isTauri()) void loadIcons(mods);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function loadIcons(items: FileItem[]) {
    for (const m of items) {
      if (icons[m.name] || iconPending[m.name] || iconFailed[m.name]) continue;
      iconPending[m.name] = true;
      try {
        const data = await modsApi.icon(id, m.name);
        if (data) icons[m.name] = data;
        else iconFailed[m.name] = true;
      } catch {
        iconFailed[m.name] = true;
      } finally {
        iconPending[m.name] = false;
      }
    }
  }

  async function search() {
    if (!query.trim()) return;
    searching = true;
    cfResults = [];
    try {
      results = await modsApi.search(query.trim(), core, mc || ".*");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      searching = false;
    }
  }

  async function curseforgeSearch() {
    if (!query.trim()) return;
    cfSearching = true;
    results = [];
    try {
      cfResults = await modsApi.curseforge(query.trim(), mc || "", core);
    } catch (e) {
      toast(String(e), "error");
    } finally {
      cfSearching = false;
    }
  }

  async function install(projectId: string, title: string) {
    installing = { ...installing, [projectId]: true };
    try {
      const res = await modsApi.install(id, projectId, mc || "");
      toast(t("Installed {filename}", { filename: res.filename }), "success");
      await refresh();
    } catch (e) {
      toast(t("Install failed: {e}", { e: String(e) }), "error");
    } finally {
      installing = { ...installing, [projectId]: false };
    }
  }

  async function toggle(m: FileItem) {
    try {
      await modsApi.setEnabled(id, m.name, !m.enabled);
      await refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function installLocal() {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "JAR or ZIP", extensions: ["jar", "zip"] }],
      });
      if (typeof picked === "string") {
        const name = await modsApi.installLocal(id, picked);
        toast(t("Installed {filename}", { filename: name }), "success");
        await refresh();
      }
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function doDelete() {
    if (!confirmDelete) return;
    const m = confirmDelete;
    confirmDelete = null;
    try {
      await modsApi.remove(id, m.name);
      toast(t("Deleted {name}", { name: m.name }), "success");
      await refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function update(m: FileItem) {
    if (updating[m.name]) return;
    updating = { ...updating, [m.name]: true };
    try {
      const res = await modsApi.update(id, m.name);
      toast(t("Updated to {filename}", { filename: res.filename }), "success");
      await refresh();
    } catch (e) {
      toast(t("Update failed: {e}", { e: String(e) }), "error");
    } finally {
      updating = { ...updating, [m.name]: false };
    }
  }

  onMount(() => {
    refresh();
    const int = setInterval(() => {
      if (!windowHidden()) refresh();
    }, 8000);
    return () => clearInterval(int);
  });
</script>

<div class="flex flex-col gap-4">
  <div class="flex flex-wrap items-end gap-3">
    <div class="w-40">
      <Input label={t("MC version")} bind:value={mc} placeholder="e.g. 1.21.4" />
    </div>
    <div class="flex-1 min-w-[220px]">
      <Input label={t(isPlugins ? "Search plugins" : "Search mods")} bind:value={query} placeholder={t("Name on Modrinth / CurseForge")} />
    </div>
    <div class="flex items-center gap-2">
      <Button onclick={search} disabled={searching || !query.trim()}>
        {#if searching}<Loader2 size={15} class="animate-spin" />{:else}<Search size={15} />{/if}
        Modrinth
      </Button>
      <Button variant="subtle" onclick={curseforgeSearch} disabled={cfSearching || !query.trim()}>
        {#if cfSearching}<Loader2 size={15} class="animate-spin" />{:else}<Search size={15} />{/if}
        CurseForge
      </Button>
      <Button variant="outline" onclick={installLocal}>
        <Upload size={15} /> {t("Local file")}
      </Button>
    </div>
  </div>

  {#if results.length > 0}
    <p class="text-xs text-fg-dim uppercase tracking-wide">{t("Modrinth results")}</p>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
      {#each results as r (r.project_id)}
        <div class="rounded-lg border border-edge bg-surface-1 p-3 flex items-start gap-3">
          {#if r.icon_url && !imgFailed[r.project_id]}
            <img src={r.icon_url} alt="" class="w-10 h-10 rounded-lg object-cover bg-surface-2" loading="lazy" onerror={() => (imgFailed[r.project_id] = true)} />
          {:else}
            <div class="w-10 h-10 rounded-lg bg-surface-2 border border-edge flex items-center justify-center text-lg font-bold text-brand-400">{r.title.slice(0, 1)}</div>
          {/if}
          <div class="flex-1 min-w-0">
            <div class="flex items-center justify-between gap-2">
              <span class="font-medium text-sm text-slate-100 truncate">{r.title}</span>
              <Badge class="bg-surface-2 text-fg-dim shrink-0">{(r.downloads / 1000).toFixed(1)}k</Badge>
            </div>
            <p class="text-xs text-fg-dim mt-0.5 line-clamp-2">{r.description ?? ""}</p>
            <div class="flex items-center justify-between mt-2">
              <span class="text-[10px] text-fg-dim uppercase tracking-wide">
                {r.server_side === "unsupported" ? t("server: unsupported") : r.server_side === "required" ? t("required") : t("compatible")} · {(r.game_versions ?? []).slice(-3).join(", ") || t("any")}
              </span>
              <Button
                variant="ghost"
                size="xs"
                disabled={installing[r.project_id] || r.server_side === "unsupported"}
                onclick={() => install(r.project_id, r.title)}
              >
                {#if installing[r.project_id]}<Loader2 size={13} class="animate-spin" />{:else}<Download size={13} />{/if}
                {t("Install")}
              </Button>
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}

  {#if cfResults.length > 0}
    <p class="text-xs text-fg-dim uppercase tracking-wide">{t("CurseForge results (modpack/plugin contexts)")}</p>
    <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
      {#each cfResults as r (r.id)}
        <div class="rounded-lg border border-edge bg-surface-1 p-3 flex items-start gap-3">
          {#if r.logo?.url && !imgFailed[String(r.id)]}
            <img src={r.logo.url} alt="" class="w-10 h-10 rounded-lg object-cover bg-surface-2" loading="lazy" onerror={() => (imgFailed[String(r.id)] = true)} />
          {:else}
            <div class="w-10 h-10 rounded-lg bg-surface-2 border border-edge flex items-center justify-center text-lg font-bold text-brand-400">{r.name.slice(0, 1)}</div>
          {/if}
          <div class="flex-1 min-w-0">
            <div class="flex items-center justify-between gap-2">
              <span class="font-medium text-sm text-slate-100 truncate">{r.name}</span>
              <Badge class="bg-surface-2 text-fg-dim shrink-0">{(r.downloadCount / 1000).toFixed(1)}k</Badge>
            </div>
            <p class="text-xs text-fg-dim mt-0.5 line-clamp-2">{r.summary}</p>
            <div class="text-[10px] text-fg-dim mt-1 uppercase tracking-wide">
              {r.latestFilesIndexes?.map((f) => f.gameVersion).filter(Boolean).slice(-3).join(", ") || "any"}
            </div>
          </div>
        </div>
      {/each}
    </div>
    <p class="text-xs text-fg-dim">
      {t(isPlugins
        ? "CurseForge plugin searches are shown here for convenience; installs go to the plugins folder."
        : "CurseForge installs for modpacks are handled by the Import page; server mods via Modrinth preferred.")}
    </p>
  {/if}

  <Card
    title={t(isPlugins ? "Installed Plugins" : "Installed Mods")}
    subtitle={t(isPlugins ? "JAR files in the plugins folder" : "JAR files in the mods folder")}
  >
    {#snippet actions()}
      <Button variant="ghost" size="sm" onclick={refresh} title={t("Refresh")}><RefreshCw size={14} /></Button>
    {/snippet}
    {#if mods.length === 0}
      <p class="text-sm text-fg-dim">{t(isPlugins ? "No plugins installed yet." : "No mods installed yet.")}</p>
    {:else}
      <ul class="flex flex-col gap-1.5">
        {#each mods as m (m.name)}
          <li class="flex items-center gap-3 px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
            <button
              class="shrink-0 cursor-pointer {m.enabled ? 'text-emerald-400' : 'text-fg-dim'}"
              title={m.enabled ? t("Enabled") : t("Disabled (.disabled)")}
              onclick={() => toggle(m)}
            >
              <Power size={15} />
            </button>
            {#if icons[m.name]}
              <img src={icons[m.name]} alt="" class="w-8 h-8 rounded-md object-cover bg-surface-2 border border-edge shrink-0" loading="lazy" />
            {:else if iconFailed[m.name]}
              <div class="w-8 h-8 rounded-md bg-surface-2 border border-edge flex items-center justify-center text-sm font-bold text-brand-400 shrink-0">{m.name.slice(0, 1)}</div>
            {:else if iconPending[m.name]}
              <div class="w-8 h-8 rounded-md bg-surface-2 border border-edge flex items-center justify-center shrink-0">
                <Loader2 size={13} class="animate-spin text-fg-dim" />
              </div>
            {/if}
            <span class={`flex-1 text-sm truncate ${m.enabled ? "text-slate-100" : "text-fg-dim line-through"}`}>
              {m.enabled ? m.name : m.name}
            </span>
            <span class="text-xs text-fg-dim">{fmtBytes(m.size)}</span>
            <button
              class="shrink-0 cursor-pointer text-fg-dim hover:text-amber-400"
              title={t("Update from Modrinth")}
              onclick={() => update(m)}
              disabled={updating[m.name]}
            >
              {#if updating[m.name]}<Loader2 size={15} class="animate-spin" />{:else}<ArrowUpCircle size={15} />{/if}
            </button>
            <button
              class="shrink-0 cursor-pointer text-fg-dim hover:text-red-400"
              title={t(isPlugins ? "Delete plugin" : "Delete mod")}
              onclick={() => (confirmDelete = m)}
            >
              <Trash2 size={15} />
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>
</div>

{#if confirmDelete}
  <ConfirmDialog
    title={t(isPlugins ? "Delete this plugin?" : "Delete this mod?")}
    message={t('Delete "{name}" permanently?\nThe JAR file will be removed from disk.', { name: confirmDelete.name })}
    confirmLabel={t("Delete")}
    open={true}
    onConfirm={doDelete}
    onClose={() => (confirmDelete = null)}
  />
{/if}