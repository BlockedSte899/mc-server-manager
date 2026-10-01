<script lang="ts">
  import "@xterm/xterm/css/xterm.css";
  import { Terminal } from "@xterm/xterm";
  import { FitAddon } from "@xterm/addon-fit";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { consoleApi } from "../lib/api";
  import { cleanConsoleLine, highlightLogAnsi } from "../lib/consoleLines";
  import Button from "../lib/ui/Button.svelte";
  import { toast } from "../lib/store.svelte";
  import { suggestCommands, type ConsoleCommand } from "../lib/consoleCommands";
  import { Eraser, Send } from "lucide-svelte";
  import { t } from "../lib/i18n.svelte";

  interface Props {
    serverId: string;
    height?: string;
  }

  let { serverId, height = "480px" }: Props = $props();

  let el: HTMLDivElement;
  let term: Terminal | undefined;
  let fit: FitAddon | undefined;
  let booted = $state(false);
  let cmd = $state("");
  let history: string[] = [];
  let historyIdx = $state(-1);
  // Lightweight autocomplete: universal commands only, never plugin/mod ones.
  let suggestions = $state<ConsoleCommand[]>([]);
  let suggestionIdx = $state(0);
  let showHints = $state(false);

  $effect(() => {
    suggestions = suggestCommands(cmd);
    if (suggestionIdx >= suggestions.length) suggestionIdx = 0;
  });

  function acceptSuggestion(c: ConsoleCommand) {
    const rest = cmd.trimStart().slice(c.name.length).trimStart();
    cmd = `${c.name}${rest ? " " + rest : " "}`;
    showHints = false;
  }

  async function sendCommand() {
    const line = cmd.trim();
    if (!line) return;
    try {
      await consoleApi.send(serverId, line);
      history.unshift(line);
      history = history.slice(0, 50);
      historyIdx = -1;
      cmd = "";
      showHints = false;
    } catch (e) {
      toast(String(e), "error");
    }
  }

  function onCmdKey(e: KeyboardEvent) {
    if (showHints && suggestions.length > 0) {
      if (e.key === "Tab") {
        e.preventDefault();
        acceptSuggestion(suggestions[suggestionIdx]);
        return;
      }
      if (e.key === "ArrowDown" && e.ctrlKey) {
        e.preventDefault();
        suggestionIdx = (suggestionIdx + 1) % suggestions.length;
        return;
      }
      if (e.key === "ArrowUp" && e.ctrlKey) {
        e.preventDefault();
        suggestionIdx = (suggestionIdx - 1 + suggestions.length) % suggestions.length;
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        showHints = false;
        return;
      }
    }
    if (e.key === "Enter") {
      e.preventDefault();
      void sendCommand();
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      if (historyIdx < 0) historyIdx = 0;
      else if (historyIdx < history.length - 1) historyIdx++;
      cmd = history[historyIdx] ?? "";
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (historyIdx > 0) historyIdx--;
      else historyIdx = -1;
      cmd = historyIdx >= 0 ? history[historyIdx] ?? "" : "";
    }
  }

  const palette = {
    background: "#0a0f1c",
    foreground: "#d6e2f6",
    cursor: "#38bdf8",
    cursorAccent: "#0a0f1c",
    selectionBackground: "rgba(56,189,248,.25)",
    black: "#0a0f1c",
    red: "#f87171",
    green: "#4ade80",
    yellow: "#facc15",
    blue: "#60a5fa",
    magenta: "#c084fc",
    cyan: "#22d3ee",
    white: "#d6e2f6",
    brightBlack: "#64748b",
    brightRed: "#fca5a5",
    brightGreen: "#86efac",
    brightYellow: "#fde047",
    brightBlue: "#93c5fd",
    brightMagenta: "#d8b4fe",
    brightCyan: "#67e8f9",
    brightWhite: "#f8fafc",
  };

  $effect(() => {
    const node = el;
    if (!node) return;
    booted = false;

    const t = new Terminal({
      fontSize: 14,
      fontFamily: "ui-monospace, 'JetBrains Mono', 'Cascadia Mono', monospace",
      lineHeight: 1.3,
      cursorBlink: true,
      scrollback: 8000,
      convertEol: true,
      disableStdin: true,
      theme: palette,
    });
    const f = new FitAddon();
    t.loadAddon(f);
    t.open(node);
    requestAnimationFrame(() => {
      try {
        f.fit();
      } catch {
        /* ignore */
      }
    });
    term = t;
    fit = f;

    const unlisteners: UnlistenFn[] = [];
    const started = (Date.now() / 1000) | 0;

    const writeLine = (raw: string) => {
      for (const seg of cleanConsoleLine(raw)) t.write(highlightLogAnsi(seg) + "\r\n");
    };

    consoleApi
      .tail(serverId)
      .then((lines) => {
        for (const l of lines) {
          for (const seg of cleanConsoleLine(l)) t.write(highlightLogAnsi(seg) + "\r\n");
        }
        booted = true;
      })
      .catch(() => (booted = true));

    t.onData(() => {
      /* stdin is disabled — commands are sent whole via the input row below */
    });

    listen<{ line: string }>(`server:${serverId}:console`, (ev) => {
      writeLine(ev.payload.line);
    }).then((u) => unlisteners.push(u));
    listen<{ id: string; state: string; code?: number | null }>("server-status", (ev) => {
      if (ev.payload.id !== serverId) return;
      if (ev.payload.state === "stopped") {
        t.write("\r\n\r\n\u001b[90m=== Server stopped ===\u001b[0m\r\n");
      }
    }).then((u) => unlisteners.push(u));

    const onResize = () => {
      try {
        f.fit();
      } catch {
        /* ignore */
      }
    };
    window.addEventListener("resize", onResize);
    const interval = window.setInterval(onResize, 600);
    void started;

    return () => {
      window.removeEventListener("resize", onResize);
      window.clearInterval(interval);
      for (const u of unlisteners) {
        try {
          u();
        } catch {
          /* ignore */
        }
      }
      t.dispose();
    };
  });

  async function clearConsole() {
    term?.clear();
    try {
      await consoleApi.clear(serverId);
    } catch {
      /* ignore */
    }
  }
