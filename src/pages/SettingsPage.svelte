<script lang="ts">
  import { settings, toast, loadSettings } from "../lib/store.svelte";
  import { settingsApi, autostartApi } from "../lib/api";
  import { open } from "@tauri-apps/plugin-dialog";
  import { onMount } from "svelte";
  import Card from "../lib/ui/Card.svelte";
  import Button from "../lib/ui/Button.svelte";
  import Input from "../lib/ui/Input.svelte";
  import Select from "../lib/ui/Select.svelte";
  import Switch from "../lib/ui/Switch.svelte";
  import type { JavaInstall } from "../lib/types";
  import { THEMES, CUSTOM_THEME_ID, DEFAULT_VARS, type ThemeVars } from "../lib/themes";
  import { FolderOpen, Save, Wrench, Palette, Plus, Search, X } from "lucide-svelte";
  import { t, LANGS } from "../lib/i18n.svelte";

  let javas: JavaInstall[] = $state([]);
  let saving = $state(false);
  let overrideMajor = $state(21);
  let overrideValue = $state("");
  let scanning = $state(false);

  // Settings are grouped so they stay readable as the page grows.
  const CATEGORIES = [
    { id: "all", label: "All" },
    { id: "general", label: "General" },
    { id: "servers", label: "Servers" },
    { id: "java", label: "Java" },
    { id: "interface", label: "Interface" },
    { id: "notifications", label: "Notifications" },
  ];
  let cat = $state("all");
  let query = $state("");
  let autostart = $state(false);

  onMount(async () => {
    try {
      autostart = await autostartApi.isEnabled();
    } catch {
      autostart = false;
    }
  });

  async function toggleAutostart(enabled: boolean) {
    try {
      await autostartApi.setEnabled(enabled);
      autostart = enabled;
    } catch (e) {
      toast(String(e), "error");
    }
  }

  /** `keys` are the searchable terms of a section (English + current language). */
  function show(...keys: string[]) {
    if (cat !== "all" && !keys.includes(cat)) return false;
    const q = query.trim().toLowerCase();
    if (!q) return true;
    return keys.some((k) => k.toLowerCase().includes(q));
  }

  const customVars: ThemeVars = $derived.by(() => {
    const base = settings.custom_theme ?? null;
    return {
      surface0: base?.surface0 ?? DEFAULT_VARS.surface0,
      surface1: base?.surface1 ?? DEFAULT_VARS.surface1,
      surface2: base?.surface2 ?? DEFAULT_VARS.surface2,
      surface3: base?.surface3 ?? DEFAULT_VARS.surface3,
      edge: base?.edge ?? DEFAULT_VARS.edge,
      fg: base?.fg ?? DEFAULT_VARS.fg,
      fg_dim: base?.fg_dim ?? DEFAULT_VARS.fg_dim,
      brand: base?.brand ?? DEFAULT_VARS.brand,
      accent: base?.accent ?? DEFAULT_VARS.accent,
      magenta: base?.magenta ?? DEFAULT_VARS.magenta,
    };
  });

  function setCustomVar(key: keyof ThemeVars, value: string) {
    settings.custom_theme = { ...customVars, [key]: value };
  }

  function resetCustom() {
    settings.custom_theme = { ...DEFAULT_VARS };
  }

  async function pickDir() {
    try {
      const dir = await open({ directory: true, multiple: false });
      if (typeof dir === "string" && dir) {
        settings.servers_dir = dir;
      }
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function scanJava() {
    scanning = true;
    try {
      javas = await settingsApi.detectJava();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      scanning = false;
    }
  }

  async function setOverride() {
    if (!overrideValue.trim()) return;
    settings.java_overrides = { ...settings.java_overrides, [String(overrideMajor)]: overrideValue.trim() };
    overrideValue = "";
  }

  async function save() {
    saving = true;
    try {
      const updated = await settingsApi.set({ ...settings });
      Object.assign(settings, updated);
      toast(t("Settings saved"), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      saving = false;
    }
  }

  onMount(() => loadSettings());
</script>

<div class="max-w-3xl mx-auto px-5 py-6 flex flex-col gap-4">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-xl font-bold text-slate-50">{t("Settings")}</h1>
      <p class="text-sm text-fg-dim mt-0.5">{t("Storage, Java and default resources.")}</p>
    </div>
    <Button onclick={save} disabled={saving}><Save size={16} /> {t("Save")}</Button>
  </div>

  <div class="flex flex-col gap-2">
    <div class="flex items-center gap-2 px-3 py-2 rounded-xl bg-surface-2/60 border border-edge">
      <Search size={14} class="text-fg-dim shrink-0" />
      <input
        class="flex-1 bg-transparent text-sm text-slate-100 placeholder:text-fg-dim/70 focus:outline-none"
        placeholder={t("Search settings…")}
        bind:value={query}
      />
      {#if query}
        <button class="text-fg-dim hover:text-slate-200 cursor-pointer" title={t("Clear")} onclick={() => (query = "")}>
          <X size={14} />
        </button>
      {/if}
    </div>
    <div class="flex flex-wrap gap-1.5">
      {#each CATEGORIES as c (c.id)}
        <button
          class="px-2.5 py-1 rounded-lg text-xs border cursor-pointer transition-colors {cat === c.id
            ? 'border-brand-500 bg-brand-500/15 text-slate-100'
            : 'border-edge bg-surface-2/60 text-fg-dim hover:text-slate-200'}"
          onclick={() => (cat = c.id)}
        >
          {t(c.label)}
        </button>
      {/each}
    </div>
  </div>

  <div class:hidden={!show("general", "language", "язык")}>
  <Card title={t("Language")} subtitle={t("Language of the interface")}>
    <Select
      label=""
      value={settings.lang}
      options={LANGS.map((l) => ({ value: l.code, label: l.label }))}
      onChange={(v) => (settings.lang = v as "en" | "ru")}
    />
  </Card>
  </div>

  <div class:hidden={!show("servers", "servers directory", "папка", "defaults", "ram", "curseforge", "api")}>
  <Card title={t("Servers Directory")} subtitle={t("Where all server folders live")}>
    <div class="flex gap-2">
      <div class="flex-1">
        <Input label="" value={settings.servers_dir} onChange={(v) => (settings.servers_dir = v)} placeholder="/path/to/servers" />
      </div>
      <div class="flex items-end">
        <Button variant="outline" onclick={pickDir}>
          <FolderOpen size={16} /> {t("Choose folder…")}
        </Button>
      </div>
    </div>
    <p class="text-xs text-fg-dim mt-2">
      {t("Each server (and modpacks you import) will be created inside this folder.")}
    </p>
  </Card>

  <Card title={t("Defaults")} subtitle={t("Used when creating a new server")}>
    <div class="grid grid-cols-2 gap-4">
      <Input label={t("Default min RAM (MB)")} type="number" value={String(settings.min_ram_default)} onChange={(v) => (settings.min_ram_default = parseInt(v) || 2048)} />
      <Input label={t("Default max RAM (MB)")} type="number" value={String(settings.max_ram_default)} onChange={(v) => (settings.max_ram_default = parseInt(v) || 4096)} />
    </div>
  </Card>

  <Card title={t("CurseForge API Key")} subtitle={t("Optional — used for CurseForge modpack search on the Import page")}>
    <Input label="" type="password" placeholder="CFCoreApiToken (avoid X-Api-Key prefix)" value={settings.curseforge_api_key ?? ""} onChange={(v) => (settings.curseforge_api_key = v || null)} />
    <p class="text-xs text-fg-dim mt-2">
      {t("Get one at")}
      <button class="text-brand-500 underline cursor-pointer" onclick={() => import("../lib/api").then(({ miscApi }) => miscApi.openUrl("https://console.curseforge.com/"))}>
        console.curseforge.com
      </button>
      . {t("Leave empty to use Modrinth only.")}
    </p>
  </Card>
  </div>

  <div class:hidden={!show("java", "jvm", "java")}>
  <Card
    title={t("Java")}
    subtitle={t("Detected JVMs plus manual overrides per major version")}
  >
    {#snippet actions()}
      <Button variant="outline" size="sm" onclick={scanJava} disabled={scanning}>
        <Wrench size={14} /> {scanning ? t("Scanning…") : t("Re-scan")}
      </Button>
    {/snippet}
    {#if javas.length === 0}
      <p class="text-sm text-fg-dim mb-2">{t("Run a scan to detect installed Java runtimes.")}</p>
    {:else}
      <ul class="flex flex-col gap-1 mb-3">
        {#each javas as j (j.path + j.major)}
          <li class="flex items-center justify-between px-3 py-2 rounded-lg bg-surface-2/60 border border-edge">
            <div>
              <span class="text-sm font-mono text-slate-100">{t("Java {major}", { major: j.major })}</span>
              <span class="text-xs text-fg-dim ml-2">{j.vendor} · {j.source}</span>
            </div>
            <code class="text-[11px] text-fg-dim truncate max-w-[55%]">{j.path}</code>
          </li>
        {/each}
      </ul>
    {/if}

    <div class="rounded-lg border border-edge bg-surface-2/40 p-3 flex flex-wrap items-end gap-3">
      <div class="w-28">
        <Input label={t("Major")} type="number" value={String(overrideMajor)} onChange={(v) => (overrideMajor = parseInt(v) || 21)} />
      </div>
      <div class="flex-1 min-w-[220px]">
        <Input label={t("Java executable path (override)")} bind:value={overrideValue} placeholder="/opt/jdk-21/bin/java" />
      </div>
      <div class="flex items-end">
        <Button variant="subtle" onclick={setOverride}>{t("Add override")}</Button>
      </div>
    </div>
    {#if Object.keys(settings.java_overrides || {}).length > 0}
      <ul class="flex flex-col gap-1 mt-3">
        {#each Object.entries(settings.java_overrides || {}) as [major, path]}
          <li class="flex items-center justify-between px-3 py-1.5 rounded-lg bg-surface-2/60 border border-edge">
            <span class="text-xs font-mono text-slate-100">{t("Java {major}", { major })}</span>
            <code class="text-[11px] text-fg-dim truncate max-w-[60%]">{path}</code>
            <button
              class="text-xs text-red-400 hover:text-red-300 cursor-pointer"
              onclick={() => {
                const { [major]: _drop, ...rest } = settings.java_overrides;
                settings.java_overrides = rest;
              }}
            >
              {t("remove")}
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </Card>
  </div>

  <div class:hidden={!show("interface", "ui", "theme", "tray", "тема", "интерфейс")}>
  <Card title={t("Interface")} subtitle={t("Zoom level, color palette and tray behaviour")}>
    <div class="grid grid-cols-1 sm:grid-cols-2 gap-4">
      <Select
        label={t("UI scale")}
        value={String(settings.ui_scale ?? 100)}
        options={[
          { value: "75", label: "75%" },
          { value: "90", label: "90%" },
          { value: "100", label: t("100% (default)") },
          { value: "110", label: "110%" },
          { value: "125", label: "125%" },
          { value: "150", label: "150%" },
          { value: "175", label: "175%" },
          { value: "200", label: "200%" },
        ]}
        onChange={(v) => (settings.ui_scale = parseInt(v) || 100)}
      />
      <Switch
        label={t("Keep running in the tray")}
        bind:checked={settings.tray_enabled}
        hint={t("Closing the window hides to the system tray, so servers keep running.")}
      />
      <div class="col-span-full">
        <Switch
          label={t("Launch app at system startup")}
          checked={autostart}
          onChange={toggleAutostart}
          hint={t("Open MC Server Manager when you log in, then start the servers marked for auto-start.")}
        />
      </div>
    </div>

    <div class="mt-4 flex flex-col gap-1.5">
      <div class="flex items-center gap-1.5 text-xs font-medium text-fg-dim uppercase tracking-wide">
        <Palette size={13} /> <span>{t("Color palette")}</span>
        <span class="normal-case tracking-normal text-fg-dim/80">{t("— applied instantly")}</span>
      </div>
      <div class="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-4 gap-2">
        {#each THEMES as theme (theme.id)}
          <button
            class="flex items-center gap-2.5 rounded-lg border px-3 py-2 text-left cursor-pointer transition-colors {settings.theme === theme.id ? 'border-brand-500 bg-brand-500/10' : 'border-edge bg-surface-2/60 hover:bg-surface-2'}"
            onclick={() => (settings.theme = theme.id)}
          >
            <span
              class="w-6 h-6 rounded-md ring-1 ring-inset ring-black/30 shrink-0"
              style="background: linear-gradient(135deg, {theme.vars.surface1} 0%, {theme.vars.surface2} 45%, {theme.vars.edge} 100%); box-shadow: inset 0 0 0 6px {theme.vars.surface0};"
            ></span>
            <span class="flex flex-col min-w-0">
              <span class="text-sm text-slate-100 truncate">{theme.label}</span>
              <span class="flex items-center gap-1 mt-0.5">
                <span class="w-3 h-3 rounded-full" style="background: {theme.vars.brand}"></span>
                <span class="w-3 h-3 rounded-full" style="background: {theme.vars.accent}"></span>
                <span class="w-3 h-3 rounded-full" style="background: {theme.vars.magenta}"></span>
              </span>
            </span>
          </button>
        {/each}
        <button
          class="flex items-center gap-2.5 rounded-lg border px-3 py-2 text-left cursor-pointer transition-colors {settings.theme === CUSTOM_THEME_ID ? 'border-brand-500 bg-brand-500/10' : 'border-dashed border-edge bg-surface-2/60 hover:bg-surface-2'}"
          onclick={() => (settings.theme = CUSTOM_THEME_ID)}
        >
          <span class="w-6 h-6 rounded-md ring-1 ring-inset ring-black/30 shrink-0 flex items-center justify-center"
            style="background: var(--color-surface-1); color: var(--color-brand-500);"><Plus size={14} /></span>
          <span class="flex flex-col min-w-0">
            <span class="text-sm text-slate-100">{t("Custom")}</span>
            <span class="text-xs text-fg-dim">{t("make your own")}</span>
          </span>
        </button>
      </div>
      {#if settings.theme === CUSTOM_THEME_ID}
        <div class="mt-2 rounded-lg border border-edge bg-surface-2/40 p-4">
          <div class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-5 gap-3">
            {#each [
              ["surface0", t("Background")],
              ["surface1", t("Panel")],
              ["surface2", t("Input field")],
              ["surface3", t("Hover")],
              ["edge", t("Border")],
              ["fg", t("Text")],
              ["fg_dim", t("Muted text")],
              ["brand", t("Primary")],
              ["accent", t("Accent")],
              ["magenta", t("Highlight")],
            ] as [key, label]}
              <label class="flex flex-col gap-1">
                <span class="text-[11px] text-fg-dim">{label}</span>
                <span class="flex items-center gap-1.5">
                  <input
                    type="color"
                    value={customVars[key as keyof ThemeVars]}
                    oninput={(e) => setCustomVar(key as keyof ThemeVars, (e.currentTarget as HTMLInputElement).value)}
                    class="w-8 h-7 rounded-md border border-edge bg-transparent cursor-pointer"
                  />
                  <code class="text-[11px] text-fg-dim font-mono">{customVars[key as keyof ThemeVars]}</code>
                </span>
              </label>
            {/each}
          </div>
          <div class="flex items-center justify-end gap-2 mt-4">
            <Button variant="outline" size="sm" onclick={resetCustom}>{t("Reset to default")}</Button>
          </div>
        </div>
      {/if}
    </div>
    <p class="text-xs text-fg-dim mt-2">
      {t("Scale is applied immediately via native webview zoom — no restart needed.")}
    </p>
  </Card>
  </div>

  <div class:hidden={!show("notifications", "notify", "desktop", "уведом")}>
  <Card title={t("Notifications")} subtitle={t("Where to hear about server events")}>
    <div class="flex flex-col gap-3">
      <Switch
        label={t("Desktop notifications")}
        bind:checked={settings.notify_desktop}
        hint={t("Uses the native notification daemon of your system.")}
      />

      <div class="grid grid-cols-2 sm:grid-cols-4 gap-3 pt-1">
        <Switch label={t("Notify on start")} bind:checked={settings.notify_on_start} />
        <Switch label={t("Notify on stop")} bind:checked={settings.notify_on_stop} />
        <Switch label={t("Notify on crash")} bind:checked={settings.notify_on_crash} />
        <Switch label={t("Notify on backup")} bind:checked={settings.notify_on_backup} />
      </div>
    </div>
  </Card>
  </div>
</div>