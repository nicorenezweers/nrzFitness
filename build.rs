//! Erzeugt `style/tailwind.css` (Eingabe für die Tailwind-CLI) aus dem Theme unten.
//! Das nrzUI-Theme lebt so im Rust-Code; die erzeugte Datei ist in `.gitignore`.

use std::path::Path;

const TAILWIND_INPUT: &str = r##"@import "tailwindcss";
@source "../src/**/*.rs";

@custom-variant dark (&:is(.dark *));

:root {
  --font-mono: ui-monospace, "SF Mono", "JetBrains Mono", Menlo, Consolas, monospace;
  --radius: 0.75rem;
  --background: oklch(1 0 0);
  --foreground: oklch(0.145 0 0);
  --card: oklch(1 0 0);
  --card-foreground: oklch(0.145 0 0);
  --popover: oklch(0.985 0 0);
  --popover-foreground: oklch(0.145 0 0);
  --primary: oklch(0.205 0 0);
  --primary-foreground: oklch(0.985 0 0);
  --primary-light: oklch(0.95 0 0);
  --primary-dark: oklch(0.145 0 0);
  --secondary: oklch(0.92 0 0);
  --secondary-foreground: oklch(0.205 0 0);
  --muted: oklch(0.985 0 0);
  --muted-foreground: oklch(0.556 0 0);
  --accent: oklch(0.97 0 0);
  --accent-foreground: oklch(0.205 0 0);
  --destructive: oklch(0.577 0.245 27.325);
  --destructive-foreground: oklch(0.985 0 0);
  --border: oklch(0.922 0 0);
  --input: oklch(0.922 0 0);
  --ring: oklch(0.708 0 0);
  --sidebar: oklch(0.985 0 0);
  --sidebar-foreground: oklch(0.145 0 0);
  --sidebar-primary: oklch(0.205 0 0);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.97 0 0);
  --sidebar-accent-foreground: oklch(0.205 0 0);
  --sidebar-border: oklch(0.922 0 0);
  --sidebar-ring: oklch(0.708 0 0);
  /* Datenfarben (Diagramme, Makros) */
  --kcal: oklch(0.62 0.17 150);
  --protein: oklch(0.6 0.15 250);
  --carbs: oklch(0.72 0.15 75);
  --fat: oklch(0.63 0.19 15);
  --water: oklch(0.65 0.13 230);
  --good: oklch(0.62 0.17 150);
  --warn: oklch(0.72 0.15 75);
  --bad: oklch(0.6 0.2 25);
}

/* nrzUI: dark theme = nrz palette (greys, no blue accents), shared with all nrzApps. */
.dark {
  color-scheme: dark;
  --background: oklch(0.18 0 0);
  --foreground: oklch(0.985 0 0);
  --card: oklch(0.205 0 0);
  --card-foreground: oklch(0.985 0 0);
  --popover: oklch(0.235 0 0);
  --popover-foreground: oklch(0.985 0 0);
  --primary: oklch(0.922 0 0);
  --primary-foreground: oklch(0.205 0 0);
  --primary-light: oklch(0.3 0 0);
  --primary-dark: oklch(0.985 0 0);
  --secondary: oklch(0.269 0 0);
  --secondary-foreground: oklch(0.985 0 0);
  --muted: oklch(0.269 0 0);
  --muted-foreground: oklch(0.708 0 0);
  --accent: oklch(0.3 0 0);
  --accent-foreground: oklch(0.985 0 0);
  --destructive: oklch(0.704 0.191 22.216);
  --destructive-foreground: oklch(0.985 0 0);
  --border: oklch(1 0 0 / 10%);
  --input: oklch(1 0 0 / 15%);
  --ring: oklch(0.556 0 0);
  --sidebar: oklch(0.205 0 0);
  --sidebar-foreground: oklch(0.985 0 0);
  --sidebar-primary: oklch(0.922 0 0);
  --sidebar-primary-foreground: oklch(0.985 0 0);
  --sidebar-accent: oklch(0.269 0 0);
  --sidebar-accent-foreground: oklch(0.985 0 0);
  --sidebar-border: oklch(1 0 0 / 10%);
  --sidebar-ring: oklch(0.556 0 0);
  --kcal: oklch(0.78 0.15 155);
  --protein: oklch(0.74 0.12 250);
  --carbs: oklch(0.82 0.13 85);
  --fat: oklch(0.74 0.15 20);
  --water: oklch(0.76 0.11 225);
  --good: oklch(0.78 0.15 155);
  --warn: oklch(0.82 0.13 85);
  --bad: oklch(0.72 0.17 25);
}

