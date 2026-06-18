# Seraph Empyrean Theme Adoption — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-theme the Seraph DAW frontend to the Empyrean visual contract — deep-space surfaces, amber `#FBBF24` interaction accent, Inter/JetBrains Mono fonts, the Seraph mark, and a status bar — driven by a generated token file.

**Architecture:** A vendored `tokens.json` (copy of the contract source) is compiled by a small Node script into `src/theme/tokens.css` (`:root` custom properties + an alias layer that maps the app's existing variable names onto the new tokens, so ~218 existing usages keep working). Static global rules and the focus ring live in a hand-authored `base.css`. Components get targeted edits for the amber accent, the mark, and the status bar.

**Tech Stack:** React 19 + Vite 7 + TypeScript, CSS Modules, Tauri 2. Fonts via `@fontsource`. Generator is plain Node ESM (`type: module` already set).

**Working dir:** `/home/volence/sonic_hacks/seraph` (run all commands from here).

**Note on TDD:** Most tasks are CSS/asset changes that aren't unit-testable; their verification is `grep` + build + visual smoke. The one logic unit — the generator — has a built-in self-check and an explicit verify step.

---

### Task 1: Vendor tokens.json with channel colors

**Files:**
- Create: `src/theme/tokens.json`

- [ ] **Step 1: Create the vendored token file**

This is a copy of `../megaforge/design/tokens.json` with one addition — a `color.channel` group holding the retuned FM/PSG/DAC data colors (blue/green/orange tuned to read on `#12151E`).

```json
{
  "$meta": {
    "name": "Empyrean Design Tokens",
    "version": "0.1.0-draft",
    "description": "Vendored copy of megaforge/design/tokens.json (source of truth). Seraph adds color.channel for FM/PSG/DAC data colors. Regenerate tokens.css via `npm run gen:theme`.",
    "direction": "dark deep-space base, luminous celestial per-tool accents"
  },
  "color": {
    "base": {
      "void":        "#0A0C12",
      "surface":     "#12151E",
      "raised":      "#1A1E2A",
      "overlay":     "#222736",
      "border":      "#2A2F3D",
      "borderStrong":"#3A4152"
    },
    "text": {
      "hi":    "#E8EAF2",
      "base":  "#B8BECE",
      "lo":    "#6E7589",
      "faint": "#474D5E"
    },
    "accent": {
      "empyrean": { "hue": "violet",  "value": "#8B7BF7", "role": "suite / umbrella" },
      "oracle":   { "hue": "cyan",    "value": "#38BDF8", "role": "emulator / debugger" },
      "aurora":   { "hue": "emerald", "value": "#34D399", "role": "art / level editor" },
      "seraph":   { "hue": "amber",   "value": "#FBBF24", "role": "DAW" },
      "crucible": { "hue": "ember",   "value": "#F97316", "role": "build node" },
      "aether":   { "hue": "lilac",   "value": "#C4B5FD", "role": "bus / connective medium" }
    },
    "semantic": {
      "success": "#34D399",
      "warning": "#FBBF24",
      "error":   "#F87171",
      "info":    "#38BDF8"
    },
    "channel": {
      "fm":  "#5EA8F5",
      "psg": "#4ADE80",
      "dac": "#FB923C"
    }
  },
  "type": {
    "font": {
      "ui":   "\"Inter\", system-ui, -apple-system, \"Segoe UI\", sans-serif",
      "mono": "\"JetBrains Mono\", ui-monospace, \"SF Mono\", \"Cascadia Code\", monospace"
    },
    "scale": {
      "xs":   { "size": "11px", "line": "16px", "use": "dense labels, table cells, register dumps" },
      "sm":   { "size": "12px", "line": "18px", "use": "secondary UI text" },
      "base": { "size": "13px", "line": "20px", "use": "default UI text" },
      "md":   { "size": "14px", "line": "22px", "use": "emphasis / panel body" },
      "lg":   { "size": "16px", "line": "24px", "use": "panel titles" },
      "xl":   { "size": "20px", "line": "28px", "use": "section headers" },
      "2xl":  { "size": "24px", "line": "32px", "use": "window / tool titles" }
    },
    "weight": { "regular": 400, "medium": 500, "semibold": 600 }
  },
  "space": { "0": "0", "1": "2px", "2": "4px", "3": "6px", "4": "8px", "5": "12px", "6": "16px", "7": "24px", "8": "32px", "9": "48px" },
  "radius": { "sm": "2px", "md": "4px", "lg": "6px", "xl": "8px", "pill": "999px" },
  "chrome": {
    "windowTitleFormat": "<Tool> — <context>",
    "commandPaletteKeybind": "Ctrl/Cmd-K",
    "notes": "Shared across all tools: title format, command-palette keybind, a bottom status bar, and the per-tool accent used for the active-tab underline, focus ring, and primary action."
  }
}
```

- [ ] **Step 2: Verify valid JSON**

Run: `node -e "JSON.parse(require('fs').readFileSync('src/theme/tokens.json','utf8')); console.log('ok')"`
Expected: `ok`

- [ ] **Step 3: Commit**

```bash
git add src/theme/tokens.json
git commit -m "feat(theme): vendor Empyrean tokens.json with channel colors"
```

---

### Task 2: Token generator script

**Files:**
- Create: `scripts/gen-theme.mjs`
- Modify: `package.json` (add `gen:theme` script)

- [ ] **Step 1: Write the generator**

```js
// scripts/gen-theme.mjs
// Generates src/theme/tokens.css from src/theme/tokens.json (vendored copy of
// megaforge/design/tokens.json). DO NOT edit tokens.css by hand — edit the JSON
// and re-run `npm run gen:theme`.
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const root = join(here, "..");
const SRC = join(root, "src/theme/tokens.json");
const OUT = join(root, "src/theme/tokens.css");

const t = JSON.parse(readFileSync(SRC, "utf8"));
const v = [];
const push = (name, val) => v.push(`  --${name}: ${val};`);

// surfaces
const baseMap = { void: "surface-void", surface: "surface", raised: "surface-raised", overlay: "surface-overlay", border: "border", borderStrong: "border-strong" };
for (const [k, name] of Object.entries(baseMap)) push(name, t.color.base[k]);
// text ramp
for (const k of ["hi", "base", "lo", "faint"]) push(`text-${k}`, t.color.text[k]);
// accents (seraph also exposed as the bare --accent)
for (const [name, a] of Object.entries(t.color.accent)) {
  if (name === "seraph") push("accent", a.value);
  push(`accent-${name}`, a.value);
}
// semantic
for (const [k, val] of Object.entries(t.color.semantic)) push(k, val);
// channel data colors
if (t.color.channel) for (const [k, val] of Object.entries(t.color.channel)) push(`channel-${k}`, val);
// fonts
push("font-ui", t.type.font.ui);
push("font-mono", t.type.font.mono);
// type scale
for (const [k, s] of Object.entries(t.type.scale)) { push(`fs-${k}`, s.size); push(`lh-${k}`, s.line); }
// space + radius
for (const [k, val] of Object.entries(t.space)) push(`space-${k}`, val);
for (const [k, val] of Object.entries(t.radius)) push(`radius-${k}`, val);

const ALIASES = `
  /* --- app-semantic aliases (existing var names -> Empyrean tokens) --- */
  --bg-app: var(--surface-void);
  --bg-panel: var(--surface);
  --bg-surface: var(--surface-raised);
  --bg-input: var(--surface-void);
  --border-focus: var(--accent);
  --text-primary: var(--text-hi);
  --text-secondary: var(--text-lo);
  --text-disabled: var(--text-faint);
  --accent-active: var(--accent);
  --accent-fm: var(--channel-fm);
  --accent-psg: var(--channel-psg);
  --accent-dac: var(--channel-dac);
  --knob-track: var(--surface-overlay);
  --knob-fill: var(--accent);
  --envelope-line: var(--accent);
  --envelope-fill: rgba(251, 191, 36, 0.15);
  --carrier-highlight: #FFCC44;`;

const css = `/* GENERATED by scripts/gen-theme.mjs from src/theme/tokens.json -- DO NOT EDIT BY HAND. */
:root {
${v.join("\n")}
${ALIASES}
}
`;

writeFileSync(OUT, css);

// self-check: required vars must be present
const required = ["--surface-void", "--surface", "--accent", "--text-hi", "--success", "--channel-fm", "--bg-app", "--accent-fm"];
const missing = required.filter((r) => !css.includes(`${r}:`));
if (missing.length) { console.error("gen-theme: MISSING vars:", missing); process.exit(1); }
console.log(`gen-theme: wrote src/theme/tokens.css (${v.length} tokens + aliases)`);
```

- [ ] **Step 2: Add the npm script**

In `package.json`, change the `"scripts"` block from:

```json
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  },
```

to:

```json
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "gen:theme": "node scripts/gen-theme.mjs"
  },
