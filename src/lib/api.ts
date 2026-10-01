import { invoke } from "@tauri-apps/api/core";
import { isTauri } from "./env";
import type {
  CreateServerRequest,
  CfMod,
  ExportResult,
  FileEntry,
  FileItem,
  ImportRequest,
  JavaInstall,
  LogView,
  ModrinthProject,
  ModrinthVersion,
  PlayersView,
  ServerListItem,
  ServerMeta,
  ServerPing,
  ScheduleRule,
  Settings,
  StatusInfo,
  VelocityCandidate,
  VelocityConfig,
  WorldInfo,
  DatapackInfo,
  WorldSettings,
} from "./types";

export const call = <T>(cmd: string, args: Record<string, unknown> = {}): Promise<T> =>
  invoke<T>(cmd, args);

// settings
export const settingsApi = {
  get: () => call<Settings>("get_settings"),
  set: (s: Settings) => call<Settings>("set_settings", { newSettings: s }),
  detectJava: () => call<JavaInstall[]>("detect_java"),
};

// servers
export const serversApi = {
  list: () => call<ServerListItem[]>("list_servers"),
  get: (id: string) => call<ServerMeta>("get_server", { id }),
  status: (id: string) => call<StatusInfo>("server_status", { id }),
  start: (id: string) => call<void>("start_server", { id }),
  stop: (id: string) => call<void>("stop_server", { id }),
  restart: (id: string) => call<void>("restart_server", { id }),
  kill: (id: string) => call<void>("kill_server", { id }),
  create: (req: CreateServerRequest) => call<ServerMeta>("create_server", { req }),
  delete: (id: string) => call<void>("delete_server", { id }),
  ping: (id: string) => call<ServerPing>("server_ping", { id }),
  setBackupConfig: (id: string, auto_backup_days: number, auto_backup_keep: number) =>
    call<void>("server_backup_config_set", { id, autoBackupDays: auto_backup_days, autoBackupKeep: auto_backup_keep }),
  setJava: (id: string, java_path: string | null) =>
    call<ServerMeta>("server_set_java", { id, javaPath: java_path }),
  resolveJava: (id: string) => call<string>("server_java_resolve", { id }),
  launchCommand: (id: string) => call<string>("server_launch_command", { id }),
  changeVersion: (id: string, mc_version: string, loader_version: string | null) =>
    call<ServerMeta>("server_change_version", { id, mcVersion: mc_version, loaderVersion: loader_version }),
  updateConfig: (
    id: string,
    config: {
      jvm_preset?: string;
      jvm_args?: string;
      auto_start_on_boot?: boolean;
      auto_restart_on_crash?: boolean;
      schedule?: ScheduleRule[];
      min_ram?: number;
      max_ram?: number;
    },
  ) => call<ServerMeta>("server_update_config", { id, config }),
};

export const consoleApi = {
  send: (id: string, line: string) => call<void>("console_send", { id, line }),
  tail: (id: string) => call<string[]>("console_tail", { id }),
  clear: (id: string) => call<void>("console_clear", { id }),
};

export const logApi = {
  get: (id: string, maxLines = 600) => call<LogView>("server_logs", { id, maxLines }),
};

export const versionsApi = {
  coreVersions: (core: string, snapshots = false) =>
    call<string[]>("fetch_core_versions", { core, includeSnapshots: snapshots }),
  loaders: (core: string, mc: string) => call<string[]>("fetch_loaders", { core, mc }),
};

export const propertiesApi = {
  get: (id: string) => call<Record<string, string>>("server_properties_get", { id }),
  set: (id: string, values: Record<string, string>) =>
    call<Record<string, string>>("server_properties_set", { id, values }),
  iconGet: (id: string) => call<string | null>("server_icon_get", { id }),
  iconSet: (id: string, path: string) => call<string | null>("server_icon_set", { id, path }),
};

export const playersApi = {
  get: (id: string) => call<PlayersView>("get_players", { id }),
  command: (id: string, command: string) => call<void>("run_command", { id, command }),
};

export const modsApi = {
  search: (query: string, core: string, mc: string, limit = 12) =>
    call<ModrinthProject[]>("modrinth_search", { query, core, mc, limit }),
  versions: (projectId: string, core: string, mc: string) =>
    call<ModrinthVersion[]>("modrinth_project_versions", { projectId, core, mc }),
  install: (id: string, projectId: string, mc: string) =>
    call<{ filename: string; folder: string; path: string }>("install_mod", { id, projectId, mc }),
  list: (id: string) => call<FileItem[]>("list_mods", { id }),
  setEnabled: (id: string, filename: string, enabled: boolean) =>
    call<FileItem>("set_mod_enabled", { id, filename, enabled }),
  installLocal: (id: string, path: string) => call<string>("install_local_mod", { id, path }),
  curseforge: (query: string, mc: string, core: string, limit = 12) =>
    call<CfMod[]>("curseforge_search", { query, mc, core, limit }),
  remove: (id: string, filename: string) => call<void>("delete_mod", { id, filename }),
  update: (id: string, filename: string) =>
    call<{ filename: string; folder: string; path: string }>("update_mod", { id, filename }),
  icon: (id: string, filename: string) => call<string | null>("mod_icon", { id, name: filename }),
};

