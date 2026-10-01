/** ANSI escape sequences (SGR colour codes etc.). */
export const ANSI_RE = /\u001b\[\d*(?:;\d+)*[A-Za-z]/g;

/** Lines the server emits automatically as `/list` summaries. */
export const LIST_SUMMARY_RE =
  /players\s+online|player\(s\)\s+connected|players\s+currently\s+connected/i;

/**
 * Cleans one raw console/log line for display. Strips ANSI codes and
 * `<--[HERE]` parse-error carets, splits `\r`-joined fragments, and drops
 * the automatic "There are N of a max of M players online: …" noise entirely
 * (the authoritative list is shown on the Players tab instead).
 * Returns segments to render; may be empty.
 */
export function cleanConsoleLine(raw: string): string[] {
  const plain = raw.replace(ANSI_RE, "").replace(/<--\[HERE\]/g, "");
  const out: string[] = [];
  for (let seg of plain.split(/\r+/)) {
    seg = seg.trim();
    if (!seg) continue;
    if (LIST_SUMMARY_RE.test(seg)) continue;
    out.push(seg);
  }
  return out;
}

/** Everything a single log line is split into for syntax highlighting. */
const HL_RE =
  /\[(\d{1,2}:\d{2}:\d{2}(?:\.\d{1,3})?)\]|\b(ERROR|FATAL|SEVERE)\b|\b(WARN|WARNING)\b|\b(DEBUG|TRACE|FINE|FINER|FINEST)\b|\b(INFO)\b|\b[XxYyZz]=(-?\d+)/g;

type Token = { text: string; cls?: string };

const HL_ANSI: Record<string, string> = {
  ts: "90",
  error: "91",
  warn: "93",
  info: "92",
  debug: "96",
  coord: "36",
};

/**
 * Splits a (cleaned) log line into plain and highlighted tokens:
 * timestamps, log levels and x/y/z coordinates get a semantic class.
 */
export function highlightTokens(line: string): Token[] {
  const out: Token[] = [];
  let last = 0;
  let m: RegExpExecArray | null;
  HL_RE.lastIndex = 0;
  while ((m = HL_RE.exec(line))) {
    if (m.index > last) out.push({ text: line.slice(last, m.index) });
    let cls = "ts";
    if (m[2]) cls = "error";
    else if (m[3]) cls = "warn";
    else if (m[4]) cls = "debug";
    else if (m[5]) cls = "info";
    else if (m[6]) cls = "coord";
    out.push({ text: m[0], cls });
    last = m.index + m[0].length;
  }
  if (last < line.length) out.push({ text: line.slice(last) });
  return out;
}

const escapeHtml = (s: string) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** Highlighted line safe to render via {@html}. */
export function highlightLogHtml(line: string): string {
  return highlightTokens(line)
    .map((tok) =>
      tok.cls ? `<span class="hl-${tok.cls}">${escapeHtml(tok.text)}</span>` : escapeHtml(tok.text)
    )
    .join("");
}

/** Highlighted line re-encoded as ANSI SGR codes (for the terminal). */
export function highlightLogAnsi(line: string): string {
  return highlightTokens(line)
    .map((tok) => (tok.cls ? `\u001b[${HL_ANSI[tok.cls]}m${tok.text}\u001b[0m` : tok.text))
    .join("");
}