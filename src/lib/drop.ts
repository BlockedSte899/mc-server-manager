/**
 * Extracts absolute file paths from a DOM DragEvent drop.
 *
 * WebKitGTK no longer exposes `File.path` on dropped files; paths arrive as
 * `text/uri-list` (`file:///...`). This tries the File.path property first and
 * falls back to parsing the URI list, so drops work across platforms.
 */
export function pathsFromDataTransfer(dt: DataTransfer | null): string[] {
  if (!dt) return [];
  const out: string[] = [];
  const files = Array.from(dt.files ?? []);
  for (const f of files) {
    const p = (f as { path?: string }).path;
    if (p) out.push(p);
  }
  if (out.length === 0) {
    const uris = dt.getData("text/uri-list") ?? "";
    for (const u of uris.match(/file:\/\/[^\s]+/g) ?? []) {
      try {
        out.push(decodeURIComponent(u.replace(/^file:\/\/(localhost\/)?/, "")));
      } catch {
        /* ignore malformed uri */
      }
    }
  }
  return out.filter((p) => p && !p.startsWith("file:"));
}

/** Extracts paths from either a DataTransfer or a DropEvent (Tauri webview). */
export function firstPath(dt: DataTransfer, tauriPaths: string[] = []): string | null {
  const fromDom = pathsFromDataTransfer(dt);
  return fromDom[0] ?? tauriPaths[0] ?? null;
}