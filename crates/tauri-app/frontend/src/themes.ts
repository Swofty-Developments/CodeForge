import { getSetting, setSetting } from "./ipc";
import { listThemes as listThemesIPC } from "./ipc";

export interface Theme {
  id: string;
  name: string;
  description: string;
  preview: string[];
  vars: Record<string, string>;
  is_custom?: boolean;
}

// Fallback theme applied when backend is unavailable (e.g. tests)
const FALLBACK_THEME: Theme = {
  id: "obsidian-forge",
  name: "Obsidian Forge",
  description: "Warm dark theme with purple undertones and blue accents",
  preview: ["#0f1012", "#151518", "#6b7cff", "#4cd694"],
  vars: {
    "--bg-base": "#0f1012",
    "--bg-card": "#151518",
    "--bg-surface": "#18181c",
    "--bg-accent": "#222228",
    "--bg-muted": "#1c1c20",
    "--bg-user-bubble": "#1e1e24",
    "--bg-hover": "rgba(255, 255, 255, 0.04)",
    "--text": "#f0eef8",
    "--text-secondary": "#b8b6c8",
    "--text-tertiary": "#807e92",
    "--primary": "#6b7cff",
    "--primary-glow": "rgba(107, 124, 255, 0.2)",
    "--primary-muted": "#5568d9",
    "--green": "#4cd694",
    "--red": "#f25f67",
    "--amber": "#f0b840",
    "--sky": "#58c4f0",
    "--purple": "#b47aff",
    "--pink": "#f07ab4",
    "--orange": "#f5943c",
    "--border": "rgba(255, 255, 255, 0.06)",
    "--border-strong": "rgba(255, 255, 255, 0.10)",
    "--border-glow": "rgba(107, 124, 255, 0.25)",
  },
};

let cachedThemes: Theme[] | null = null;

export async function getThemes(): Promise<Theme[]> {
  try {
    const themes = await listThemesIPC();
    cachedThemes = themes;
    return themes;
  } catch {
    return cachedThemes ?? [FALLBACK_THEME];
  }
}

export function applyTheme(themeId: string, themes?: Theme[]) {
  const list = themes ?? cachedThemes ?? [FALLBACK_THEME];
  const theme = list.find((t) => t.id === themeId);
  if (!theme) return;

  const root = document.documentElement;
  for (const [prop, value] of Object.entries(theme.vars)) {
    root.style.setProperty(prop, value);
  }

  setSetting("theme", themeId).catch(() => {});
}

export async function loadSavedTheme() {
  try {
    const themes = await getThemes();
    const saved = await getSetting("theme");
    if (saved) {
      applyTheme(saved, themes);
    }
  } catch {
    // Default theme is already set via CSS
  }
}
