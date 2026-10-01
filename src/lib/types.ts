export interface CustomTheme {
  surface0: string;
  surface1: string;
  surface2: string;
  surface3: string;
  edge: string;
  fg: string;
  fg_dim: string;
  brand: string;
  accent: string;
  magenta: string;
}

export interface Settings {
  servers_dir: string;
  java_overrides: Record<string, string>;
  curseforge_api_key: string | null;
  min_ram_default: number;
  max_ram_default: number;
  console_max_lines: number;
  ui_scale: number;
  theme: string;
  tray_enabled: boolean;
  lang: string;
  custom_theme: CustomTheme | null;
  notify_desktop: boolean;
  notify_on_start: boolean;
  notify_on_stop: boolean;
  notify_on_crash: boolean;
  notify_on_backup: boolean;
}

export interface JavaInstall {
  major: number;
  path: string;
  vendor: string;
  source: string;
}

export interface ScheduleRule {
  day: string;
  hour: number;
  minute: number;
  action: string;
}

export interface ServerMeta {
  name: string;
  id: string;
  java: number;
  core: string;
  mc_version: string;
  loader_version: string | null;
  build: string | null;
  jar: string;
  min_ram: number;
  max_ram: number;
  created_at: string;
  is_proxy: boolean;
  source_url: string | null;
  warnings: string[];
  auto_backup_days: number;
  auto_backup_keep: number;
  java_path: string | null;
  jvm_preset: string;
  jvm_args: string;
  auto_start_on_boot: boolean;
  auto_restart_on_crash: boolean;
  schedule: ScheduleRule[];
}

export interface StatusInfo {
  state: string;
  pid: number | null;
  uptime_secs: number | null;
}

export interface LogView {
  path: string;
  lines: string[];
  total_lines: number;
  truncated: boolean;
}

export interface ServerListItem {
  meta: ServerMeta;
  status: StatusInfo;
  players: number | null;
}

export interface StatusEvent {
  id: string;
  state: string;
  code: number | null;
}

export interface ServerMetrics {
  id: string;
  cpu_percent: number | null;
  mem_bytes: number | null;
  system_cpu_percent: number;
  system_mem_used: number;
  system_mem_total: number;
}

export interface DownloadProgress {
  key: string;
  received: number;
  total: number;
  percent: number;
}

export interface ServerPing {
  online: number | null;
  max: number | null;
  players: string[];
  motd: string | null;
  version_name: string | null;
  protocol: number | null;
  latency_ms: number;
}

export interface CreateServerRequest {
  name: string;
  java: number;
  core: string;
  mc_version: string;
  loader_version: string | null;
  min_ram: number;
  max_ram: number;
  accept_eula: boolean;
}

export interface Entry {
  name: string;
  uuid: string;
}

export interface PlayersView {
  online: string[];
  max_online: number;
  whitelist_enabled: boolean;
  whitelist: Entry[];
  banned: Entry[];
  banned_ips: string[];
  ops: Entry[];
  usercache: Entry[];
}

export interface ModrinthProject {
  project_id: string;
  slug: string | null;
  title: string;
  description: string | null;
  project_type: string;
  downloads: number;
  icon_url: string | null;
  server_side: string | null;
  client_side: string | null;
  game_versions: string[];
  loaders: string[];
}

export interface ModrinthVersion {
  id: string;
  project_id: string;
  name: string;
  version_number: string;
  date_published: string;
  game_versions: string[];
  loaders: string[];
  files: VersionFile[];
  dependencies: { project_id: string | null; dependency_type: string | null }[];
}

export interface VersionFile {
  filename: string;
  url: string;
  hashes: Record<string, string>;
  size: number;
  primary: boolean;
  env?: { client?: string | null; server?: string | null };
}

export interface FileItem {
  name: string;
  enabled: boolean;
  size: number;
  modified: string;
}

export interface FileEntry {
  name: string;
  is_dir: boolean;
  size: number;
  modified: string;
}

export interface ExportResult {
  path: string;
  name: string;
  files: number;
}

export interface WorldInfo {
  name: string;
  size_bytes: number;
  modified: string;
  backup: boolean;
}

export interface DatapackInfo {
  name: string;
  enabled: boolean;
  kind: string;
}

export interface Gamerule {
  name: string;
  value: string;
}

export interface WorldSettings {
  seed: number | null;
  difficulty: string;
  hardcore: boolean;
  allow_cheats: boolean;
  gamerules: Gamerule[];
}

export interface VelocityConfig {
  bind: string;
  motd: string;
  show_max_players: number;
  online_mode: boolean;
  force_key_authentication: boolean;
  prevent_client_proxy_connections: boolean;
  player_info_forwarding_mode: string;
  forwarding_secret: string;
  servers: Record<string, { address: string; enabled: boolean }>;
  forced_hosts: Record<string, string>;
  try?: string[];
}

export interface VelocityCandidate {
  id: string;
  name: string;
  port: number;
  address: string;
}

export interface CfMod {
  id: number;
  name: string;
  summary: string;
  slug: string;
  downloadCount: number;
  logo?: { url?: string | null };
  latestFilesIndexes: { gameVersion: string | null; fileId: number }[];
}

export interface ImportRequest {
  path: string;
  name: string | null;
  java: number;
  min_ram: number;
  max_ram: number;
  accept_eula: boolean;
}

export type CoreKind =
  | "paper"
  | "purpur"
  | "spigot"
  | "bukkit"
  | "velocity"
  | "forge"
  | "neoforge"
  | "vanilla";

export const templateColor = (core: string): string => {
  switch (core) {
    case "paper":
      return "bg-orange-500/15 text-orange-300 border-orange-500/30";
    case "purpur":
      return "bg-purple-500/15 text-purple-300 border-purple-500/30";
    case "spigot":
    case "bukkit":
      return "bg-amber-500/15 text-amber-300 border-amber-500/30";
    case "velocity":
      return "bg-sky-500/15 text-sky-300 border-sky-500/30";
    case "forge":
    case "neoforge":
      return "bg-red-500/15 text-red-300 border-red-500/30";
    case "fabric":
      return "bg-emerald-500/15 text-emerald-300 border-emerald-500/30";
    case "vanilla":
      return "bg-slate-500/15 text-slate-300 border-slate-500/30";
    default:
      return "bg-slate-500/15 text-slate-300 border-slate-500/30";
  }
};