```

- [ ] **Step 3: Run the generator and verify it succeeds**

Run: `npm run gen:theme`
Expected: prints `gen-theme: wrote src/theme/tokens.css (NN tokens + aliases)`, exit 0.

- [ ] **Step 4: Verify key tokens emitted correctly**

Run: `grep -E -- "--accent:|--surface:|--channel-fm:|--bg-app:" src/theme/tokens.css`
Expected (order may vary):
```
  --surface: #12151E;
  --accent: #FBBF24;
  --channel-fm: #5EA8F5;
  --bg-app: var(--surface-void);
```

- [ ] **Step 5: Commit**

```bash
git add scripts/gen-theme.mjs package.json src/theme/tokens.css
git commit -m "feat(theme): add tokens.css generator and regenerate"
```

---

### Task 3: base.css + main.tsx wiring

The generator now owns `tokens.css` (`:root` only). The static rules that used to live at the bottom of the old `tokens.css` move into a hand-authored `base.css`, which also adds the amber focus ring.

**Files:**
- Create: `src/theme/base.css`
- Modify: `src/main.tsx`

- [ ] **Step 1: Create base.css**

```css
/* Static base styles. Variables come from tokens.css (generated). */
*, *::before, *::after { box-sizing: border-box; }

html, body {
  margin: 0;
  padding: 0;
  height: 100%;
  overflow: hidden;
  background: var(--bg-app);
  color: var(--text-primary);
  color-scheme: dark;
  font-family: var(--font-ui);
  font-size: var(--fs-base);
  line-height: var(--lh-base);
}

