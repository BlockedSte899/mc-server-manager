import type { ServerListItem, Settings, DownloadProgress, ServerMetrics, ServerMeta } from "./types";
import { settingsApi, serversApi } from "./api";

export const settings = $state<Settings>({
  servers_dir: "",
  java_overrides: {},
  curseforge_api_key: null,
  min_ram_default: 512,
  max_ram_default: 4096,
  console_max_lines: 2000,
  ui_scale: 100,
  theme: "dark",
  tray_enabled: true,
  lang: "en",
  custom_theme: null,
  notify_desktop: true,
  notify_on_start: true,
  notify_on_stop: false,
  notify_on_crash: true,
  notify_on_backup: false,
});

export const servers = $state<ServerListItem[]>([]);
export const statuses = $state<Record<string, string>>({});
export const metrics = $state<Record<string, ServerMetrics>>({});
export const downloads = $state<Record<string, DownloadProgress>>({});

export interface Toast {
  id: number;
  message: string;
  kind: "info" | "success" | "error";
}

let toastId = 0;
export const toasts = $state<Toast[]>([]);

export function toast(message: string, kind: Toast["kind"] = "info") {
  const id = ++toastId;
  toasts.push({ id, message, kind });
  setTimeout(() => {
    const idx = toasts.findIndex((t) => t.id === id);
    if (idx >= 0) toasts.splice(idx, 1);
  }, 4500);
}

export async function loadSettings() {
  try {
    const s = await settingsApi.get();
    Object.assign(settings, s);
  } catch (e) {
    toast(String(e), "error");
  }
}

export async function refreshServers() {
  try {
    const list = await serversApi.list();
    servers.splice(0, servers.length, ...list);
    for (const { meta, status } of list) {
      statuses[meta.id] = status.state;
      const running = status.state === "running" || status.state === "starting";
      if (!running && metrics[meta.id]) {
        delete metrics[meta.id];
      }
    }
  } catch (e) {
    toast(String(e), "error");
  }
}

export function setStatus(id: string, state: string) {
  statuses[id] = state;
}

/** Replaces one server's meta in the cached list (e.g. right after a Java change). */
export function patchServerMeta(id: string, meta: ServerMeta) {
  const i = servers.findIndex((s) => s.meta.id === id);
  if (i >= 0) servers[i] = { ...servers[i], meta };
}

export const isRunning = (id: string) => statuses[id] === "running" || statuses[id] === "stopping";

export function fmtBytes(n: number): string {
  if (!n || n <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let i = 0;
  let v = n;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v.toFixed(i === 0 ? 0 : 1)} ${units[i]}`;
}

export function fmtDuration(secs: number | null): string {
  if (!secs) return "—";
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  const pad = (x: number) => String(x).padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

/**
 * True while the main window is hidden (sent to the tray). Background
 * polling is pointless then, so timers skip their work to keep the webview
 * idle and the tray footprint low.
 */
export const windowHidden = () =>
  typeof document !== "undefined" && document.hidden === true;