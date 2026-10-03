<script lang="ts">
  import { onMount } from "svelte";
  import { t } from "../i18n.svelte";

  import { EditorState, StateField, type Extension } from "@codemirror/state";
  import {
    EditorView,
    gutter,
    GutterMarker,
    keymap,
    lineNumbers,
    drawSelection,
    highlightActiveLine,
  } from "@codemirror/view";
  import { defaultKeymap, history, historyKeymap, indentWithTab, undo, redo } from "@codemirror/commands";
  import { syntaxHighlighting, HighlightStyle, StreamLanguage, bracketMatching } from "@codemirror/language";
  import { tags as tg } from "@lezer/highlight";

  import { javascript } from "@codemirror/lang-javascript";
  import { json } from "@codemirror/lang-json";
  import { yaml } from "@codemirror/lang-yaml";
  import { html } from "@codemirror/lang-html";
  import { css } from "@codemirror/lang-css";
  import { markdown } from "@codemirror/lang-markdown";
  import { java } from "@codemirror/lang-java";
  import { python } from "@codemirror/lang-python";
  import { sql } from "@codemirror/lang-sql";
  import { properties } from "@codemirror/legacy-modes/mode/properties";
  import { toml } from "@codemirror/legacy-modes/mode/toml";
  import { lua } from "@codemirror/legacy-modes/mode/lua";
  import { shell } from "@codemirror/legacy-modes/mode/shell";

  interface Props {
    path: string;
    name: string;
    /** File content as loaded from disk (the "saved" state for change marks). */
    content: string;
    onSave: (content: string) => void;
    onClose: () => void;
  }

  let { path, name, content, onSave, onClose }: Props = $props();

  let host = $state<HTMLDivElement | undefined>();
  let view: EditorView | null = null;

  let docText = $state(content);
  let changedCount = $state(0);
  let dirty = $state(false);

  // ---- Kate-style changed-line tracking ------------------------------------
  //
  // A StateField holds the set of line numbers whose text differs from the
  // content the editor was opened with. On every transaction the existing
  // marks are mapped through the change (so lines keep their status when text
  // is inserted or removed above them — like Kate's line tracker), and lines
  // inside the changed range are re-compared against the original text.
  let originalLines: string[] = [];

  const changedLinesField = StateField.define<Set<number>>({
    create: () => new Set<number>(),
    update(marks, tr) {
      if (!tr.docChanged) return marks;
      const mapped = new Set<number>();
      for (const ln of marks) {
        const pos = tr.startState.doc.line(ln).from;
        mapped.add(tr.state.doc.lineAt(tr.changes.mapPos(pos)).number);
      }
      tr.changes.iterChangedRanges((_fA, _tA, fromB, toB) => {
        const first = tr.state.doc.lineAt(fromB).number;
        const last = tr.state.doc.lineAt(toB).number;
        for (let ln = first; ln <= last; ln++) {
          if (originalLines[ln - 1] !== tr.state.doc.line(ln).text) mapped.add(ln);
          else mapped.delete(ln);
        }
      });
      return mapped;
    },
  });

  const changeBar: GutterMarker = new (class extends GutterMarker {
    toDOM() {
      const bar = document.createElement("div");
      bar.className = "cm-change-bar";
      return bar;
    }
  })();

  const changeGutter = gutter({
    class: "cm-change-gutter",
    lineMarker(view, line) {
      const ln = view.state.doc.lineAt(line.from).number;
      return view.state.field(changedLinesField).has(ln) ? changeBar : null;
    },
    initialSpacer: () => changeBar,
  });

  // ---- language --------------------------------------------------------------
  function languageFor(n: string): Extension | null {
    const f = n.toLowerCase();
    if (f.endsWith(".properties") || f.endsWith(".ini") || f.endsWith(".cfg") || f.endsWith(".conf"))
      return StreamLanguage.define(properties);
    if (f.endsWith(".toml")) return StreamLanguage.define(toml);
    if (f.endsWith(".json") || f.endsWith(".json5") || f.endsWith(".mcmeta") || f.endsWith(".lang"))
      return json();
    if (f.endsWith(".yml") || f.endsWith(".yaml")) return yaml();
    if (f.endsWith(".xml") || f.endsWith(".html") || f.endsWith(".htm") || f.endsWith(".xhtml") || f.endsWith(".svg"))
      return html();
    if (f.endsWith(".css") || f.endsWith(".scss")) return css();
    if (f.endsWith(".md") || f.endsWith(".markdown") || f.endsWith(".txt") || f.endsWith(".log"))
      return markdown();
    if (f.endsWith(".java")) return java();
    if (f.endsWith(".js") || f.endsWith(".mjs") || f.endsWith(".cjs") || f.endsWith(".jsx"))
      return javascript();
    if (f.endsWith(".ts") || f.endsWith(".tsx")) return javascript({ typescript: true });
    if (f.endsWith(".py") || f.endsWith(".pyw")) return python();
    if (f.endsWith(".sql")) return sql();
    if (f.endsWith(".lua")) return StreamLanguage.define(lua);
    if (f.endsWith(".sh") || f.endsWith(".bash")) return StreamLanguage.define(shell);
    return null;
  }

  // ---- theme (matches the app's dark palette) --------------------------------
  const mcsmTheme = EditorView.theme(
    {
      "&": {
        color: "#e6edf7",
        backgroundColor: "#0b1220",
        height: "100%",
        fontSize: "13px",
      },
      ".cm-scroller": {
        fontFamily: "ui-monospace, SFMono-Regular, Menlo, Consolas, monospace",
        lineHeight: "1.55",
      },
      ".cm-content": { caretColor: "#e6edf7" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "#e6edf7" },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, .cm-content ::selection": {
        backgroundColor: "#264f78",
      },
      ".cm-gutters": {
        backgroundColor: "#0b1220",
        color: "#64748b",
        border: "none",
        borderRight: "1px solid #2a3a58",
      },
      ".cm-activeLine": { backgroundColor: "rgba(56, 189, 248, 0.07)" },
      ".cm-activeLineGutter": { backgroundColor: "rgba(56, 189, 248, 0.10)" },
      ".cm-panels": { backgroundColor: "#111a2e", color: "#e6edf7" },
      ".cm-searchMatch": { backgroundColor: "rgba(245, 158, 11, 0.35)" },
      ".cm-searchMatch.cm-searchMatch-selected": { backgroundColor: "rgba(245, 158, 11, 0.6)" },
      ".cm-change-gutter": { width: "8px" },
      ".cm-change-gutter .cm-gutterElement": {
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
      },
      ".cm-change-bar": {
        width: "3px",
        height: "1.2em",
        borderRadius: "2px",
        backgroundColor: "#f59e0b",
      },
      ".cm-tooltip": {
        backgroundColor: "#182338",
        border: "1px solid #2a3a58",
        color: "#e6edf7",
      },
    },
    { dark: true }
  );

  const mcsmHighlight = HighlightStyle.define([
    { tag: tg.comment, color: "#64748b", fontStyle: "italic" },
    { tag: [tg.keyword, tg.moduleKeyword, tg.controlKeyword, tg.operatorKeyword], color: "#f0abfc" },
    { tag: [tg.string, tg.special(tg.string)], color: "#6ee7b7" },
    { tag: [tg.number, tg.bool, tg.null, tg.atom], color: "#fcd34d" },
    { tag: [tg.function(tg.variableName), tg.function(tg.propertyName)], color: "#93c5fd" },
    { tag: [tg.typeName, tg.className], color: "#7dd3fc" },
    { tag: tg.propertyName, color: "#c4b5fd" },
    { tag: tg.definition(tg.variableName), color: "#e6edf7" },
    { tag: tg.variableName, color: "#e6edf7" },
    { tag: [tg.operator, tg.punctuation, tg.bracket], color: "#8fa3bf" },
    { tag: tg.heading, color: "#7dd3fc", fontWeight: "bold" },
    { tag: [tg.link, tg.url], color: "#93c5fd" },
    { tag: tg.emphasis, fontStyle: "italic" },
    { tag: tg.strong, fontWeight: "bold" },
    { tag: [tg.regexp, tg.escape], color: "#fbbf24" },
    { tag: [tg.meta, tg.processingInstruction], color: "#8fa3bf" },
  ]);

  onMount(() => {
    originalLines = content.split("\n");

    const ext: Extension[] = [
      lineNumbers(),
      changeGutter,
      changedLinesField,
      history(),
      drawSelection(),
      highlightActiveLine(),
      bracketMatching(),
      keymap.of([
        { key: "Mod-s", preventDefault: true, run: () => (saveRef(), true) },
        { key: "Escape", preventDefault: true, run: () => (closeRef(), true) },
        indentWithTab,
        ...defaultKeymap,
        ...historyKeymap,
      ]),
      languageFor(name) ?? [],
      syntaxHighlighting(mcsmHighlight),
      mcsmTheme,
      EditorView.updateListener.of((u) => {
        if (u.docChanged) {
          docText = u.state.doc.toString();
          dirty = docText !== content;
          changedCount = u.state.field(changedLinesField).size;
        }
      }),
      EditorView.lineWrapping,
    ];

    const state = EditorState.create({ doc: content, extensions: ext });
    view = new EditorView({ state, parent: host! });
    view.focus();

    return () => {
      view?.destroy();
      view = null;
    };
  });

  // Latest callbacks for the CM keymap (captured once at mount).
  let saveRef = () => onSave(docText);
  let closeRef = onClose;

  function doUndo() {
    if (view) undo(view);
  }
  function doRedo() {
    if (view) redo(view);
  }
  function doSave() {
    onSave(docText);
  }
