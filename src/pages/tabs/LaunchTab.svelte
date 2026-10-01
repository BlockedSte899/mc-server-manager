<script lang="ts">
  import { serversApi, versionsApi } from "../../lib/api";
  import { toast, patchServerMeta, servers, statuses, downloads } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Switch from "../../lib/ui/Switch.svelte";
  import Input from "../../lib/ui/Input.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import Dialog from "../../lib/ui/Dialog.svelte";
  import ConfirmDialog from "../../lib/ui/ConfirmDialog.svelte";
  import Spinner from "../../lib/ui/Spinner.svelte";
  import { onMount } from "svelte";
  import type { ScheduleRule } from "../../lib/types";
  import {
    Trash2,
    Plus,
    Terminal,
    RefreshCw,
    Cpu,
    CalendarClock,
    Zap,
    TriangleAlert,
  } from "lucide-svelte";
  import { t } from "../../lib/i18n.svelte";

  let { id }: { id: string } = $props();

  const meta = $derived(servers.find((s) => s.meta.id === id)?.meta ?? null);
  const running = $derived(statuses[id] === "running" || statuses[id] === "stopping");

  const PRESETS = [
    { value: "auto", labelKey: "No extra flags" },
    { value: "aikar", labelKey: "Aikar's flags (Paper)" },
    { value: "vanilla", labelKey: "G1GC tuning (vanilla/Forge)" },
    { value: "custom", labelKey: "Custom" },
  ];
  const DAYS = [
    { value: "every", label: t("Every day") },
    { value: "mon", label: t("Mon") },
    { value: "tue", label: t("Tue") },
    { value: "wed", label: t("Wed") },
    { value: "thu", label: t("Thu") },
    { value: "fri", label: t("Fri") },
    { value: "sat", label: t("Sat") },
    { value: "sun", label: t("Sun") },
  ];

  let command = $state("");
  let saving = $state(false);
  let minRam = $state("");
  let maxRam = $state("");
  let jvmPreset = $state("auto");
  let jvmArgs = $state("");
  let autoStart = $state(false);
  let watchdog = $state(false);
  let rules = $state<ScheduleRule[]>([]);
  let loadedFor = "";

  // Pull the editable fields out of the cached meta once per server.
  $effect(() => {
    if (meta && meta.id !== loadedFor) {
      loadedFor = meta.id;
      minRam = String(meta.min_ram);
      maxRam = String(meta.max_ram);
      jvmPreset = meta.jvm_preset || "auto";
      jvmArgs = meta.jvm_args || "";
      autoStart = meta.auto_start_on_boot;
      watchdog = meta.auto_restart_on_crash;
      rules = [...(meta.schedule ?? [])];
      void loadCommand();
    }
  });

  async function loadCommand() {
    try {
      command = await serversApi.launchCommand(id);
    } catch {
      command = t("Unavailable");
    }
  }

  async function save(extra: Record<string, unknown> = {}) {
    saving = true;
    try {
      const updated = await serversApi.updateConfig(id, {
        jvm_preset: jvmPreset,
        jvm_args: jvmArgs,
        auto_start_on_boot: autoStart,
        auto_restart_on_crash: watchdog,
        schedule: rules,
        min_ram: Math.max(128, parseInt(minRam) || 512),
        max_ram: Math.max(256, parseInt(maxRam) || 2048),
        ...extra,
      });
      minRam = String(updated.min_ram);
      maxRam = String(updated.max_ram);
      patchServerMeta(id, updated);
      void loadCommand();
      toast(t("Launch settings saved"), "success");
    } catch (e) {
      toast(t("Could not save: {e}", { e: String(e) }), "error");
    } finally {
      saving = false;
    }
  }

  function addRule() {
    rules = [...rules, { day: "every", hour: 4, minute: 0, action: "stop" }];
  }

  function updateRule(i: number, patch: Partial<ScheduleRule>) {
    rules = rules.map((r, idx) => (idx === i ? { ...r, ...patch } : r));
  }

  function removeRule(i: number) {
    rules = rules.filter((_, idx) => idx !== i);
  }

  // --- version switcher ---
  let versionOpen = $state(false);
  let versions = $state<string[]>([]);
  let loaders = $state<string[]>([]);
  let loadingVersions = $state(false);
  let applying = $state(false);
  let pickedVersion = $state("");
  let pickedLoader = $state("");

  const needsLoader = $derived(
    meta ? ["forge", "neoforge", "fabric", "quilt"].includes(meta.core) : false,
  );

  async function openVersions() {
    versionOpen = true;
    pickedVersion = meta?.mc_version ?? "";
    pickedLoader = meta?.loader_version ?? "";
    versions = [];
    loaders = [];
    loadingVersions = true;
    try {
      versions = await versionsApi.coreVersions(meta?.core ?? "paper", false);
      if (needsLoader && pickedVersion) await loadLoaders(pickedVersion);
    } catch (e) {
      toast(t("Could not load versions: {e}", { e: String(e) }), "error");
    } finally {
      loadingVersions = false;
    }
  }

  async function loadLoaders(mc: string) {
    if (!meta) return;
    try {
      loaders = await versionsApi.loaders(meta.core, mc);
      if (!loaders.includes(pickedLoader)) pickedLoader = loaders[0] ?? "";
    } catch {
      loaders = [];
    }
  }

  const coreDownload = $derived(
    meta && downloads[`server:${meta.id}:core`]
      ? downloads[`server:${meta.id}:core`].percent
      : null,
  );

  // A real version switch (not just a loader bump on the same MC version)
  // requires an explicit confirmation.
  const versionChanged = $derived(
    pickedVersion !== "" && pickedVersion !== (meta?.mc_version ?? ""),
  );
  let confirmOpen = $state(false);

  async function applyVersion() {
    if (!pickedVersion) return;
    applying = true;
    try {
      const updated = await serversApi.changeVersion(
        id,
        pickedVersion,
        needsLoader ? pickedLoader || null : null,
      );
      patchServerMeta(id, updated);
      versionOpen = false;
      toast(t("Server is now on {v}", { v: updated.mc_version }), "success");
      void loadCommand();
    } catch (e) {
      toast(t("Could not change version: {e}", { e: String(e) }), "error");
    } finally {
      applying = false;
    }
  }