export const worldsApi = {
  list: (id: string) => call<WorldInfo[]>("worlds_list", { id }),
  backup: (id: string, world: string) => call<string>("worlds_backup", { id, world }),
  listBackups: (id: string) => call<string[]>("worlds_list_backups", { id }),
  restore: (id: string, zipfile: string) => call<void>("worlds_restore", { id, zipfile }),
  delete: (id: string, world: string) => call<void>("worlds_delete", { id, world }),
  deleteBackup: (id: string, zipfile: string) => call<void>("worlds_delete_backup", { id, zipfile }),
  renameBackup: (id: string, from: string, to: string) =>
    call<string>("worlds_rename_backup", { id, from, to }),
  datapacks: (id: string, world: string) => call<DatapackInfo[]>("worlds_datapacks", { id, world }),
  installDatapack: (id: string, world: string, source: string) =>
    call<string>("worlds_install_datapack", { id, world, source }),
  downloadDatapack: (id: string, world: string, url: string) =>
    call<string>("worlds_download_datapack", { id, world, url }),
  toggleDatapack: (id: string, world: string, name: string, enabled: boolean) =>
    call<void>("worlds_toggle_datapack", { id, world, name, enabled }),
  deleteDatapack: (id: string, world: string, name: string) =>
    call<void>("worlds_delete_datapack", { id, world, name }),
  settings: (id: string, world: string) => call<WorldSettings>("worlds_settings", { id, world }),
  searchModrinth: (query: string, mc: string, limit = 12) =>
    call<ModrinthProject[]>("worlds_datapack_search", { query, mc, limit }),
  modrinthIcon: (url: string) => call<string | null>("modrinth_icon", { url }),
  installDatapackModrinth: (id: string, world: string, projectId: string, mc: string) =>
    call<string>("worlds_install_datapack_modrinth", { id, world, projectId, mc }),
};

export const importerApi = {
  import: (req: ImportRequest) => call<ServerMeta>("import_pack", { req }),
};

export const filesApi = {
  list: (id: string, path: string = "") => call<FileEntry[]>("files_list", { id, path }),
  read: (id: string, path: string) => call<string>("files_read", { id, path }),
  readDataUrl: (id: string, path: string) => call<string>("files_read_data_url", { id, path }),
  upload: (id: string, path: string, sources: string[]) =>
    call<number>("files_upload", { id, path, sources }),
  write: (id: string, path: string, content: string) =>
    call<void>("files_write", { id, path, content }),
  create: (id: string, path: string, isDir: boolean) =>
    call<void>("files_create", { id, path, isDir }),
  remove: (id: string, paths: string[]) => call<number>("files_delete", { id, paths }),
  rename: (id: string, path: string, newName: string) =>
    call<void>("files_rename", { id, path, newName }),
  copyMove: (id: string, sources: string[], destDir: string, isMove: boolean) =>
    call<void>("files_copy_move", { id, sources, destDir, isMove }),
  zip: (id: string, sources: string[], baseDir: string, name: string) =>
    call<string>("files_zip", { id, sources, baseDir, name }),
  unzip: (id: string, path: string, destDir: string) =>
    call<void>("files_unzip", { id, path, destDir }),
};

export const exportApi = {
  exportServer: (id: string) => call<ExportResult>("export_server", { id }),
};

export const velocityApi = {
  get: (id: string) => call<VelocityConfig>("velocity_get", { id }),
  save: (id: string, cfg: VelocityConfig) => call<VelocityConfig>("velocity_save", { id, cfg }),
  generateSecret: (id: string) => call<string>("velocity_generate_secret", { id }),
  candidates: () => call<VelocityCandidate[]>("velocity_candidates"),
};

export const miscApi = {
  openUrl: (url: string) => call<void>("open_url", { url }),
  uiMetrics: () => call<{ scale: number; physical: { width: number; height: number }; inner: { width: number; height: number } }>("ui_metrics"),
  uiLog: (msg: string) => call<void>("ui_log", { msg }),
};

/** OS-level autostart of the app itself (desktop / main window hidden). */
export const autostartApi = {
  isEnabled: async (): Promise<boolean> => {
    if (!isTauri()) return false;
    const { isEnabled } = await import("@tauri-apps/plugin-autostart");
    return isEnabled();
  },
  setEnabled: async (enabled: boolean): Promise<void> => {
    if (!isTauri()) return;
    const mod = await import("@tauri-apps/plugin-autostart");
    if (enabled) await mod.enable();
    else await mod.disable();
  },
};