#root { height: 100%; }

button { font-family: inherit; font-size: inherit; cursor: pointer; }

input, select {
  font-family: inherit;
  font-size: inherit;
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}

select option { background: var(--bg-panel); color: var(--text-primary); }

/* amber focus ring (interaction accent) */
:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
```

- [ ] **Step 2: Wire imports in main.tsx**

Replace the entire contents of `src/main.tsx` with (font imports added in Task 4 — leave them out for now):

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./theme/tokens.css";
import "./theme/base.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

- [ ] **Step 3: Verify build**

Run: `npm run build`
Expected: succeeds (exit 0), no CSS/TS errors.

- [ ] **Step 4: Commit**

```bash
git add src/theme/base.css src/main.tsx
git commit -m "feat(theme): split base.css, wire generated tokens"
```

---

### Task 4: Self-hosted fonts (Inter + JetBrains Mono)

**Files:**
- Modify: `package.json` (deps — done by `npm install`)
- Modify: `src/main.tsx`

- [ ] **Step 1: Install @fontsource packages**

Run: `npm install @fontsource/inter @fontsource/jetbrains-mono`
Expected: both added to `dependencies`, exit 0.

- [ ] **Step 2: Import the needed weights in main.tsx**

Replace `src/main.tsx` contents with:

```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "./theme/tokens.css";
import "./theme/base.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
```

- [ ] **Step 3: Verify build**

Run: `npm run build`
Expected: succeeds; build output includes bundled woff2 assets.

- [ ] **Step 4: Commit**

```bash
git add package.json package-lock.json src/main.tsx
git commit -m "feat(theme): self-host Inter and JetBrains Mono via fontsource"
```

---

### Task 5: Seraph mark in the title bar

**Files:**
- Create: `src/assets/SeraphMark.tsx`
- Modify: `src/components/TopBar.tsx`
- Modify: `src/components/TopBar.module.css`

- [ ] **Step 1: Create the mark component**

```tsx
// src/assets/SeraphMark.tsx
// Seraph mark from megaforge/design/icons/seraph.svg. Uses currentColor — tint via `color`.
interface SeraphMarkProps {
  size?: number;
  className?: string;
}