</script>

<div class="flex flex-col gap-4">
  <Card title={t("Memory")} subtitle={t("Heap allocated to the server process")}>
    <div class="flex flex-col gap-3">
      <div class="grid grid-cols-2 gap-3">
        <Input
          label={t("Minimum RAM (MB)")}
          type="number"
          value={minRam}
          oninput={(v) => (minRam = v)}
        />
        <Input
          label={t("Maximum RAM (MB)")}
          type="number"
          value={maxRam}
          oninput={(v) => (maxRam = v)}
        />
      </div>
      <div class="flex justify-end">
        <Button size="sm" disabled={saving} onclick={() => void save()}>
          {t("Save")}
        </Button>
      </div>
    </div>
  </Card>

  <Card title={t("JVM flags")} subtitle={t("Tuning applied when the server starts")}>
    <div class="flex flex-col gap-3">
      <Select
        label={t("Preset")}
        value={jvmPreset}
        options={PRESETS.map((p) => ({ value: p.value, label: t(p.labelKey) }))}
        oninput={(v) => {
          jvmPreset = v;
          void save({ jvm_preset: v });
        }}
      />
      {#if jvmPreset === "custom"}
        <div class="flex flex-col gap-1">
          <span class="text-xs text-fg-dim">{t("Custom JVM arguments")}</span>
          <textarea
            class="w-full h-24 px-3 py-2 rounded-lg bg-surface-2 border border-edge text-sm text-slate-100 font-mono focus:outline-none focus:border-brand-500"
            placeholder="-XX:+UseZGC -XX:MaxDirectMemorySize=1G"
            value={jvmArgs}
            oninput={(e) => (jvmArgs = e.currentTarget.value)}
          ></textarea>
          <div class="flex justify-end">
            <Button size="sm" disabled={saving} onclick={() => void save()}>
              {t("Save")}
            </Button>
          </div>
        </div>
      {:else}
        <p class="text-xs text-fg-dim">{t("These flags are appended to the launch command.")}</p>
      {/if}

      <div class="rounded-lg border border-edge bg-surface-2/60 p-3">
        <div class="flex items-center gap-2 mb-1.5">
          <Terminal size={13} class="text-fg-dim" />
          <span class="text-xs text-fg-dim">{t("Launch command")}</span>
        </div>
        <code class="text-[11px] leading-relaxed text-accent break-all font-mono">{command}</code>
      </div>
    </div>
  </Card>

  <Card title={t("Automation")} subtitle={t("Start, restart and schedule without touching the app")}>
    <div class="flex flex-col gap-3">
      <Switch
        label={t("Start with the app")}
        hint={t("Launch this server automatically when the app starts")}
        checked={autoStart}
        onChange={(v) => {
          autoStart = v;
          void save({ auto_start_on_boot: v });
        }}
      />
      <Switch
        label={t("Restart on crash")}
        hint={t("Bring the server back up if it dies unexpectedly")}
        checked={watchdog}
        onChange={(v) => {
          watchdog = v;
          void save({ auto_restart_on_crash: v });
        }}
      />

      <div class="border-t border-edge pt-3 mt-1">
        <div class="flex items-center justify-between mb-2">
          <div class="flex items-center gap-2">
            <CalendarClock size={14} class="text-fg-dim" />
            <span class="text-sm font-medium">{t("Schedule")}</span>
          </div>
          <Button size="xs" variant="outline" onclick={addRule}>
            <Plus size={12} /> {t("Add rule")}
          </Button>
        </div>

        {#if rules.length === 0}
          <p class="text-xs text-fg-dim py-2">{t("No rules yet — the server stays up as you leave it.")}</p>
        {:else}
          <div class="flex flex-col gap-2">
            {#each rules as rule, i (i)}
              <div class="flex items-center gap-2 flex-wrap">
                <div class="w-28">
                  <Select
                    size="xs"
                    value={rule.action}
                    options={[
                      { value: "start", label: t("Start") },
                      { value: "stop", label: t("Stop") },
                    ]}
                    oninput={(v) => updateRule(i, { action: v })}
                  />
                </div>
                <div class="w-28">
                  <Select
                    size="xs"
                    value={rule.day}
                    options={DAYS}
                    oninput={(v) => updateRule(i, { day: v })}
                  />
                </div>
                <div class="flex items-center gap-1" title={t("24-hour time")}>
                  <input
                    type="number"
                    min="0"
                    max="23"
                    class="w-14 px-2 py-1 rounded-lg bg-surface-2 border border-edge text-sm text-slate-100 text-center"
                    value={rule.hour}
                    oninput={(e) =>
                      updateRule(i, {
                        hour: Math.min(23, Math.max(0, parseInt(e.currentTarget.value) || 0)),
                      })}
                  />
                  <span class="text-fg-dim text-sm">:</span>
                  <input
                    type="number"
                    min="0"
                    max="59"
                    class="w-14 px-2 py-1 rounded-lg bg-surface-2 border border-edge text-sm text-slate-100 text-center"
                    value={rule.minute}
                    oninput={(e) =>
                      updateRule(i, {
                        minute: Math.min(59, Math.max(0, parseInt(e.currentTarget.value) || 0)),
                      })}
                  />
                </div>
                <button
                  class="text-fg-dim hover:text-red-400 cursor-pointer"
                  title={t("Remove rule")}
                  onclick={() => removeRule(i)}
                >
                  <Trash2 size={14} />
                </button>
              </div>
            {/each}
          </div>
          <div class="flex justify-end mt-2">
            <Button size="sm" disabled={saving} onclick={() => void save()}>
              {t("Save schedule")}
            </Button>
          </div>
        {/if}
      </div>
    </div>
  </Card>

  <Card title={t("Version")} subtitle={t("Switch the core to another Minecraft version")}>
    <div class="flex flex-col gap-3">
      <div class="flex items-center gap-2 px-3 py-2 rounded-lg border border-amber-500/40 bg-amber-500/10">
        <TriangleAlert size={15} class="text-amber-400 shrink-0" />
        <span class="text-sm font-semibold text-amber-300">
          {t("Dangerous: changing the version can make the world and your mods or plugins incompatible.")}
        </span>
      </div>
      <div class="flex items-center justify-between">
        <div class="flex items-center gap-2 text-sm">
          <Cpu size={14} class="text-fg-dim" />
          <span class="text-slate-100">
            {meta?.mc_version ?? "—"}
            {#if meta?.loader_version}
              <span class="text-fg-dim">({meta.loader_version})</span>
            {/if}
          </span>
        </div>
        <Button
          size="xs"
          variant="outline"
          disabled={running}
          title={running ? t("Stop the server first") : t("Change version")}
          onclick={() => void openVersions()}
        >
          <RefreshCw size={12} /> {t("Change version")}
        </Button>
      </div>
      {#if running}
        <p class="text-xs text-amber-400/80">{t("Stop the server before switching versions.")}</p>
      {:else}
        <p class="text-xs text-fg-dim">
          {t("The world and configs are kept — only the server jar is replaced.")}
        </p>
      {/if}
    </div>
  </Card>
</div>

{#if versionOpen}
  <Dialog title={t("Change version")} wide open={versionOpen} onClose={() => (versionOpen = false)}>
    <div class="flex flex-col gap-3">
      {#if loadingVersions}
        <div class="flex items-center justify-center py-6 gap-2 text-sm text-fg-dim">
          <Spinner class="w-4 h-4" /> {t("Loading versions…")}
        </div>
      {:else}
        <Select
          label={t("Minecraft version")}
          value={pickedVersion}
          options={versions.map((v) => ({ value: v, label: v }))}
          oninput={(v) => {
            pickedVersion = v;
            pickedLoader = "";
            if (needsLoader) void loadLoaders(v);
          }}
        />
        {#if needsLoader}
          <Select
            label={t("Loader version")}
            value={pickedLoader}
            options={loaders.map((v) => ({ value: v, label: v }))}
            oninput={(v) => (pickedLoader = v)}
          />
        {/if}

        {#if pickedVersion && pickedVersion !== meta?.mc_version}
          <div class="flex items-start gap-2 px-3 py-2 rounded-lg border border-red-500/40 bg-red-500/10">
            <TriangleAlert size={15} class="text-red-400 shrink-0 mt-0.5" />
            <p class="text-xs text-red-300">
              {t("The server will be moved from {from} to {to}. Make a backup first — a rollback is not guaranteed.", {
                from: meta?.mc_version ?? "—",
                to: pickedVersion,
              })}
            </p>
          </div>
        {/if}
      {/if}

      {#if coreDownload !== null}
        <div class="flex items-center gap-2 text-xs text-fg-dim">
          <Zap size={12} /> {t("Downloading…")} {coreDownload.toFixed(0)}%
        </div>
      {/if}

      <div class="flex justify-end gap-2">
        <Button variant="ghost" size="sm" onclick={() => (versionOpen = false)}>
          {t("Cancel")}
        </Button>
        <Button
          size="sm"
          variant={versionChanged ? "danger" : "primary"}
          disabled={applying || loadingVersions || !pickedVersion}
          onclick={() => (versionChanged ? (confirmOpen = true) : void applyVersion())}
        >
          {applying ? t("Applying…") : versionChanged ? t("I understand, change it") : t("Apply")}
        </Button>
      </div>
    </div>
  </Dialog>
{/if}

<ConfirmDialog
  open={confirmOpen}
  title={t("Change version?")}
  message={t("Backup first. The world and configs stay, but the core will be replaced and there is no automatic rollback.", {
    from: meta?.mc_version ?? "—",
    to: pickedVersion,
  })}
  confirmLabel={t("Change version")}
  danger
  onConfirm={() => {
    confirmOpen = false;
    void applyVersion();
  }}
  onClose={() => (confirmOpen = false)}
/>
