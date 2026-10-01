<script lang="ts">
  import { filesApi } from "../../lib/api";
  import { toast } from "../../lib/store.svelte";
  import Card from "../../lib/ui/Card.svelte";
  import Button from "../../lib/ui/Button.svelte";
  import Input from "../../lib/ui/Input.svelte";
  import Select from "../../lib/ui/Select.svelte";
  import Dialog from "../../lib/ui/Dialog.svelte";
  import ConfirmDialog from "../../lib/ui/ConfirmDialog.svelte";
  import type { FileEntry } from "../../lib/types";
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import { isTauri } from "../../lib/env";
  import {
    Folder,
    FileText,
    ArrowUp,
    RefreshCw,
    FilePlus2,
    FolderPlus,
    Pencil,
    Copy,
    Scissors,
    Clipboard,
    Archive,
    FileArchive,
    Trash2,
    ChevronRight,
    Upload,
    List,
    LayoutGrid,
    ArrowDownUp,
  } from "lucide-svelte";
  import { t, pl } from "../../lib/i18n.svelte";

  let { id }: { id: string } = $props();

  let path = $state("");
  let entries: FileEntry[] = $state([]);
  let selected: string[] = $state([]);
  let loading = $state(false);
  let err = $state<string | null>(null);
  let sortKey = $state<"name" | "size" | "modified">("name");
  let sortDesc = $state(false);
  let view = $state<"list" | "grid">("list");
  let dragging = $state(false);
  let uploading = $state(false);

  let editor = $state<{ path: string; name: string; content: string } | null>(null);
  let editorDraft = $state("");
  let createOpen = $state(false);
  let createName = $state("");
  let createIsDir = $state(false);
  let renameTarget = $state<FileEntry | null>(null);
  let renameName = $state("");
  let zipOpen = $state(false);
  let zipName = $state("");
  let confirmDelete = $state(false);
  let clip = $state<{ mode: "copy" | "cut"; paths: string[] } | null>(null);

  let taRef: HTMLTextAreaElement | undefined = $state();
  let preRef: HTMLPreElement | undefined = $state();
  let gutterRef: HTMLDivElement | undefined = $state();

  const join = (p: string, name: string) => (p ? `${p}/${name}` : name);
  const segments = $derived(path ? path.split("/") : []);
  const LINE_H = "1.5em";

  const sortedEntries = $derived(
    [...entries].sort((a, b) => {
      if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
      const dir = sortDesc ? -1 : 1;
      switch (sortKey) {
        case "size":
          return (a.size - b.size) * dir;
        case "modified":
          return (a.modified.localeCompare(b.modified)) * dir;
        default:
          return a.name.toLowerCase().localeCompare(b.name.toLowerCase()) * dir;
      }
    })
  );

  const fmtSize = (e: FileEntry) => {
    if (e.is_dir) return "—";
    if (e.size >= 1024 * 1024 * 1024) return `${(e.size / (1024 * 1024 * 1024)).toFixed(2)} GB`;
    if (e.size >= 1024 * 1024) return `${(e.size / (1024 * 1024)).toFixed(1)} MB`;
    if (e.size >= 1024) return `${Math.round(e.size / 1024)} KB`;
    return `${e.size} B`;
  };

  const fmtModified = (iso?: string) => {
    if (!iso) return "—";
    const d = new Date(iso);
    if (Number.isNaN(d.getTime())) return "—";
    return d.toLocaleString();
  };

  async function refresh() {
    loading = true;
    err = null;
    try {
      entries = await filesApi.list(id, path);
    } catch (e) {
      err = String(e);
    } finally {
      loading = false;
    }
  }

  const relOf = (name: string) => join(path, name);

  function enterDir(name: string) {
    path = join(path, name);
    selected = [];
    refresh();
  }

  function upDir() {
    if (!path) return;
    path = path.split("/").slice(0, -1).join("/");
    selected = [];
    refresh();
  }

  function goSegment(i: number) {
    path = segments.slice(0, i + 1).join("/");
    selected = [];
    refresh();
  }

  function isSel(name: string) {
    return selected.includes(name);
  }

  function toggleSelect(name: string) {
    if (isSel(name)) selected = selected.filter((s) => s !== name);
    else selected = [...selected, name];
  }

  function onRowClick(e: MouseEvent, entry: FileEntry) {
    if (e.ctrlKey || e.metaKey) {
      toggleSelect(entry.name);
      return;
    }
    if (isSel(entry.name)) selected = selected.filter((s) => s !== entry.name);
    else selected = [entry.name];
  }

  function onRowDblClick(entry: FileEntry) {
    if (entry.is_dir) enterDir(entry.name);
    else if (isImageName(entry.name)) openImageViewer(entry);
    else openEditor(entry);
  }

  const IMAGE_EXTS = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "ico"];

  function isImageName(name: string): boolean {
    const ext = name.toLowerCase().split(".").pop() ?? "";
    return IMAGE_EXTS.includes(ext);
  }

  let imageViewer = $state<{ name: string; dataUrl: string } | null>(null);
  let imageLoading = $state(false);

  async function openImageViewer(entry: FileEntry) {
    imageLoading = true;
    try {
      const dataUrl = await filesApi.readDataUrl(id, relOf(entry.name));
      imageViewer = { name: entry.name, dataUrl };
    } catch (e) {
      toast(String(e), "error");
    } finally {
      imageLoading = false;
    }
  }

  async function openEditor(entry: FileEntry) {
    try {
      const content = await filesApi.read(id, relOf(entry.name));
      if (content.includes("\u0000")) {
        toast(t("This looks like a binary file — can't edit as text"), "error");
        return;
      }
      editor = { path: relOf(entry.name), name: entry.name, content };
      editorDraft = content;
      undoStack = [];
      redoStack = [];
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function saveEditor() {
    if (!editor) return;
    try {
      await filesApi.write(id, editor.path, editorDraft);
      toast(t("Saved"), "success");
      editor = null;
      refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function doCreate() {
    const name = createName.trim().replace(/^\/+|\/+$/g, "");
    if (!name) return;
    if (name.includes("/")) {
      toast(t("Name can't contain slashes"), "error");
      return;
    }
    try {
      await filesApi.create(id, relOf(name), createIsDir);
      toast(createIsDir ? t("Folder created") : t("File created"), "success");
      createOpen = false;
      createName = "";
      refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function doRename() {
    if (!renameTarget) return;
    const name = renameName.trim();
    if (!name) return;
    try {
      await filesApi.rename(id, relOf(renameTarget.name), name);
      toast(t("Renamed"), "success");
      renameTarget = null;
      refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function doDelete() {
    const targets = selected.length > 0 ? selected : [];
    if (targets.length === 0) return;
    try {
      const n = await filesApi.remove(id, targets.map((n) => relOf(n)));
      toast(pl(n, ["Удалено: {n} элемент", "Удалено: {n} элемента", "Удалено: {n} элементов"], "Deleted {n} item(s)"), "success");
      selected = [];
      refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function doZip() {
    const targets = selected.length > 0 ? selected : entries.map((e) => e.name);
    if (targets.length === 0) return;
    const name = zipName.trim() || (path.split("/").pop() || "archive");
    try {
      const out = await filesApi.zip(id, targets.map((n) => relOf(n)).filter((r) => r), path, name);
      toast(t("Created {out}", { out: out.split("/").pop() ?? "" }), "success");
      zipOpen = false;
      refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function doUnzip(entry: FileEntry) {
    try {
      await filesApi.unzip(id, relOf(entry.name), path);
      toast(t("Extracted {name} into this folder", { name: entry.name }), "success");
      refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function setClip(mode: "copy" | "cut") {
    if (selected.length === 0) return;
    clip = { mode, paths: selected.map((n) => relOf(n)) };
  }

  async function paste() {
    if (!clip) return;
    try {
      await filesApi.copyMove(id, clip.paths, path, clip.mode === "cut");
      toast(clip.mode === "cut" ? t("Moved") : t("Copied"), "success");
      clip = null;
      selected = [];
      refresh();
    } catch (e) {
      toast(String(e), "error");
    }
  }

  async function uploadFiles(paths: string[]) {
    if (paths.length === 0) return;
    uploading = true;
    try {
      const n = await filesApi.upload(id, path, paths);
      toast(pl(n, ["Загружено: {n} файл", "Загружено: {n} файла", "Загружено: {n} файлов"], "Uploaded {n} file(s)"), "success");
      refresh();
    } catch (e) {
      toast(String(e), "error");
    } finally {
      uploading = false;
    }
  }

  async function pickUpload() {
    try {
      const picked = await open({ multiple: true, directory: false });
      if (Array.isArray(picked) || typeof picked === "string") {
        const paths = (Array.isArray(picked) ? picked : [picked]).filter((p): p is string => typeof p === "string");
        await uploadFiles(paths);
      }
    } catch (e) {
      toast(String(e), "error");
    }
  }

  const zipCandidate = $derived(
    selected.length === 1 ? entries.find((e) => e.name === selected[0] && e.name.toLowerCase().endsWith(".zip")) : null
  );

  // ---- editor highlighting -------------------------------------------------

  function langFor(
    name: string
  ): "props" | "json" | "yaml" | "xml/html" | "code" | "md" | "sql" | "text" {
    const n = name.toLowerCase();
    if (n.endsWith(".properties") || n === "server.properties") return "props";
    if (n.endsWith(".toml") || n.endsWith(".cfg") || n.endsWith(".conf") || n.endsWith(".ini")) return "props";
    if (n.endsWith(".json") || n.endsWith(".json5") || n.endsWith(".mcmeta") || n.endsWith(".lang")) return "json";
    if (n.endsWith(".yml") || n.endsWith(".yaml")) return "yaml";
    if (n.endsWith(".xml") || n.endsWith(".html") || n.endsWith(".htm") || n.endsWith(".xhtml") || n.endsWith(".svg")) return "xml/html";
    if (
      n.endsWith(".java") || n.endsWith(".js") || n.endsWith(".mjs") || n.endsWith(".cjs") ||
      n.endsWith(".ts") || n.endsWith(".tsx") || n.endsWith(".jsx") || n.endsWith(".css") ||
      n.endsWith(".scss") || n.endsWith(".kotlin") || n.endsWith(".kts") || n.endsWith(".groovy") ||
      n.endsWith(".sh") || n.endsWith(".bash") || n.endsWith(".py") || n.endsWith(".pyw") ||
      n.endsWith(".rs") || n.endsWith(".go") || n.endsWith(".c") || n.endsWith(".h") ||
      n.endsWith(".cc") || n.endsWith(".cpp") || n.endsWith(".hpp") || n.endsWith(".cs") ||
      n.endsWith(".rb") || n.endsWith(".pl") || n.endsWith(".php") || n.endsWith(".swift") ||
      n.endsWith(".gradle") || n.endsWith(".properties") || n.endsWith(".mcmeta")
    ) return "code";
    if (n.endsWith(".sql") || n === "sqlite") return "sql";
    if (n.endsWith(".md") || n.endsWith(".markdown") || n.endsWith(".txt") || n.endsWith(".log")) return "md";
    return "text";
  }

  function hl(line: string, lang: string): string {
    const esc = (s: string) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
    const span = (cls: string, t: string) => `<span class="${cls}">${t}</span>`;
    if (lang === "text") return esc(line);
    const src = esc(line);
    if (lang === "props" || lang === "yaml") {
      return src
        .split("\n")
        .map((line) => {
          if (/^\s*#/.test(line)) return span("text-slate-500 italic", line);
          const m = line.match(/^([A-Za-z0-9_.\-~\[\]"]*?)(\s*[:=]\s*)(.*)$/);
          if (!m) return line;
          const [, key, op, val] = m;
          const v = val.replace(/("[^"]*"|'[^']*')/g, (q) => span("text-emerald-300", q));
          return `${span("text-cyan-300", key)}${span("text-fg-dim", op)}${v}`;
        })
        .join("\n");
    }
    if (lang === "json") {
      let out = src.replace(/"([^"]*)"(\s*:)/g, (_, k, col) => `\u0001${k}\u0002${col}`);
      out = out.replace(/"[^"]*"/g, (m) => span("text-emerald-300", m));
      out = out.replace(/\b(true|false|null)\b/g, (m) => span("text-amber-300", m));
      out = out.replace(/[\u0001]([^\u0002]*)[\u0002]/g, (_, k) => span("text-violet-300", `"${k}"`));
      return out;
    }
    if (lang === "xml/html") {
      return src
        .replace(/(&lt;!--.*?--&gt;)/g, (m) => span("text-slate-500 italic", m))
        .replace(/(<\/?)([A-Za-z0-9-]+)/g, (_, lt, t) => `${lt}${span("text-fuchsia-300", t)}`)
        .replace(/([A-Za-z-]+)=("[^"]*"|'[^']*')/g, (_, a, v) => `${span("text-cyan-300", a)}=${span("text-emerald-300", v)}`)
        .replace(/("[^"]*"|'[^']*')/g, (m) => span("text-emerald-300", m));
    }
    if (lang === "code") {
      const chunkRe = /((?:\/\/.*?$)|(?:#.*?$))|("(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'|`[^`]*`)|(\b\d+(?:\.\d+)?[a-zA-Z]*\b)|(\b(?:const|let|var|function|return|if|else|elif|for|while|do|class|new|this|super|public|private|protected|static|final|void|int|boolean|long|double|float|short|byte|char|String|List|Map|Set|Object|Boolean|Integer|import|export|from|extends|implements|throw|try|catch|finally|break|continue|def|None|True|False|and|or|not|in|is|lambda|async|await|with|as|fn|let|mut|impl|trait|struct|enum|pub|use|mod|self|Self|move|package|type|interface|select|defer|go|range|map|chan|then|fi|case|esac|done|echo|printf|declare|global|match)\b)|([A-Za-z_$][\w$]*)(?=\s*\()/gm;
      const parts: string[] = [];
      let last = 0;
      let m: RegExpExecArray | null;
      chunkRe.lastIndex = 0;
      while ((m = chunkRe.exec(src)) !== null) {
        parts.push(src.slice(last, m.index));
        const com = m[1], str = m[2], num = m[3], kw = m[4], fn = m[5];
        if (com) parts.push(span("text-slate-500 italic", com));
        else if (str) parts.push(span("text-emerald-300", str));
        else if (num) parts.push(span("text-amber-300", num));
        else if (kw) parts.push(span("text-fuchsia-300", kw));
        else if (fn) parts.push(span("text-blue-300", fn));
        last = chunkRe.lastIndex;
      }
      parts.push(src.slice(last));
      return parts.join("");
    }
    if (lang === "sql") {
      let out = src.replace(/^(\s*--.*)$/gm, (_, c) => span("text-slate-500 italic", c));
      out = out.replace(/\b(SELECT|FROM|WHERE|INSERT|UPDATE|DELETE|INTO|VALUES|SET|JOIN|ON|AS|GROUP|BY|ORDER|LIMIT|CREATE|TABLE|INDEX|DROP|ALTER|AND|OR|NOT|NULL|PRIMARY|KEY|FOREIGN|REFERENCES|UNION|CASE|WHEN|THEN|END|COUNT|SUM|AVG|MIN|MAX|IN|EXISTS|DISTINCT|LEFT|RIGHT|INNER|OUTER)\b/gi, (m) => span("text-fuchsia-300", m));
      out = out.replace(/("[^"]*"|'[^']*')/g, (m) => span("text-emerald-300", m));
      return out;
    }
    if (lang === "md") {
      let out = src.replace(/^(#+)\s+(.*)$/gm, (_, h, t) => span("text-sky-300 font-bold", `${h} ${t}`));
      out = out.replace(/`([^`]+)`/g, (_, c) => span("text-emerald-300", `\u0001${c}\u0002`));
      out = out.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_, t, u) => `${span("text-blue-300", t)}(${span("text-fg-dim", u)})`);
      out = out.replace(/\*\*([^*]+)\*\*/g, (_, t) => span("text-amber-300 font-bold", t));
      out = out.replace(/\*([^*]+)\*/g, (_, t) => span("text-amber-300 italic", t));
      out = out.replace(/\u0001([^\u0002]*)\u0002/g, (_, t) => span("text-emerald-300", t));
      return out;
    }
    return src;
  }

  const highlighted = $derived(editor ? hl(editorDraft, langFor(editor.name)) : "");
  const editorLines = $derived(editor ? Math.max(editorDraft.split("\n").length, editor.content.split("\n").length) : 0);
  const changedLines = $derived(editor ? (() => {
    const a = editor.content.split("\n");
    const b = editorDraft.split("\n");
    const set = new Set<number>();
    const max = Math.max(a.length, b.length);
    for (let i = 0; i < max; i++) if (a[i] !== b[i]) set.add(i);
    return set;
  })() : new Set<number>());
  const lineNums = $derived(Array.from({ length: editorLines }, (_, i) => i + 1));

  function syncScroll() {
    if (!taRef) return;
    if (preRef) {
      preRef.scrollTop = taRef.scrollTop;
      preRef.scrollLeft = taRef.scrollLeft;
    }
    if (gutterRef) gutterRef.scrollTop = taRef.scrollTop;
  }

  function onKeyEditor(e: KeyboardEvent) {
    if (e.key === "Escape") editor = null;
    if (e.ctrlKey || e.metaKey) {
      const code = e.code;
      if (code === "KeyS") {
        e.preventDefault();
        saveEditor();
      } else if (code === "KeyZ") {
        e.preventDefault();
        if (e.shiftKey) redoEditor();
        else undoEditor();
      } else if (code === "KeyY") {
        e.preventDefault();
        redoEditor();
      }
    }
    e.stopPropagation();
  }

  // ---- editor undo/redo (WebKit textareas don't reliably keep undo history
  // when the value is re-bound on every keystroke) ----------------------------

  let undoStack: string[] = $state([]);
  let redoStack: string[] = $state([]);

  function captureUndo() {
    if (!editor) return;
    if (undoStack[undoStack.length - 1] === editorDraft) return;
    undoStack.push(editorDraft);
    if (undoStack.length > 200) undoStack.shift();
    redoStack = [];
  }

  function undoEditor() {
    if (!editor || undoStack.length === 0) return;
    redoStack.push(editorDraft);
    editorDraft = undoStack.pop()!;
  }

  function redoEditor() {
    if (!editor || redoStack.length === 0) return;
    undoStack.push(editorDraft);
    editorDraft = redoStack.pop()!;
  }

  // ---- drag & drop ---------------------------------------------------------

  async function setupDnd(): Promise<() => void> {
    if (!isTauri()) return () => {};
    try {
      const unlisten = await getCurrentWebview().onDragDropEvent((event) => {
        if (event.payload.type === "over" || event.payload.type === "enter") {
          dragging = true;
        } else if (event.payload.type === "drop") {
          dragging = false;
          uploadFiles(event.payload.paths);
        } else if (event.payload.type === "leave") {
          dragging = false;
        }
      });
      return unlisten;
    } catch {
      return () => {};
    }
  }

  let cleanupDnd: () => void = () => {};

  onMount(() => {
    refresh();
    void setupDnd().then((fn) => {
      cleanupDnd = fn;
    });
    return () => {
      cleanupDnd();
    };
  });
</script>

<Card title={t("Files")} subtitle={t("Full file access inside this server's folder")} class="h-full flex flex-col" bodyClass="flex-1 flex flex-col min-h-0">
  {#snippet actions()}
    <span class="text-xs text-fg-dim mr-1">
      {#if selected.length > 0}
        {pl(selected.length, ["{n} выбран", "{n} выбрано", "{n} выбрано"], "{n} selected")}
      {:else}
        {pl(entries.length, ["{n} элемент", "{n} элемента", "{n} элементов"], "{n} item(s)")}
      {/if}
    </span>
    <Button variant="outline" size="sm" onclick={refresh} title={t("Refresh")} disabled={loading}>
      <RefreshCw size={14} class={loading ? "animate-spin" : ""} />
    </Button>
  {/snippet}

  <div class="flex items-center gap-1.5 mb-3 flex-wrap">
    <Button variant="outline" size="xs" onclick={() => goSegment(-1)} disabled={path === ""} title={t("Root")}>
      <ArrowUp size={13} /> {t("Root")}
    </Button>
    <ol class="flex items-center gap-0.5 flex-wrap text-xs text-fg-dim">
      {#each segments as seg, i}
        <li class="flex items-center gap-0.5">
          <ChevronRight size={12} />
          <button class="hover:text-white cursor-pointer" onclick={() => goSegment(i)}>{seg}</button>
        </li>
      {/each}
    </ol>
    <div class="flex-1"></div>
    <div class="flex items-center gap-1.5">
      <ArrowDownUp size={13} class="text-fg-dim" />
      <Select
        value={sortKey}
        size="xs"
        options={[
          { value: "name", label: t("Name") },
          { value: "size", label: t("Size") },
          { value: "modified", label: t("Modified") },
        ]}
        onChange={(v) => (sortKey = v as "name" | "size" | "modified")}
      />
      <button class="cursor-pointer hover:text-white" onclick={() => (sortDesc = !sortDesc)} title={t("Toggle order")}>
        {sortDesc ? "↓" : "↑"}
      </button>
    </div>
    <span class="flex items-center gap-0.5 rounded-md border border-edge overflow-hidden">
      <button
        class="px-2 py-1 cursor-pointer {view === 'list' ? 'bg-brand-500/20 text-brand-300' : 'text-fg-dim hover:text-white'}"
        onclick={() => (view = "list")}
        title={t("List view")}
      >
        <List size={13} />
      </button>
      <button
        class="px-2 py-1 cursor-pointer {view === 'grid' ? 'bg-brand-500/20 text-brand-300' : 'text-fg-dim hover:text-white'}"
        onclick={() => (view = "grid")}
        title={t("Grid view")}
      >
        <LayoutGrid size={13} />
      </button>
    </span>
  </div>

  <div class="flex items-center gap-1 flex-wrap mb-3">
    <Button variant="subtle" size="xs" onclick={() => { createIsDir = false; createName = ""; createOpen = true; }} title={t("Create new file")}>
      <FilePlus2 size={13} /> {t("New file")}
    </Button>
    <Button variant="subtle" size="xs" onclick={() => { createIsDir = true; createName = ""; createOpen = true; }} title={t("Create new folder")}>
      <FolderPlus size={13} /> {t("New folder")}
    </Button>
    <Button variant="subtle" size="xs" onclick={pickUpload} disabled={uploading} title={t("Upload files into this folder")}>
      <Upload size={13} /> {t("Upload")}
    </Button>
    <span class="w-px h-5 bg-edge mx-1"></span>
    <Button variant="subtle" size="xs" disabled={selected.length === 0} onclick={setClip.bind(null, "copy")} title={t("Copy selection")}>
      <Copy size={13} /> {t("Copy")}
    </Button>
    <Button variant="subtle" size="xs" disabled={selected.length === 0} onclick={setClip.bind(null, "cut")} title={t("Cut selection")}>
      <Scissors size={13} /> {t("Cut")}
    </Button>
    <Button variant="subtle" size="xs" disabled={!clip} onclick={paste} title={t("Paste into current folder")}>
      <Clipboard size={13} /> {t("Paste")}
    </Button>
    <span class="w-px h-5 bg-edge mx-1"></span>
    <Button variant="subtle" size="xs" disabled={selected.length === 0} onclick={() => { renameTarget = entries.find((e) => e.name === selected[0]) ?? null; renameName = renameTarget?.name ?? ""; }} title={t("Rename first selected")}>
      <Pencil size={13} /> {t("Rename")}
    </Button>
    <Button variant="subtle" size="xs" disabled={selected.length === 0} onclick={() => { zipOpen = true; zipName = path.split("/").pop() || "archive"; }} title={t("Archive selection into a .zip")}>
      <Archive size={13} /> {t("Zip…")}
    </Button>
    <Button variant="subtle" size="xs" disabled={!zipCandidate} onclick={() => zipCandidate && doUnzip(zipCandidate)} title={t("Extract the selected .zip here")}>
      <FileArchive size={13} /> {t("Unzip")}
    </Button>
    <span class="w-px h-5 bg-edge mx-1"></span>
    <Button variant="danger" size="xs" disabled={selected.length === 0} onclick={() => (confirmDelete = true)} title={t("Delete selection")}>
      <Trash2 size={13} /> {t("Delete")}
    </Button>
  </div>

  {#if err}
    <div class="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-sm text-red-300 mb-3">{err}</div>
  {/if}

  {#if path}
    <button
      class="flex items-center gap-2 w-full px-3 py-2 rounded-lg border border-edge bg-surface-2/40 text-sm text-fg-dim hover:text-white cursor-pointer mb-1"
      onclick={upDir}
    >
      <ArrowUp size={14} /> <span>{t(".. (up to {dest})", { dest: path.split("/").slice(0, -1).pop() || t("root") })}</span>
    </button>
  {/if}

  {#if view === "list"}
    <ul class="scroll-gutter flex flex-col gap-0.5 flex-1 min-h-0 overflow-y-auto">
      {#each sortedEntries as entry (relOf(entry.name))}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions a11y_no_static_element_interactions a11y_click_events_have_key_events -->
        <li
          class="flex items-center gap-2 px-3 py-1.5 rounded-lg border cursor-pointer select-none transition-colors {isSel(entry.name) ? 'border-brand-500/60 bg-brand-500/10' : 'border-edge bg-surface-2/40 hover:bg-surface-2'}"
          onclick={(e) => onRowClick(e, entry)}
          ondblclick={() => onRowDblClick(entry)}
        >
          <button
            class="shrink-0 w-6 h-6 rounded-md flex items-center justify-center text-brand-500 hover:bg-surface-3/60 cursor-pointer"
            onclick={(e) => { e.stopPropagation(); if (entry.is_dir) enterDir(entry.name); else if (isImageName(entry.name)) void openImageViewer(entry); else void openEditor(entry); }}
            title={entry.is_dir ? t("Open folder") : isImageName(entry.name) ? t("Preview image") : t("Open as text")}
          >
            {#if entry.is_dir}
              <Folder size={15} />
            {:else}
              <FileText size={15} class="text-fg-dim" />
            {/if}
          </button>
          <span class="flex-1 min-w-0 truncate text-sm {entry.is_dir ? 'text-slate-100 font-medium' : 'text-slate-200'}">{entry.name}</span>
          <span class="shrink-0 text-xs text-fg-dim tabular-nums">{fmtSize(entry)}</span>
          <span class="shrink-0 text-xs text-fg-dim tabular-nums w-40 text-right hidden md:block">{fmtModified(entry.modified)}</span>
        </li>
      {/each}
      {#if entries.length === 0}
        <li class="text-sm text-fg-dim px-3 py-6 text-center">{t("Folder is empty.")}</li>
      {/if}
    </ul>
  {:else}
    <div class="scroll-gutter grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-2.5 flex-1 min-h-0 overflow-y-auto">
      {#each sortedEntries as entry (relOf(entry.name))}
        <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
        <div
          class="flex flex-col items-center gap-1.5 rounded-lg border p-3 cursor-pointer select-none transition-colors text-center {isSel(entry.name) ? 'border-brand-500/60 bg-brand-500/10' : 'border-edge bg-surface-2/40 hover:bg-surface-2'}"
          onclick={(e) => onRowClick(e, entry)}
          ondblclick={() => onRowDblClick(entry)}
        >
          {#if entry.is_dir}
            <Folder size={30} class="text-brand-500 mt-1" />
          {:else}
            <FileText size={30} class="text-fg-dim mt-1" />
          {/if}
          <span class="w-full truncate text-xs {entry.is_dir ? 'text-slate-100 font-medium' : 'text-slate-200'}">{entry.name}</span>
          <span class="text-[10px] text-fg-dim tabular-nums">{fmtSize(entry)}</span>
        </div>
      {/each}
      {#if entries.length === 0}
        <div class="col-span-full text-sm text-fg-dim px-3 py-6 text-center">{t("Folder is empty.")}</div>
      {/if}
    </div>
  {/if}

  {#if dragging}
    <div class="mt-3 rounded-lg border-2 border-dashed border-brand-500/60 bg-brand-500/10 px-4 py-8 text-center text-sm text-brand-300">
      {t("Drop files to upload into {path}", { path: path || t("server root") })}
    </div>
  {/if}

  {#if clip}
    <div class="flex items-center gap-3 mt-3 px-3 py-2 rounded-lg border border-brand-500/40 bg-brand-500/10 text-sm">
      <Clipboard size={14} class="text-brand-400" />
      <span class="text-slate-100">{t("clipboard items", { n: clip.paths.length, mode: clip.mode === "copy" ? t("Copy") : t("Move") })}</span>
      <div class="flex-1"></div>
      <Button size="xs" onclick={paste}>{clip.mode === "copy" ? t("Paste") : t("Move here")}</Button>
      <Button variant="ghost" size="xs" onclick={() => (clip = null)}>{t("Cancel")}</Button>
    </div>
  {/if}
</Card>

{#if editor}
  <Dialog title={t("Edit: {name}", { name: editor.name })} open={true} onClose={() => (editor = null)} wide>
    <div class="flex flex-col gap-2">
      <div class="flex border border-edge rounded-lg overflow-hidden bg-surface-0" style="min-height: 320px;">
        <div
          bind:this={gutterRef}
          class="shrink-0 w-12 text-right px-2 py-3 overflow-hidden select-none text-[12px] text-fg-dim font-mono text-slate-500"
          style="line-height: {LINE_H};"
        >
          {#each lineNums as ln (ln)}
            <div class="flex items-center gap-1 justify-end">
              <span class="shrink-0 text-slate-600">{ln}</span>
              <span class="w-px h-full shrink-0 {changedLines.has(ln - 1) ? 'bg-amber-400' : 'bg-transparent'}"></span>
            </div>
          {/each}
        </div>
        <div class="relative flex-1 min-w-0">
          <pre
            bind:this={preRef}
            class="m-0 px-3 py-3 font-mono text-[13px] text-slate-100 overflow-hidden pointer-events-none whitespace-pre"
            style="line-height: {LINE_H};"
          >{@html highlighted}</pre>
          <textarea
            bind:this={taRef}
            bind:value={editorDraft}
            onbeforeinput={captureUndo}
            oninput={syncScroll}
            onscroll={syncScroll}
            onkeydown={onKeyEditor}
            spellcheck="false"
            autocomplete="off"
            class="absolute inset-0 w-full h-full resize-none bg-transparent text-transparent caret-slate-100 outline-none font-mono text-[13px] whitespace-pre"
            style="line-height: {LINE_H}; padding: 0.75rem 0.75rem;"
          ></textarea>
        </div>
      </div>
      <div class="flex items-center justify-end gap-2">
        <span class="flex-1 text-xs text-fg-dim">
          {t("editor status", { path: editor.path, n: editorDraft.split("\n").length, m: changedLines.size })}
        </span>
        <span class="text-[11px] text-fg-dim hidden md:block">{t("Ctrl+S to save · Esc to close")}</span>
        <Button variant="outline" size="sm" onclick={() => (editor = null)}>{t("Cancel")}</Button>
        <Button size="sm" onclick={saveEditor}>{t("Save")}</Button>
      </div>
    </div>
  </Dialog>
{/if}

{#if imageViewer}
  <Dialog title={imageViewer.name} open={true} onClose={() => (imageViewer = null)} wide>
    <div class="flex flex-col gap-2">
      <div class="flex items-center justify-center rounded-lg border border-edge bg-surface-0 p-4 max-h-[70vh] overflow-auto">
        {#if imageViewer.dataUrl.startsWith("data:image/")}
          <img src={imageViewer.dataUrl} alt="" class="max-w-full max-h-[65vh] object-contain rounded" />
        {:else}
          <p class="text-sm text-fg-dim">{t("This file is not a supported image.")}</p>
        {/if}
      </div>
      <div class="flex items-center justify-between gap-2">
        <span class="text-xs text-fg-dim truncate">{imageViewer.dataUrl.length > 100 ? `${Math.round(imageViewer.dataUrl.length * 0.75)} bytes` : ""}</span>
        <Button variant="outline" size="sm" onclick={() => (imageViewer = null)}>{t("Close")}</Button>
      </div>
    </div>
  </Dialog>
{/if}

<Dialog title={createIsDir ? t("New folder") : t("New file")} open={createOpen} onClose={() => (createOpen = false)}>
  <div class="flex flex-col gap-3">
    <Input label={t("Name")} bind:value={createName} placeholder={createIsDir ? "mods" : "server.properties"} onkeydown={(e) => { if (e.key === "Enter") doCreate(); }} />
    <div class="flex justify-end gap-2">
      <Button variant="outline" size="sm" onclick={() => (createOpen = false)}>{t("Cancel")}</Button>
      <Button size="sm" onclick={doCreate}>{t("Create")}</Button>
    </div>
  </div>
</Dialog>

{#if renameTarget}
  <Dialog title={t("Rename: {name}", { name: renameTarget.name })} open={true} onClose={() => (renameTarget = null)}>
    <div class="flex flex-col gap-3">
      <Input label={t("New name")} bind:value={renameName} onkeydown={(e) => { if (e.key === "Enter") doRename(); }} />
      <div class="flex justify-end gap-2">
        <Button variant="outline" size="sm" onclick={() => (renameTarget = null)}>{t("Cancel")}</Button>
        <Button size="sm" onclick={doRename}>{t("Rename")}</Button>
      </div>
    </div>
  </Dialog>
{/if}

<Dialog title={t("Create .zip archive")} open={zipOpen} onClose={() => (zipOpen = false)}>
  <div class="flex flex-col gap-3">
    <Input label={t("Archive name")} bind:value={zipName} placeholder="archive" onkeydown={(e) => { if (e.key === "Enter") doZip(); }} />
    <p class="text-xs text-fg-dim">
      {selected.length > 0 ? t("Archiving {n} items", { n: selected.length }) : t("Archiving the whole folder")}.
    </p>
    <div class="flex justify-end gap-2">
      <Button variant="outline" size="sm" onclick={() => (zipOpen = false)}>{t("Cancel")}</Button>
      <Button size="sm" onclick={doZip}>{t("Zip")}</Button>
    </div>
  </div>
</Dialog>

<ConfirmDialog
  title={t("Delete selected files?")}
  message={t("Delete message", { n: selected.length, path: path || t("root") })}
  confirmLabel={t("Delete")}
  open={confirmDelete}
  onConfirm={doDelete}
  onClose={() => (confirmDelete = false)}
/>