/**
 * Small, deliberately static list of console commands that exist across
 * vanilla Minecraft (and, where noted, proxies). It is a convenience for
 * typing, not an authoritative command registry — plugin/mod commands are
 * intentionally left out because they cannot be known ahead of time.
 */
export interface ConsoleCommand {
  name: string;
  hint: string;
}

export const CONSOLE_COMMANDS: ConsoleCommand[] = [
  { name: "help", hint: "list of available commands" },
  { name: "list", hint: "online players" },
  { name: "say", hint: "broadcast a message" },
  { name: "me", hint: "broadcast an action" },
  { name: "msg", hint: "private message a player" },
  { name: "tell", hint: "alias of msg" },
  { name: "reply", hint: "answer the last private message" },
  { name: "tps", hint: "ticks per second" },
  { name: "seed", hint: "show the world seed" },
  { name: "time set", hint: "set the world time" },
  { name: "time query", hint: "show the world time" },
  { name: "weather", hint: "clear or change the weather" },
  { name: "difficulty", hint: "change the difficulty" },
  { name: "gamemode", hint: "change a player's game mode" },
  { name: "gamerule", hint: "read or change a game rule" },
  { name: "tp", hint: "teleport a player" },
  { name: "kill", hint: "kill a player or entity" },
  { name: "give", hint: "give items to a player" },
  { name: "summon", hint: "spawn an entity" },
  { name: "clear", hint: "clear a player's inventory" },
  { name: "effect", hint: "apply a status effect" },
  { name: "whitelist", hint: "manage the whitelist" },
  { name: "kick", hint: "disconnect a player" },
  { name: "ban", hint: "ban a player" },
  { name: "pardon", hint: "unban a player" },
  { name: "op", hint: "grant operator status" },
  { name: "deop", hint: "revoke operator status" },
  { name: "save-all", hint: "flush all pending writes" },
  { name: "save-off", hint: "disable world saving" },
  { name: "save-on", hint: "re-enable world saving" },
  { name: "stop", hint: "stop the server" },
  { name: "restart", hint: "restart the server" },
  { name: "reload", hint: "reload configuration" },
  { name: "listplugins", hint: "installed plugins (plugin servers)" },
];

/** Suggestions for the current input; matches whole leading words. */
export function suggestCommands(input: string, limit = 8): ConsoleCommand[] {
  const q = input.trim().toLowerCase();
  if (!q) return [];
  const starts = CONSOLE_COMMANDS.filter((c) => c.name.startsWith(q));
  const contains = CONSOLE_COMMANDS.filter(
    (c) => !c.name.startsWith(q) && c.name.includes(q),
  );
  return [...starts, ...contains].slice(0, limit);
}
