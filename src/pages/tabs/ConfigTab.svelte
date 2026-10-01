<script lang="ts">
  import { propertiesApi, exportApi } from "../../lib/api";
  import { toast } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Switch from "../../lib/ui/Switch.svelte";
  import Input from "../../lib/ui/Input.svelte";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { isTauri } from "../../lib/env";
  import { Save, RotateCcw, Package, ImagePlus } from "lucide-svelte";
  import { pathsFromDataTransfer } from "../../lib/drop";
  import { t } from "../../lib/i18n.svelte";

  let { id }: { id: string } = $props();

  let p = $state<Record<string, string>>({});
  let text = $state("");
  let dirty = $state(false);
  let saving = $state(false);
  let exporting = $state(false);
  let icon = $state<string | null>(null);
  let iconDragging = $state(false);
  let motdText = $state("");
  let motdTextarea: HTMLTextAreaElement | undefined = $state();

  const formatChars = [
    { code: "0", label: "Black", cls: "bg-black border-slate-700" },
    { code: "1", label: "Dark blue", cls: "bg-blue-950" },
    { code: "2", label: "Dark green", cls: "bg-green-900" },
    { code: "3", label: "Dark aqua", cls: "bg-cyan-900" },
    { code: "4", label: "Dark red", cls: "bg-red-900" },
    { code: "5", label: "Dark purple", cls: "bg-purple-900" },
    { code: "6", label: "Gold", cls: "bg-amber-500" },
    { code: "7", label: "Gray", cls: "bg-slate-500" },
    { code: "8", label: "Dark gray", cls: "bg-slate-700" },
    { code: "9", label: "Blue", cls: "bg-blue-600" },
    { code: "a", label: "Green", cls: "bg-green-500" },
    { code: "b", label: "Aqua", cls: "bg-cyan-400" },
    { code: "c", label: "Red", cls: "bg-red-500" },
    { code: "d", label: "Light purple", cls: "bg-fuchsia-400" },
    { code: "e", label: "Yellow", cls: "bg-yellow-400" },
    { code: "f", label: "White", cls: "bg-white" },
  ];

  const common: { key: string; label: string; hint?: string }[] = [
    { key: "motd", label: "MOTD" },
    { key: "server-port", label: "Server port" },
    { key: "online-mode", label: "Online mode (auth)" },
    { key: "max-players", label: "Max players" },
    { key: "white-list", label: "Whitelist" },
    { key: "view-distance", label: "View distance (chunks)" },
    { key: "simulation-distance", label: "Simulation distance" },
    { key: "difficulty", label: "Difficulty" },
    { key: "gamemode", label: "Default gamemode" },
    { key: "pvp", label: "PvP" },
    { key: "hardcore", label: "Hardcore" },
    { key: "spawn-protection", label: "Spawn protection" },
    { key: "enable-command-block", label: "Enable command blocks" },
    { key: "spawn-monsters", label: "Spawn monsters" },
    { key: "spawn-animals", label: "Spawn animals" },
    { key: "allow-nether", label: "Allow nether" },
    { key: "enforce-secure-profile", label: "Enforce secure profile" },
    { key: "sync-chunk-writes", label: "Sync chunk writes" },
    { key: "prevent-proxy-connections", label: "Prevent proxy connections" },
    { key: "max-tick-time", label: "Max tick time (ms)" },
  ];

  const boolProps = new Set(["online-mode", "white-list", "pvp", "hardcore", "enable-command-block", "spawn-monsters", "spawn-animals", "allow-nether", "enforce-secure-profile", "sync-chunk-writes", "prevent-proxy-connections"]);

  const boolean = (v: string | undefined) => v === "true";

  async function load() {
    try {
      p = await propertiesApi.get(id);
      rebuildText();
      motdText = p["motd"] ?? "";
      icon = await propertiesApi.iconGet(id);
      dirty = false;
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function setMOTD(v: string) {
    setP("motd", v);
  }

  const colors: Record<string, string> = { "0": "#000000", "1": "#0000AA", "2": "#00AA00", "3": "#00AAAA", "4": "#AA0000", "5": "#AA00AA", "6": "#FFAA00", "7": "#AAAAAA", "8": "#555555", "9": "#5555FF", a: "#55FF55", b: "#55FFFF", c: "#FF5555", d: "#FF55FF", e: "#FFFF55", f: "#FFFFFF" };
  const special: Record<string, string> = { k: "animation:mcobfuscate 1s infinite", l: "font-weight:bold", m: "text-decoration:line-through", n: "text-decoration:underline", o: "font-style:italic" };

  function motdPreview(motd: string): string {
    const s = motd.replaceAll("\\u00A7", "\u00A7");
    const segs: { text: string; style: string }[] = [];
    let buf = "";
    let style = "";
    const flush = () => {
      if (buf) {
        segs.push({ text: buf, style });
        buf = "";
      }
    };
    let i = 0;
    while (i < s.length) {
      const c = s[i];
      if ((c === "&" || c === "\u00A7") && i + 1 < s.length) {
        const code = s[i + 1].toLowerCase();
        flush();
        if (code === "r") {
          style = "";
        } else if (code in colors) {
          style = `color:${colors[code]};`;
        } else if (code in special) {
          style = `${special[code]};`;
        }
        i += 2;
        continue;
      }
      buf += c;
      i += 1;
    }
    flush();
    if (segs.length === 0) return "&nbsp;";
    return segs.map((g) => (g.style ? `<span style="${g.style}">${g.text}</span>` : g.text)).join("");
  }

  function insertFormat(code: string) {
    const el = motdTextarea;
    const ins = `&${code}`;
    if (el) {
      const start = el.selectionStart ?? motdText.length;
      const end = el.selectionEnd ?? motdText.length;
      const next = motdText.slice(0, start) + ins + motdText.slice(end);
      motdText = next;
      setP("motd", next);
      requestAnimationFrame(() => {
        el.focus();
        el.selectionStart = el.selectionEnd = start + ins.length;
      });
    } else {
      motdText += ins;
      setP("motd", motdText);
    }
  }

  async function applyIcon(path: string) {
    if (!path) return;
    try {
      icon = await propertiesApi.iconSet(id, path);
      toast(t("Server icon updated (applies after restart)"), "success");
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function changeIcon() {
    try {
      const picked = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "PNG icon", extensions: ["png"] }],
      });
      if (typeof picked === "string") await applyIcon(picked);
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function dropIconPaths(paths: string[]) {
    const png = paths.find((p) => p.toLowerCase().endsWith(".png")) ?? paths[0];
    void applyIcon(png);
  }

  async function setupIconDnd(): Promise<() => void> {
    if (!isTauri()) return () => {};
    try {
      const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over" || event.payload.type === "enter") {
          iconDragging = true;
        } else if (event.payload.type === "drop") {
          iconDragging = false;
          dropIconPaths(event.payload.paths);
        } else if (event.payload.type === "leave") {
          iconDragging = false;
        }
      });
      return unlisten;
    } catch {
      return () => {};
    }
  }

  onMount(() => {
    void load();
    void setupIconDnd().then((fn) => {
      cleanupIconDnd = fn;
    });
    return () => cleanupIconDnd();
  });

  let cleanupIconDnd: () => void = () => {};

  function rebuildText() {
    text = Object.entries(p)
      .map(([k, v]) => `${k}=${v}`)
      .join("\n");
  }

  function setP(key: string, value: string) {
    dirty = true;
    p = { ...p, [key]: value };
    rebuildText();
  }

  function parseText() {
    const map: Record<string, string> = {};
    for (const line of text.split("\n")) {
      const raw = line.trim();
      if (!raw || raw.startsWith("#")) {
        if (raw.startsWith("#")) continue;
        continue;
      }
      const eq = raw.indexOf("=");
      if (eq <= 0) continue;
      const k = raw.slice(0, eq).trim();
      const v = raw.slice(eq + 1).trim();
      if (k) map[k] = v;
    }
    return map;
  }

  async function save() {
    saving = true;
    try {
      p = await propertiesApi.set(id, parseText());
      rebuildText();
      dirty = false;
      toast(t("Saved (some settings need a restart)"), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      saving = false;
    }
  }

  function reset() {
    if (!confirm(t("Reload current values from disk? Unsaved changes are lost."))) return;
    load();
  }

  async function exportServer() {
    exporting = true;
    try {
      const r = await exportApi.exportServer(id);
      toast(t("Exported {name} ({n} files)", { name: r.name, n: r.files }), "success");
    } catch (e) {
      toast(String(e), "error");
    } finally {
      exporting = false;
    }
  }

  onMount(load);
</script>

<Card title={t("Server Icon & MOTD")} class="mb-4">
    <div class="flex flex-col md:flex-row gap-5 items-start">
      <div
        class="shrink-0 flex flex-col items-center gap-2 rounded-xl border-2 border-dashed p-3 cursor-pointer transition-colors {iconDragging ? 'border-brand-500 bg-brand-500/10' : 'border-edge hover:border-brand-500/50'}"
        title={t("Click to pick a PNG, or drag & drop one here")}
        onclick={changeIcon}
        ondragover={(e) => e.preventDefault()}
        ondragenter={(e) => { e.preventDefault(); iconDragging = true; }}
        ondragleave={() => (iconDragging = false)}
        ondrop={(e) => {
          e.preventDefault();
          iconDragging = false;
          const paths = pathsFromDataTransfer(e.dataTransfer as DataTransfer);
          const png = paths.find((p) => p.toLowerCase().endsWith(".png")) ?? paths[0];
          if (png) void applyIcon(png);
        }}
      >
        <div class="relative">
          {#if icon}
            <img src={icon} alt="" class="w-16 h-16 rounded-lg object-cover bg-surface-2 border border-edge" />
          {:else}
            <div class="w-16 h-16 rounded-lg bg-surface-2 border border-edge flex items-center justify-center text-xl text-fg-dim">?</div>
          {/if}
        </div>
        <Button variant="outline" size="sm" onclick={(e) => { e.stopPropagation(); changeIcon(); }}>
          <ImagePlus size={14} /> {icon ? t("Change icon") : t("Set icon")}
        </Button>
        <span class="text-[11px] text-fg-dim text-center leading-tight">
          {@html t("64×64 PNG — drop a file here<br />or click to pick")}
        </span>
      </div>
      <div class="flex-1 w-full min-w-0">
        <div class="flex flex-wrap gap-0.5 mb-1.5">
          {#each formatChars as f (f.code)}
            <button
              class={`w-6 h-6 rounded border border-edge cursor-pointer hover:ring-2 ring-brand-500 ${f.cls}`}
              title={`&${f.code} ${t(f.label)}`}
              onclick={() => insertFormat(f.code)}
            ></button>
          {/each}
          <span class="w-px h-6 bg-edge mx-1 self-center"></span>
          {#each [
            { code: "l", label: "Bold", cls: "font-bold", glyph: "B" },
            { code: "o", label: "Italic", cls: "italic", glyph: "i" },
            { code: "n", label: "Underline", cls: "underline", glyph: "U" },
            { code: "m", label: "Strikethrough", cls: "line-through", glyph: "S" },
            { code: "k", label: "Obfuscated (random symbols)", cls: "text-obfuscate", glyph: "K" },
          ] as f}
            <button
              class={`w-6 h-6 rounded border border-edge bg-surface-2 text-sm text-slate-200 cursor-pointer hover:ring-2 ring-brand-500 ${f.cls}`}
              title={`&${f.code} — ${t(f.label)}`}
              onclick={() => insertFormat(f.code)}
            >{f.glyph}</button>
          {/each}
          <button
            class="w-6 h-6 rounded border border-edge bg-surface-2 text-[10px] text-slate-200 cursor-pointer hover:ring-2 ring-brand-500"
            title={t("&r — Reset formatting")}
            onclick={() => insertFormat("r")}
          >R</button>
        </div>
        <p class="text-[11px] text-fg-dim mb-1.5">
          {@html t("Motd color hint")}
        </p>
        <textarea
          bind:this={motdTextarea}
          bind:value={motdText}
          placeholder={t("&aWelcome to &6your server &r- type with & style codes")}
          class="w-full h-20 font-mono text-sm bg-surface-2 border border-edge rounded-lg p-3 text-slate-100 outline-none focus:border-brand-500 resize-y"
          oninput={(e) => setMOTD(motdText)}
        ></textarea>
        {#if p["motd"]}
          <div class="mt-1.5 flex items-center gap-2 text-sm bg-surface-2 border border-edge rounded-lg px-3 py-2">
            <span class="text-fg-dim text-xs shrink-0">{t("Preview:")}</span>
            <span class="min-w-0 truncate">{@html motdPreview(p["motd"])}</span>
          </div>
        {/if}
      </div>
    </div>
  </Card>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-4">
    <Card title={t("Quick Settings")} subtitle={t("Common server.properties values")} class="flex flex-col" bodyClass="flex-1">
      {#snippet actions()}
        <Button variant="ghost" size="sm" onclick={reset} title={t("Reload")}><RotateCcw size={14} /></Button>
      {/snippet}
      <div class="flex flex-col gap-4">
        {#each common as c (c.key)}
          {#if boolProps.has(c.key)}
            <Switch label={t(c.label)} checked={boolean(p[c.key])} onChange={(v) => setP(c.key, String(v))} />
          {:else}
            <Input
              label={t(c.label)}
              value={p[c.key] ?? ""}
              onChange={(v) => setP(c.key, v)}
            />
          {/if}
        {/each}
      </div>
    </Card>

    <Card title={t("Raw server.properties")} subtitle={t("Full file — one `key=value` per line")} class="flex flex-col" bodyClass="flex-1 flex flex-col">
      {#snippet actions()}
        <Button variant="ghost" size="sm" onclick={reset} title={t("Reload")}><RotateCcw size={14} /></Button>
      {/snippet}
      <textarea
        bind:value={text}
        class="w-full flex-1 min-h-[320px] font-mono text-sm bg-surface-2 border border-edge rounded-lg p-3 text-slate-100 outline-none focus:border-brand-500 resize-y"
      ></textarea>
    </Card>
  </div>

<div class="mt-4 flex items-center gap-3 flex-wrap">
  <Button onclick={save} disabled={saving}>
    <Save size={16} /> {t("Save Changes")}
  </Button>
  <Button variant="outline" onclick={exportServer} disabled={exporting}>
    <Package size={16} /> {exporting ? t("Exporting…") : t("Export as .mrpack")}
  </Button>
  {#if dirty}
    <span class="text-xs text-amber-400">{t("Unsaved changes")}</span>
  {/if}
</div>