@theme inline {
  --color-background: var(--background);
  --color-foreground: var(--foreground);
  --color-card: var(--card);
  --color-card-foreground: var(--card-foreground);
  --color-popover: var(--popover);
  --color-popover-foreground: var(--popover-foreground);
  --color-primary: var(--primary);
  --color-primary-foreground: var(--primary-foreground);
  --color-primary-light: var(--primary-light);
  --color-primary-dark: var(--primary-dark);
  --color-secondary: var(--secondary);
  --color-secondary-foreground: var(--secondary-foreground);
  --color-muted: var(--muted);
  --color-muted-foreground: var(--muted-foreground);
  --color-accent: var(--accent);
  --color-accent-foreground: var(--accent-foreground);
  --color-destructive: var(--destructive);
  --color-destructive-foreground: var(--destructive-foreground);
  --color-border: var(--border);
  --color-input: var(--input);
  --color-ring: var(--ring);
  --color-sidebar: var(--sidebar);
  --color-sidebar-foreground: var(--sidebar-foreground);
  --color-sidebar-primary: var(--sidebar-primary);
  --color-sidebar-primary-foreground: var(--sidebar-primary-foreground);
  --color-sidebar-accent: var(--sidebar-accent);
  --color-sidebar-accent-foreground: var(--sidebar-accent-foreground);
  --color-sidebar-border: var(--sidebar-border);
  --color-sidebar-ring: var(--sidebar-ring);
  --color-kcal: var(--kcal);
  --color-protein: var(--protein);
  --color-carbs: var(--carbs);
  --color-fat: var(--fat);
  --color-water: var(--water);
  --color-good: var(--good);
  --color-warn: var(--warn);
  --color-bad: var(--bad);
  --radius-sm: calc(var(--radius) - 4px);
  --radius-md: calc(var(--radius) - 2px);
  --radius-lg: var(--radius);
  --radius-xl: calc(var(--radius) + 4px);
}

@layer base {
  * {
    border-color: var(--border);
    outline-color: var(--ring);
  }
}

html {
  background: oklch(0.18 0 0);
  height: 100vh;
  height: 100dvh;
  overflow: hidden;
}

body {
  @apply h-full bg-background text-foreground antialiased;
  -webkit-tap-highlight-color: transparent;
  overscroll-behavior: none;
  font-feature-settings: "tnum" 1, "cv11" 1;
}

/* Kein Zoom beim Fokussieren von Eingaben auf iOS: mind. 16px auf kleinen Bildschirmen. */
@media (max-width: 639px) {
  input, select, textarea { font-size: 16px !important; }
}

input[type="number"]::-webkit-inner-spin-button,
input[type="number"]::-webkit-outer-spin-button { -webkit-appearance: none; margin: 0; }
input[type="number"] { -moz-appearance: textfield; }

.no-scrollbar::-webkit-scrollbar { display: none; }
.no-scrollbar { scrollbar-width: none; }

@keyframes sheet-up { from { transform: translateY(100%); } to { transform: translateY(0); } }
@keyframes fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes pop-in { from { opacity: 0; transform: scale(.96) translateY(6px); } to { opacity: 1; transform: none; } }
@keyframes toast-in { from { opacity: 0; transform: translate(-50%, 12px); } to { opacity: 1; transform: translate(-50%, 0); } }
.anim-fade { animation: fade-in .18s ease-out both; }
.anim-sheet { animation: sheet-up .28s cubic-bezier(.32,.72,0,1) both; }
.anim-pop { animation: pop-in .2s ease-out both; }
.anim-toast { animation: toast-in .22s ease-out both; }
@media (min-width: 640px) { .sm-anim-pop { animation: pop-in .2s ease-out both; } }
.ring-track { transition: stroke-dashoffset .6s cubic-bezier(.32,.72,0,1); }
.pt-safe { padding-top: env(safe-area-inset-top); }
.pb-safe { padding-bottom: env(safe-area-inset-bottom); }

/* Dioxus WASM mount point */
#main {
  height: 100%;
}

input[type="search"]::-webkit-search-cancel-button,
input[type="search"]::-webkit-search-decoration { -webkit-appearance: none; appearance: none; }

html:not(.dark) { background: oklch(1 0 0); }
"##;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let path = Path::new("style/tailwind.css");
    if std::fs::read_to_string(path).ok().as_deref() != Some(TAILWIND_INPUT) {
        let _ = std::fs::create_dir_all("style");
        std::fs::write(path, TAILWIND_INPUT).expect("style/tailwind.css schreiben");
    }
}