export function SeraphMark({ size = 22, className }: SeraphMarkProps) {
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 96 96"
      role="img"
      aria-label="Seraph"
      fill="none"
      stroke="currentColor"
      strokeLinecap="round"
      strokeLinejoin="round"
    >
      <title>Seraph</title>
      <circle cx="48" cy="48" r="42" strokeWidth="1.4" />
      <circle cx="48" cy="48" r="38" strokeWidth="1" opacity="0.22" />
      <g strokeWidth="1.4">
        <path d="M48 30 Q26 22 18 40" />
        <path d="M48 30 Q70 22 78 40" />
        <path d="M48 46 Q22 44 14 60" />
        <path d="M48 46 Q74 44 82 60" />
        <path d="M48 62 Q30 66 26 82" />
        <path d="M48 62 Q66 66 70 82" />
      </g>
      <g strokeWidth="1" opacity="0.42">
        <line x1="48" y1="33" x2="30" y2="29" />
        <line x1="48" y1="33" x2="66" y2="29" />
        <line x1="48" y1="48" x2="26" y2="49" />
        <line x1="48" y1="48" x2="70" y2="49" />
      </g>
      <path d="M48 22 Q42 32 48 42 Q54 32 48 22 Z" strokeWidth="1.8" />
      <path d="M38 52 Q48 45 58 52 Q48 59 38 52 Z" strokeWidth="1.4" />
      <circle cx="48" cy="52" r="3.2" fill="currentColor" stroke="none" />
      <circle cx="28" cy="40" r="2.2" strokeWidth="1" opacity="0.42" />
      <circle cx="68" cy="40" r="2.2" strokeWidth="1" opacity="0.42" />
    </svg>
  );
}
```

- [ ] **Step 2: Render it in TopBar**

In `src/components/TopBar.tsx`, add the import after the existing imports (after line 5 `import styles from "./TopBar.module.css";`):

```tsx
import { SeraphMark } from "../assets/SeraphMark";
```

Then change the `projectInfo` block. From:

```tsx
      <div className={styles.projectInfo}>
        <span className={styles.projectName}>{projectMeta?.name ?? "Seraph"}</span>
```

to:

```tsx
      <div className={styles.projectInfo}>
        <SeraphMark className={styles.mark} />
        <span className={styles.projectName}>{projectMeta?.name ?? "Seraph"}</span>
```

- [ ] **Step 3: Style the mark**

In `src/components/TopBar.module.css`, add after the `.projectInfo` rule (after its closing `}` near line 18):

```css
.mark {
  color: var(--accent);
  flex-shrink: 0;
}
```

- [ ] **Step 4: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/assets/SeraphMark.tsx src/components/TopBar.tsx src/components/TopBar.module.css
git commit -m "feat(theme): add amber Seraph mark to title bar"
```

---

### Task 6: Bottom status bar

**Files:**
- Create: `src/components/StatusBar.tsx`
- Create: `src/components/StatusBar.module.css`
- Modify: `src/App.tsx`

- [ ] **Step 1: Create the component**

```tsx
// src/components/StatusBar.tsx
import styles from "./StatusBar.module.css";

export function StatusBar() {
  return (
    <div className={styles.statusBar}>
      <span className={styles.aether}>
        <span className={styles.diamond}>◇</span> Aether offline
      </span>
    </div>
  );
}
```

- [ ] **Step 2: Create the styles**

```css
/* src/components/StatusBar.module.css */
.statusBar {
  height: 22px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  padding: 0 12px;
  background: var(--surface);
  border-top: 1px solid var(--border);
  font-size: var(--fs-xs);
  color: var(--text-lo);
  font-family: var(--font-mono);
}

.aether {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.diamond {
  color: var(--text-faint);
}
```

- [ ] **Step 3: Mount it in App.tsx**

In `src/App.tsx`, add the import after line 7 (`import { BottomPanel } ...`):

```tsx
import { StatusBar } from "./components/StatusBar";
```

Then insert `<StatusBar />` as a flex child immediately after the BottomPanel block. Change:

```tsx
      {projectOpen && (
        <BottomPanel
          selectedInstrument={selectedInstrument}
          selectedRegion={selectedRegions[selectedRegions.length - 1] ?? null}
          onCloseRegion={() => setSelectedRegions([])}
          playing={playing}
          projectMeta={projectMeta!}
        />
      )}
```

to:

```tsx
      {projectOpen && (
        <BottomPanel
          selectedInstrument={selectedInstrument}
          selectedRegion={selectedRegions[selectedRegions.length - 1] ?? null}
          onCloseRegion={() => setSelectedRegions([])}
          playing={playing}
          projectMeta={projectMeta!}
        />
      )}
      <StatusBar />
```

- [ ] **Step 4: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 5: Commit**

```bash
git add src/components/StatusBar.tsx src/components/StatusBar.module.css src/App.tsx
git commit -m "feat(theme): add Aether status bar (offline placeholder)"
```

---

### Task 7: Apply amber accent to interaction states

Switch the active-tab underline, the resize-handle/slider accents, and the app-level primary action buttons from the old blue (`--accent-fm`) to amber (`--accent`), with dark text for contrast. Channel-specific editor buttons (PSG/DAC/FM) keep their data colors — they are handled in Task 8.

