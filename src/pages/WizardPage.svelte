<script lang="ts">
  import Button from "../lib/ui/Button.svelte";
  import Input from "../lib/ui/Input.svelte";
  import Select from "../lib/ui/Select.svelte";
  import Switch from "../lib/ui/Switch.svelte";
  import Progress from "../lib/ui/Progress.svelte";
  import { serversApi, settingsApi, versionsApi } from "../lib/api";
  import { settings, toast } from "../lib/store.svelte";
  import { navigate } from "../lib/router.svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import type { JavaInstall } from "../lib/types";
  import { Shield, Loader2 } from "lucide-svelte";
  import { t } from "../lib/i18n.svelte";

  const query = new URLSearchParams(location.hash.split("?")[1] ?? "");

  let name = $state("");
  let core = $state(query.get("core") === "velocity" ? "velocity" : "paper");
  let mcVersion = $state("");
  let loaderVersion = $state("");
  let java = $state(21);
  let javaTouched = $state(false);
  let minRam = $state(512);
  let maxRam = $state(4096);
  let acceptEula = $state(true);

  let versions = $state<string[]>([]);
  let loaders = $state<string[]>([]);
  let javas = $state<JavaInstall[]>([]);
  let loadingVersions = $state(false);
  let loadingLoaders = $state(false);
  let customVersionOpen = $state(false);
  let importing = $state(false);
  let progress = $state(0);
  let stage = $state("");
  let includeSnapshots = $state(false);

  const needsLoader = () => core === "forge" || core === "neoforge" || core === "fabric" || core === "quilt";

  // Cores that run Minecraft snapshots (backed by the Mojang version manifest).
  const snapshotsSupported = () => core === "vanilla" || core === "fabric" || core === "quilt";

  const versionLabel = () => (core === "velocity" ? t("Velocity version") : t("Minecraft version"));

  const loaderOptions = $derived(
    loaders.map((v, i) => ({
      value: v,
      label: i === 0 ? `${v}  * ${t("recommended")}` : v,
    }))
  );

  const coreOptions = [
    { value: "velocity", label: "Velocity (proxy)" },
    { value: "paper", label: "Paper" },
    { value: "purpur", label: "Purpur" },
    { value: "spigot", label: "Spigot" },
    { value: "bukkit", label: "Bukkit" },
    { value: "vanilla", label: "Vanilla" },
    { value: "fabric", label: "Fabric" },
    { value: "quilt", label: "Quilt" },
    { value: "forge", label: "Forge" },
    { value: "neoforge", label: "NeoForge" },
  ];

  const javaMajorRequired = $derived.by(() => {
    if (core === "velocity" || core === "paper" || core === "purpur" || core === "vanilla") return 21;
    if (core === "spigot" || core === "bukkit") return 17;
    return 21;
  });

  async function loadVersions() {
    loadingVersions = true;
    loaderVersion = "";
    try {
      const v = await versionsApi.coreVersions(core, snapshotsSupported() && includeSnapshots);
      versions = v;
      mcVersion = v[0] ?? "";
    } catch (e) {
      toast(String(e), "error");
    } finally {
      loadingVersions = false;
    }
  }

  async function loadLoaders() {
    if (!needsLoader()) {
      loaders = [];
      loaderVersion = "";
      return;
    }
    if (!mcVersion) return;
    loadingLoaders = true;
    loaderVersion = "";
    try {
      const l = await versionsApi.loaders(core, mcVersion);
      loaders = l;
      loaderVersion = l[0] ?? "";
    } catch (e) {
      toast(String(e), "error");
    } finally {
      loadingLoaders = false;
    }
  }

  let unlisteners: UnlistenFn[] = [];

  onMount(() => {
    (async () => {
      minRam = settings.min_ram_default;
      maxRam = settings.max_ram_default;
      try {
        javas = await settingsApi.detectJava();
        if (javas.length && !javaTouched) {
          const rec = javaMajorRequired;
          java =
            (javas.find((j) => j.major === rec) ?? javas.find((j) => j.major >= 21) ?? javas[0]).major;
        }
      } catch {
        /* ignore */
      }
      await loadVersions();

      const d1 = await listen<{ key: string; percent: number }>("download:progress", (ev) => {
        if (ev.payload.key.startsWith("create:")) progress = ev.payload.percent;
      });
      const d2 = await listen<{ id: string; stage: string }>("server-stage", (ev) => {
        stage = ev.payload.stage;
      });
      unlisteners.push(d1, d2);
    })();
    return () => unlisteners.forEach((u) => u());
  });

  async function onSubmit() {
    if (!name.trim()) {
      toast(t("Server name is required"), "error");
      return;
    }
    if (needsLoader() && !loaderVersion.trim()) {
      toast(t("Select a {core} loader version first", { core }), "error");
      return;
    }
    importing = true;
    progress = 0;
    stage = "starting";
    try {
      const meta = await serversApi.create({
        name: name.trim(),
        java,
        core,
        mc_version: mcVersion,
        loader_version: needsLoader() ? loaderVersion : null,
        min_ram: minRam,
        max_ram: maxRam,
        accept_eula: acceptEula,
      });
      toast(t('Server "{name}" created', { name: meta.name }), "success");
      navigate(`/s/${meta.id}`);
    } catch (e) {
      toast(t("Failed to create: {e}", { e: String(e) }), "error");
      importing = false;
    }
  }
</script>