</script>

<div class="rounded-lg border border-edge overflow-hidden bg-[#0a0f1c]">
  <div class="flex items-center justify-between px-3 py-1.5 border-b border-white/5 bg-black/30">
    <span class="text-xs font-medium text-fg-dim uppercase tracking-wide flex items-center gap-2">
      <span class="w-1.5 h-1.5 rounded-full {booted ? 'bg-emerald-500' : 'bg-amber-400 animate-pulse'}"></span>
      {t("Server Console")}
    </span>
    <Button variant="ghost" size="xs" onclick={clearConsole} title={t("Clear console")}>
      <Eraser size={12} /> {t("Clear")}
    </Button>
  </div>
  <div class="p-2" bind:this={el} style="height: {height}"></div>
  <form
    class="flex items-center gap-2 px-3 py-2 border-t border-white/5 bg-black/30"
    onsubmit={(e) => {
      e.preventDefault();
      void sendCommand();
    }}
  >
    <span class="text-brand-400 font-semibold text-sm select-none">/</span>
    <div class="relative flex-1 min-w-0">
      {#if showHints && suggestions.length > 0}
        <ul
          class="absolute bottom-full left-0 mb-1 w-[min(420px,90vw)] max-h-56 overflow-y-auto rounded-lg border border-edge bg-surface-1/95 backdrop-blur shadow-xl z-20 py-1"
          role="listbox"
        >
          {#each suggestions as s, i (s.name)}
            <li
              class="flex items-baseline gap-2 px-3 py-1.5 cursor-pointer text-sm {i === suggestionIdx
                ? 'bg-brand-500/15'
                : 'hover:bg-surface-2'}"
              role="option"
              aria-selected={i === suggestionIdx}
              onmousedown={(e) => {
                e.preventDefault();
                acceptSuggestion(s);
              }}
              onmouseenter={() => (suggestionIdx = i)}
            >
              <span class="font-mono text-brand-400">{s.name}</span>
              <span class="text-fg-dim text-xs truncate">{s.hint}</span>
            </li>
          {/each}
        </ul>
      {/if}
      <input
        bind:value={cmd}
        onkeydown={onCmdKey}
        onfocus={() => (showHints = true)}
        onblur={() => (showHints = false)}
        placeholder={t("Type a server command and press Enter…")}
        class="w-full bg-transparent outline-none text-sm text-slate-100 placeholder:text-fg-dim/70 font-mono"
        spellcheck="false"
        autocomplete="off"
      />
    </div>
    <button
      type="submit"
      class="p-1.5 rounded-md bg-brand-500/15 text-brand-400 hover:bg-brand-500/25 border border-brand-500/20 cursor-pointer transition"
      title={t("Send command")}
    >
      <Send size={14} />
    </button>
  </form>
</div>