**Files:**
- Modify: `src/components/Sidebar.module.css:31`
- Modify: `src/components/BottomPanel.module.css:22`
- Modify: `src/components/TopBar.module.css:113`
- Modify: `src/components/TransportControls.module.css:25-26`
- Modify: `src/components/InstrumentBrowser.module.css:186-187`
- Modify: `src/components/AddTrackDialog.module.css:62-63`
- Modify: `src/components/NewProjectDialog.module.css:142-143`

- [ ] **Step 1: Active-tab underline → amber**

`src/components/Sidebar.module.css` — in `.tab.active`, change `border-bottom-color: var(--accent-fm);` to:

```css
  border-bottom-color: var(--accent);
```

- [ ] **Step 2: Resize handle + master-volume slider → amber**

`src/components/BottomPanel.module.css` — in `.resizeHandle:hover`, change `background: var(--accent, #4488ff);` to:

```css
  background: var(--accent);
```

`src/components/TopBar.module.css` — in `.masterVolSlider`, change `accent-color: var(--accent, #4a9eff);` to:

```css
  accent-color: var(--accent);
```

- [ ] **Step 3: Transport active button → amber**

`src/components/TransportControls.module.css` — in `.btn.active`, change:

```css
  background: var(--accent-fm);
  color: #fff;
```
to:
```css
  background: var(--accent);
  color: var(--surface-void);
```

- [ ] **Step 4: Dialog primary buttons → amber**

`src/components/InstrumentBrowser.module.css` — in `.renameOkBtn`, change:
```css
  background: var(--accent-fm);
  color: #fff;
```
to:
```css
  background: var(--accent);
  color: var(--surface-void);
```

`src/components/AddTrackDialog.module.css` — in `.createBtn`, change:
```css
  background: var(--accent-fm);
  color: #fff;
```
to:
```css
  background: var(--accent);
  color: var(--surface-void);
```

`src/components/NewProjectDialog.module.css` — in `.createBtn`, change:
```css
  background: var(--accent-fm);
  color: #fff;
```
to:
```css
  background: var(--accent);
  color: var(--surface-void);
```

- [ ] **Step 5: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 6: Commit**

```bash
git add src/components/Sidebar.module.css src/components/BottomPanel.module.css src/components/TopBar.module.css src/components/TransportControls.module.css src/components/InstrumentBrowser.module.css src/components/AddTrackDialog.module.css src/components/NewProjectDialog.module.css
git commit -m "feat(theme): amber accent for tabs, focus, primary actions"
```

---

### Task 8: Migrate remaining hardcoded hex to tokens

Replace the remaining literal hex colors with tokens so nothing bypasses the theme. Each replacement below is exact (file → rule → change).

**Files:** `App.module.css`, `widgets/PianoKeys.module.css`, `TrackList.module.css`, `PsgEditor.module.css`, `DacEditor.module.css`, `FmEditor.module.css`, `TrackHeader.module.css`.

- [ ] **Step 1: App.module.css notification toasts**

In `src/App.module.css`:

- `.exportSuccess`: `background: #2a5a2a;` → `background: var(--surface-raised);`; `color: #88ff88;` → `color: var(--success);`
- `.exportError`: `background: #3a1a1a;` → `background: var(--surface-raised);`; `border: 1px solid #ff4444;` → `border: 1px solid var(--error);`; `color: #ffaaaa;` → `color: var(--text-base);`
- `.exportErrorHeader button`: `color: #ffaaaa;` → `color: var(--error);`
- `.importWarning`: `background: #2a2a1a;` → `background: var(--surface-raised);`; `border: 1px solid #ccaa44;` → `border: 1px solid var(--warning);`; `color: #eedd88;` → `color: var(--text-base);`
- `.importWarningHeader button`: `color: #eedd88;` → `color: var(--warning);`

- [ ] **Step 2: PianoKeys.module.css**

In `src/widgets/PianoKeys.module.css`:

- `.whiteKey` `background: #eee;` → `background: var(--text-hi);`
- `.whiteKey` `border: 1px solid #999;` → `border: 1px solid var(--text-lo);`
- (line 52) `background: #ddd;` → `background: var(--text-base);`
- `.blackKey` `background: #222;` → `background: var(--surface-raised);`
- `.blackKey` `border: 1px solid #111;` → `border: 1px solid var(--surface-void);`
- (line 72) `background: #444;` → `background: var(--border-strong);`

- [ ] **Step 3: Channel chip / badge dark text (keep channel/data bg)**