<div class="max-w-2xl mx-auto px-5 py-6">
  <h1 class="text-xl font-bold text-slate-50 mb-1">{t("New Server")}</h1>
  <p class="text-sm text-fg-dim mb-6">{t("Choose a core, version and resources to spin up a server.")}</p>

  <div class="rounded-xl border border-edge bg-surface-1 p-5 flex flex-col gap-5">
    <Input label={t("Server name")} bind:value={name} placeholder="My Survival Server" />

    <div class="flex flex-col gap-1.5">
      <span class="flex items-center justify-between text-xs font-medium text-fg-dim uppercase tracking-wide">
        {versionLabel()}
        {#if loadingVersions}
          <span class="flex items-center gap-1 normal-case tracking-normal text-fg-dim"><Loader2 size={12} class="animate-spin" /> {t("loading…")}</span>
        {/if}
      </span>
      {#if versions.length === 0}
        <button class="text-left text-sm text-brand-500 underline cursor-pointer" onclick={loadVersions}>
          {t("Load versions")}
        </button>
      {:else}
        <Select
          label=""
          value={mcVersion}
          options={versions.map((v) => ({ value: v, label: v }))}
          onChange={(v) => {
            mcVersion = v;
            if (needsLoader()) loadLoaders();
          }}
        />
        {#if snapshotsSupported()}
          <Switch
            label={t("Include snapshots")}
            bind:checked={includeSnapshots}
            onChange={() => loadVersions()}
          />
        {/if}
        <button
          class="text-left text-xs text-fg-dim underline cursor-pointer hover:text-white transition-colors"
          onclick={() => (customVersionOpen = !customVersionOpen)}
        >
          {customVersionOpen ? t("Hide custom version input") : t("… or type a custom version")}
        </button>
        {#if customVersionOpen}
          <div class="mt-1">
            <Input label="" type="text" bind:value={mcVersion} placeholder={t("e.g. 1.20.6-pre3")} />
            <p class="text-[11px] text-fg-dim mt-1">
              {t("Typed versions override the dropdown. The core must provide a build for it.")}
            </p>
          </div>
        {/if}
      {/if}
    </div>

    <Select
      label={t("Core")}
      bind:value={core}
      options={coreOptions}
      onChange={(v) => {
        if (v === core) return;
        core = v;
        customVersionOpen = false;
        loadVersions();
      }}
    />

    {#if needsLoader()}
      <div class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">
          {t("{core} loader version", { core })}
          <span class="normal-case tracking-normal text-fg-dim/80">{t("— recommended marked with *")}</span>
        </span>
        {#if loadingLoaders}
          <div class="flex items-center gap-2 text-sm text-fg-dim"><Loader2 size={14} class="animate-spin" /> {t("Loading…")}</div>
        {:else if loaderOptions.length === 0}
          <div class="text-sm text-fg-dim">{t("No loader versions found for {mc}.", { mc: mcVersion })}</div>
        {:else}
          <Select
            label=""
            value={loaderVersion}
            options={loaderOptions}
            onChange={(v) => (loaderVersion = v)}
          />
        {/if}
        <p class="text-[11px] text-fg-dim">
          {mcVersion || t("(select a Minecraft version first)")} — {t("The top entry is the latest recommended build.")}
        </p>
      </div>
    {:else if core === "velocity"}
      <p class="text-xs text-fg-dim rounded-lg border border-edge bg-surface-2/40 px-3 py-2">
        {t("Velocity proxies have their own versioning — the version selected above is used directly.")}
      </p>
    {:else}
      <p class="text-xs text-fg-dim rounded-lg border border-edge bg-surface-2/40 px-3 py-2">
        {t("{core} always uses its latest recommended build for the selected Minecraft version — no build picker needed.", { core })}
      </p>
    {/if}

    <div class="flex flex-col gap-1.5">
      <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">{t("Java version")}</span>
      <div class="flex gap-2">
        {#each Array.from(new Set(javas.map((j) => j.major))).sort() as major}
          <button
            class="px-3 py-1.5 rounded-lg text-xs border cursor-pointer transition-colors {java === major ? 'border-brand-500 bg-brand-500/15 text-brand-300' : 'border-edge bg-surface-2 text-fg-dim hover:text-white'}"
            onclick={() => { java = major; javaTouched = true; }}
          >
            {t("Java {major}", { major })}
          </button>
        {/each}
        {#if javas.length === 0}
          <span class="text-sm text-fg-dim">{t("No Java detected — install a JRE/JDK (see Settings).")}</span>
        {/if}
      </div>
    </div>

    <div class="grid grid-cols-2 gap-4">
      <div class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">{t("Min RAM (MB)")}</span>
        <input
          type="number"
          step="512"
          min="512"
          bind:value={minRam}
          class="bg-surface-2 border border-edge rounded-lg px-3 py-2 text-sm text-slate-100 outline-none focus:border-brand-500 transition"
        />
      </div>
      <div class="flex flex-col gap-1.5">
        <span class="text-xs font-medium text-fg-dim uppercase tracking-wide">{t("Max RAM (MB)")}</span>
        <input
          type="number"
          step="512"
          min="512"
          bind:value={maxRam}
          class="bg-surface-2 border border-edge rounded-lg px-3 py-2 text-sm text-slate-100 outline-none focus:border-brand-500 transition"
        />
      </div>
    </div>

    <Switch label={t("Accept Minecraft EULA")} bind:checked={acceptEula} />

    <Button onclick={onSubmit} disabled={importing || loadingVersions}>
      {#if importing}
        <Loader2 size={16} class="animate-spin" />
      {:else}
        <Shield size={16} />
      {/if}
      {t("Create Server")}
    </Button>
  </div>

  {#if importing}
    <div class="mt-4 rounded-xl border border-edge bg-surface-1 p-4">
      <div class="flex items-center justify-between text-xs text-fg-dim mb-2">
        <span>{stage}</span>
        <span>{Math.round(progress)}%</span>
      </div>
      <Progress value={progress} />
    </div>
  {/if}
</div>