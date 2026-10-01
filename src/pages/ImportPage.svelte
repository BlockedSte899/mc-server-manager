<script lang="ts">
  import { importerApi, settingsApi } from "../lib/api";
  import { settings, toast } from "../lib/store.svelte";
  import { navigate } from "../lib/router.svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import Card from "../lib/ui/Card.svelte";
  import Button from "../lib/ui/Button.svelte";
  import Input from "../lib/ui/Input.svelte";
  import Select from "../lib/ui/Select.svelte";
  import Switch from "../lib/ui/Switch.svelte";
  import Spinner from "../lib/ui/Spinner.svelte";
  import { FileArchive, Loader2, Package } from "lucide-svelte";
  import { t } from "../lib/i18n.svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { isTauri } from "../lib/env";
  import { pathsFromDataTransfer } from "../lib/drop";

  let path = $state("");
  let dragging = $state(false);
  let name = $state("");
  let java = $state(17);
  let minRam = $state(512);
  let maxRam = $state(4096);
  let acceptEula = $state(true);

  let importing = $state(false);
  let stage = $state("");

  let un: UnlistenFn[] = [];

  async function pickFile() {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [{ name: t("Modrinth pack or CurseForge zip"), extensions: ["mrpack", "zip"] }],
      });
      if (typeof picked === "string" && picked) {
        path = picked;
        const base = picked.split(/[\\/]/).pop() ?? "";
        name = base.replace(/\.(mrpack|zip)$/i, "");
      }
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function dropPath(p: string) {
    if (!p) return;
    path = p;
    const base = p.split(/[\\/]/).pop() ?? "";
    name = base.replace(/\.(mrpack|zip)$/i, "");
  }

  function setupDrop(): () => void {
    if (!isTauri()) return () => {};
    let un = () => {};
    void getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === "over" || event.payload.type === "enter") {
        dragging = true;
      } else if (event.payload.type === "drop") {
        dragging = false;
        dropPath(event.payload.paths[0]);
      } else if (event.payload.type === "leave") {
        dragging = false;
      }
    }).then((fn) => (un = fn));
    return () => un();
  }

  async function doImport() {
    if (!path) {
      toast(t("Pick a pack file first"), "error");
      return;
    }
    importing = true;
    stage = t("extracting pack…");
    try {
      const meta = await importerApi.import({
        path,
        name: name.trim() || null,
        java,
        min_ram: minRam,
        max_ram: maxRam,
        accept_eula: acceptEula,
      });
      toast(t('Imported "{name}" ({n} warnings)', { name: meta.name, n: meta.warnings?.length ?? 0 }), "success");
      navigate(`/s/${meta.id}`);
    } catch (e) {
      toast(t("Import failed: {e}", { e: String(e) }), "error");
      importing = false;
    }
  }

  onMount(() => {
    (async () => {
      minRam = settings.min_ram_default;
      maxRam = settings.max_ram_default;
      try {
        const javas = await settingsApi.detectJava();
        if (javas.length) java = javas.some((j) => j.major === 21) ? 21 : javas[0].major;
      } catch {
        /* ignore */
      }
      const l1 = await listen<{ id: string; stage: string }>("imp:progress", (ev) => {
        stage = ev.payload.stage;
      });
      un.push(l1);
      un.push(setupDrop());
    })();
    return () => un.forEach((u) => u());
  });
</script>

<div
  class="relative max-w-2xl mx-auto px-5 py-6"
  ondragover={(e) => { e.preventDefault(); dragging = true; }}
  ondragenter={(e) => { e.preventDefault(); dragging = true; }}
  ondragleave={() => (dragging = false)}
  ondrop={(e) => {
    e.preventDefault();
    dragging = false;
    const p = pathsFromDataTransfer(e.dataTransfer as DataTransfer)[0];
    if (p) dropPath(p);
  }}
>
  {#if dragging}
    <div class="absolute inset-2 z-10 rounded-2xl border-2 border-dashed border-brand-500 bg-brand-500/10 flex items-center justify-center pointer-events-none">
      <div class="text-center">
        <FileArchive size={28} class="mx-auto text-brand-400 mb-2" />
        <p class="text-sm font-medium text-slate-100">{t("Drop the pack file to import")}</p>
        <p class="text-xs text-fg-dim">.mrpack or .zip</p>
      </div>
    </div>
  {/if}
  <div class="flex items-center gap-3 mb-6">
    <div class="w-10 h-10 rounded-lg bg-brand-500/15 border border-brand-500/30 flex items-center justify-center text-brand-500">
      <Package size={18} />
    </div>
    <div>
      <h1 class="text-xl font-bold text-slate-50">{t("Import Server Pack")}</h1>
      <p class="text-sm text-fg-dim mt-0.5">
        {@html t("Install a Modrinth ({mrpack}) or CurseForge ({zip}) modpack as a full server.", {
          mrpack: "<code class=\"text-brand-300\">.mrpack</code>",
          zip: "<code class=\"text-brand-300\">.zip</code>",
        })}
      </p>
    </div>
  </div>

  <Card class="pb-2">
    <div class="flex flex-col gap-5">
      <div class="flex gap-2 items-end">
        <div class="flex-1">
          <div class="text-xs font-medium text-fg-dim uppercase tracking-wide">{t("Pack file")}</div>
          <div class={"mt-1.5 text-sm " + (path ? "font-mono text-slate-100 truncate" : "text-fg-dim")}>
            {path || t("No file selected")}
          </div>
        </div>
        <Button variant="outline" onclick={pickFile}><FileArchive size={16} /> {t("Choose file…")}</Button>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <Input label={t("Server name")} bind:value={name} placeholder="My Modpack" />
        <div class="flex flex-col gap-1.5">
          <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">{t("Java")}</span>
          <Select
            value={String(java)}
            options={[
              { value: "21", label: t("Java 21 (modern)") },
              { value: "17", label: t("Java 17") },
              { value: "8", label: t("Java 8 (legacy)") },
            ]}
            onChange={(v) => (java = parseInt(v) || 21)}
          />
        </div>
      </div>

      <div class="grid grid-cols-2 gap-4">
        <Input label={t("Min RAM (MB)")} type="number" value={String(minRam)} onChange={(v) => (minRam = parseInt(v) || 512)} />
        <Input label={t("Max RAM (MB)")} type="number" value={String(maxRam)} onChange={(v) => (maxRam = parseInt(v) || 4096)} />
      </div>

      <Switch label={t("Accept Minecraft EULA")} checked={acceptEula} />

      <Button onclick={doImport} disabled={importing || !path}>
        {#if importing}
          <Loader2 size={16} class="animate-spin" />
        {:else}
          <Package size={16} />
        {/if}
        {t("Install pack")}
      </Button>

      {#if importing}
        <div class="flex items-center gap-2 text-sm text-fg-dim">
          <Spinner class="w-4 h-4" /> {stage}
        </div>
      {/if}
    </div>
  </Card>
</div>