</script>

<div class="flex flex-col gap-2 min-h-0">
  <div class="flex items-center gap-2 flex-wrap">
    <span class="text-xs text-fg-dim truncate flex-1 min-w-0" title={path}>{path}</span>
    {#if dirty}
      <span class="text-[11px] text-amber-300 shrink-0" title={t("Lines changed since open")}>
        {changedCount > 0 ? `${changedCount}` : "0"} +/-
      </span>
    {:else}
      <span class="text-[11px] text-fg-dim shrink-0">{t("unmodified")}</span>
    {/if}
    <span class="w-px h-5 bg-edge mx-0.5 shrink-0"></span>
    <button
      class="p-1.5 rounded-md text-fg-dim hover:text-white hover:bg-surface-2 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed shrink-0"
      onclick={doUndo}
      title={t("Undo (Ctrl+Z)")}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M3 7v6h6"/><path d="M21 17a9 9 0 0 0-9-9 9 9 0 0 0-6 2.3L3 13"/></svg>
    </button>
    <button
      class="p-1.5 rounded-md text-fg-dim hover:text-white hover:bg-surface-2 cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed shrink-0"
      onclick={doRedo}
      title={t("Redo (Ctrl+Shift+Z)")}
    >
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M21 7v6h-6"/><path d="M3 17a9 9 0 0 1 9-9 9 9 0 0 1 6 2.3l3 2.7"/></svg>
    </button>
    <span class="w-px h-5 bg-edge mx-0.5 shrink-0"></span>
    <button
      class="px-2.5 py-1 rounded-md border border-edge text-xs text-fg-dim hover:text-white hover:bg-surface-2 cursor-pointer shrink-0"
      onclick={onClose}
      title={t("Close (Esc)")}
    >
      {t("Close")}
    </button>
    <button
      class="px-3 py-1 rounded-md text-xs font-medium text-white bg-brand-600 hover:bg-brand-500 cursor-pointer shrink-0"
      onclick={doSave}
      title={t("Save (Ctrl+S)")}
    >
      {t("Save")}
    </button>
  </div>
  <div bind:this={host} class="rounded-lg border border-edge overflow-hidden" style="height: min(62vh, 680px);"></div>
</div>