- `src/components/TrackList.module.css` lines 27–29: change each `color: #000;` → `color: var(--surface-void);` (the `.fm`/`.psg`/`.dac` chips keep their `var(--accent-*)` backgrounds).
- `src/components/PsgEditor.module.css` `.noiseBtn.activeNoise` (line 101) `color: #000;` → `color: var(--surface-void);` (keeps `background: var(--accent-psg);`).
- `src/components/PsgEditor.module.css` line 128 `color: #000;` → `color: var(--surface-void);`
- `src/components/DacEditor.module.css` `.previewBtn` (line 82) `color: #000;` → `color: var(--surface-void);` (keeps `background: var(--accent-dac);`).
- `src/components/FmEditor.module.css` `.carrierBadge` (line 50) `color: #000;` → `color: var(--surface-void);` (keeps `background: var(--carrier-highlight);`).

- [ ] **Step 4: TrackHeader.module.css**

In `src/components/TrackHeader.module.css`:

- `.badge` (line 60) `color: #fff;` → `color: var(--text-hi);`
- `.muteBtn.active` (line 96) `color: #fff;` → `color: var(--text-hi);` (keeps `background: var(--error);`)
- `.soloBtn.active` (line 102) `color: #000;` → `color: var(--surface-void);` (keeps `background: var(--carrier-highlight);`)
- `.meter` (line 154) `background: #1a1a1a;` → `background: var(--surface-void);`

- [ ] **Step 5: Verify no hardcoded hex remains in modules**

Run: `grep -rnE "#[0-9a-fA-F]{3,8}" src --include="*.module.css"`
Expected: no output (empty). If any line prints, fix it with the appropriate token before continuing.

- [ ] **Step 6: Verify build**

Run: `npm run build`
Expected: succeeds.

- [ ] **Step 7: Commit**

```bash
git add src/App.module.css src/widgets/PianoKeys.module.css src/components/TrackList.module.css src/components/PsgEditor.module.css src/components/DacEditor.module.css src/components/FmEditor.module.css src/components/TrackHeader.module.css
git commit -m "refactor(theme): migrate remaining hardcoded hex to tokens"
```

---

### Task 9: Final verification

- [ ] **Step 1: Regenerate theme is idempotent**

Run: `npm run gen:theme && git status --short src/theme/tokens.css`
Expected: generator exits 0; `tokens.css` shows no diff (already up to date).

- [ ] **Step 2: No stray hex in modules**

Run: `grep -rnE "#[0-9a-fA-F]{3,8}" src --include="*.module.css"`
Expected: empty.

- [ ] **Step 3: Production build**

Run: `npm run build`
Expected: exit 0.

- [ ] **Step 4: Visual smoke test**

Run: `npm run tauri dev`
Confirm by eye:
- Deep-space dark surfaces (near-black `#0A0C12` app bg, `#12151E` panels).
- Amber active tab underline, amber focus ring on keyboard focus, amber primary buttons (New Project / Add Track / Save-rename) with dark text.
- Inter for UI text, JetBrains Mono in the status bar.
- The amber Seraph mark in the top bar.
- Bottom status bar reading `◇ Aether offline`.
- FM/PSG/DAC track chips still blue/green/orange (retuned), readable on the dark panels.

- [ ] **Step 5: Final commit (if any tweaks from smoke test)**

```bash
git add -A
git commit -m "fix(theme): visual smoke-test adjustments"
```

(Skip if the smoke test required no changes.)

---

## Self-Review notes (coverage vs. spec)

- Spec §1 token pipeline → Tasks 1, 2, 9.
- Spec §2 tokens.css content + alias layer → Task 2 (generator emits raw tokens + aliases).
- Spec §3 fonts self-hosted → Task 4 (refinement: `@fontsource` instead of manual woff2/@font-face — same self-hosted result, more robust).
- Spec §4 amber accent → Tasks 3 (focus ring) + 7 (tabs, slider, primary actions).
- Spec §5 title-bar mark → Task 5.
- Spec §6 status bar → Task 6.
- Spec §7 migrate hardcoded hex → Task 8.
- base.css split → Task 3.

**Deviation from spec:** fonts use `@fontsource` packages rather than hand-vendored woff2 in `src/assets/fonts/` + a hand-written `fonts.css`. This still self-hosts both fonts (files bundled by Vite from the packages) and is the idiomatic, lower-maintenance approach. No `src/theme/fonts.css` or `src/assets/fonts/` is created.
