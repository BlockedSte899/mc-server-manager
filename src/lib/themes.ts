export interface ThemeVars {
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

export interface ThemeDef {
  id: string;
  label: string;
  vars: ThemeVars;
}

// Kanagawa Paper colours taken from the user's Stylix palette
// (home-manager/modules/stylix.nix): mSurface/base00 etc. mapped onto the app.
export const THEMES: ThemeDef[] = [
  {
    id: "dark",
    label: "Dark",
    vars: {
      surface0: "#0b1220",
      surface1: "#111a2e",
      surface2: "#182338",
      surface3: "#22304a",
      edge: "#2a3a58",
      fg: "#e6edf7",
      fg_dim: "#8fa3bf",
      brand: "#27c985",
      accent: "#38bdf8",
      magenta: "#c084fc",
    },
  },
  {
    id: "midnight",
    label: "Midnight",
    vars: {
      surface0: "#070b18",
      surface1: "#0d1430",
      surface2: "#16204a",
      surface3: "#22305f",
      edge: "#33437a",
      fg: "#eef2ff",
      fg_dim: "#93a3cf",
      brand: "#27c985",
      accent: "#60a5fa",
      magenta: "#c084fc",
    },
  },
  {
    id: "kanagawa-paper",
    label: "Kanagawa Paper",
    vars: {
      surface0: "#1f1f28",
      surface1: "#2a2a37",
      surface2: "#363646",
      surface3: "#3f3f4d",
      edge: "#435965",
      fg: "#dcd7ba",
      fg_dim: "#c8c093",
      brand: "#8ea49e",
      accent: "#c4b28a",
      magenta: "#938aa9",
    },
  },
  {
    id: "catppuccin",
    label: "Catppuccin Mocha",
    vars: {
      surface0: "#11111b",
      surface1: "#1e1e2e",
      surface2: "#313244",
      surface3: "#45475a",
      edge: "#585b70",
      fg: "#cdd6f4",
      fg_dim: "#a6adc8",
      brand: "#a6e3a1",
      accent: "#89b4fa",
      magenta: "#f5c2e7",
    },
  },
  {
    id: "gruvbox",
    label: "Gruvbox Dark",
    vars: {
      surface0: "#1d2021",
      surface1: "#282828",
      surface2: "#32302f",
      surface3: "#3c3836",
      edge: "#504945",
      fg: "#ebdbb2",
      fg_dim: "#a89984",
      brand: "#b8bb26",
      accent: "#83a598",
      magenta: "#d3869b",
    },
  },
  {
    id: "tokyonight",
    label: "Tokyo Night",
    vars: {
      surface0: "#16161e",
      surface1: "#1f2335",
      surface2: "#24283b",
      surface3: "#2f334d",
      edge: "#3b4261",
      fg: "#c0caf5",
      fg_dim: "#888ba6",
      brand: "#7aa2f7",
      accent: "#7dcfff",
      magenta: "#bb9af7",
    },
  },
  {
    id: "nord",
    label: "Nord",
    vars: {
      surface0: "#0f1b24",
      surface1: "#1d2a37",
      surface2: "#263544",
      surface3: "#31414f",
      edge: "#3b4c59",
      fg: "#eceff4",
      fg_dim: "#a5b4c2",
      brand: "#88c0d0",
      accent: "#81a1c1",
      magenta: "#b48ead",
    },
  },
];

export const CUSTOM_THEME_ID = "custom";

export const DEFAULT_VARS: ThemeVars = THEMES[0].vars;

const clamp = (n: number) => Math.max(0, Math.min(255, Math.round(n)));

function hexToRgb(hex: string): [number, number, number] {
  let h = hex.replace("#", "");
  if (h.length === 3) h = h.split("").map((c) => c + c).join("");
  const n = parseInt(h, 16);
  if (Number.isNaN(n) || h.length !== 6) return [26, 26, 26];
  return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
}

function rgbToHex(r: number, g: number, b: number): string {
  return "#" + [r, g, b].map((c) => clamp(c).toString(16).padStart(2, "0")).join("");
}

/** Lightens toward white by `pct`%. */
export function lighten(hex: string, pct: number): string {
  const [r, g, b] = hexToRgb(hex);
  return rgbToHex(r + (255 - r) * pct, g + (255 - g) * pct, b + (255 - b) * pct);
}

/** Darkens by `pct`%. */
export function darken(hex: string, pct: number): string {
  const [r, g, b] = hexToRgb(hex);
  return rgbToHex(r * (1 - pct), g * (1 - pct), b * (1 - pct));
}

export const applyThemeVars = (vars: ThemeVars, root: HTMLElement) => {
  root.style.setProperty("--color-surface-0", vars.surface0);
  root.style.setProperty("--color-surface-1", vars.surface1);
  root.style.setProperty("--color-surface-2", vars.surface2);
  root.style.setProperty("--color-surface-3", vars.surface3);
  root.style.setProperty("--color-edge", vars.edge);
  root.style.setProperty("--color-fg", vars.fg);
  root.style.setProperty("--color-fg-dim", vars.fg_dim);
  root.style.setProperty("--color-brand-50", lighten(vars.brand, 0.55));
  root.style.setProperty("--color-brand-500", vars.brand);
  root.style.setProperty("--color-brand-600", darken(vars.brand, 0.15));
  root.style.setProperty("--color-brand-700", darken(vars.brand, 0.28));
  root.style.setProperty("--color-accent", vars.accent);
  root.style.setProperty("--color-magenta", vars.magenta);
};