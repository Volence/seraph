# Seraph — Empyrean Theme Adoption (Design)

*Date: 2026-06-18*

## Context

Seraph (the DAW, repo `seraph`, formerly `megadaw`) is one tool in the **Empyrean**
suite. The suite owns a shared visual contract in the sibling repo
`/home/volence/sonic_hacks/megaforge`:

- `design/tokens.json` — machine-readable source of truth (surfaces, text ramp, the six
  per-tool accents, semantic colors, type scale, spacing, radius, chrome conventions).
- `design/README.md` — the visual contract narrative.
- `design/icons/seraph.svg` — Seraph's mark (six-winged seraph, `currentColor`, `0 0 96 96`).

**Governing principle (from the contract):** *eldritch identity, serene workspace.* Awe at
the edges (mark, accent glow, empty states); calm, dense, readable working surfaces.

**Seraph's accent:** amber `#FBBF24`.

### Current state of the app

- `src/theme/tokens.css` already exists and defines a **custom, non-Empyrean** palette
  (`--bg-app`, `--bg-panel`, `--text-primary`, `--accent-fm/psg/dac`, …). It is imported once
  in `src/main.tsx`.
- ~218 `var(--…)` usages across CSS modules already flow through these variables.
- 13 `*.module.css` files still contain hardcoded hex colors that bypass the variables:
  App, InstrumentBrowser, TrackList, BottomPanel, TrackHeader, AddTrackDialog, TopBar,
  PsgEditor, PianoKeys, TransportControls, FmEditor, NewProjectDialog, DacEditor.
- Fonts are system fonts, not Inter / JetBrains Mono.
- The app uses **three** instrument-type accent colors (FM blue / PSG green / DAC orange),
  not Empyrean's single-accent model.

## Decisions

1. **Accent model — amber for interaction, keep data colors.** Amber `#FBBF24` drives the
   active-tab underline, the focus ring, primary buttons, and the title-bar mark. FM/PSG/DAC
   remain *functional data-category colors* (waveforms, track headers, meters) but are retuned
   to sit cleanly on the deep-space palette. Clean split: brand/interaction vs. data encoding.

2. **Token sync — vendored copy + generator.** A copy of `tokens.json` lives in the repo at
   `src/theme/tokens.json` so the app builds standalone (no dependency on the `../megaforge`
   path). A small Node script `scripts/gen-theme.mjs` reads that JSON and emits
   `src/theme/tokens.css`. Re-sync = copy the new JSON + `npm run gen:theme`.

3. **Fonts — self-hosted.** Inter and JetBrains Mono are vendored as woff2 into
   `src/assets/fonts/` and declared via `@font-face`, with the contract's system fallbacks
   retained in the font stack.

4. **Scope — visual theme only.** Explicitly *out of scope* for this chunk:
   - The **Ctrl-K command palette** (a real feature; separate work).
   - A **live Aether connection** (the bus phase; Seraph joins the bus last by design).
   - The **simplified ≤24px icon tier** (a contract follow-up, still TBD upstream).

## Architecture

### Token pipeline

```
src/theme/tokens.json     vendored copy of megaforge/design/tokens.json (source of truth)
        │  (npm run gen:theme)
        ▼
scripts/gen-theme.mjs      reads json → writes css; idempotent, no network
        │
        ▼
src/theme/tokens.css       GENERATED — header comment: "do not edit by hand"
        │  (import "./theme/tokens.css" in main.tsx — already present)
        ▼
all CSS modules            consume var(--…)
```

`gen-theme.mjs` responsibilities:
- Read and parse `src/theme/tokens.json`.
- Emit a `:root { … }` block of CSS custom properties under three groups:
  1. **Raw Empyrean tokens** (canonical names), e.g.
     `--surface-void`, `--surface`, `--surface-raised`, `--surface-overlay`,
     `--border`, `--border-strong`,
     `--text-hi`, `--text`, `--text-lo`, `--text-faint`,
     `--accent` (= seraph amber `#FBBF24`), plus the other suite accents for reference,
     `--success`, `--warning`, `--error`, `--info`,
     type scale (`--font-ui`, `--font-mono`, `--text-xs … --text-2xl` size+line),
     spacing (`--space-0 … --space-9`), radius (`--radius-sm … --radius-pill`).
  2. **App-semantic alias layer** mapping the existing variable names onto the new tokens so
     current usages keep working without edits. Mapping (final values tuned in implementation):
     - `--bg-app: var(--surface-void)`
     - `--bg-panel: var(--surface)`
     - `--bg-surface: var(--surface-raised)`
     - `--bg-input: var(--surface-void)` (inputs read as recessed)
     - `--border: var(--border)` / `--border-focus: var(--accent)`
     - `--text-primary: var(--text-hi)`
     - `--text-secondary: var(--text-lo)`
     - `--text-disabled: var(--text-faint)`
     - `--accent-active: var(--accent)`
     - `--error: var(--error)` / `--success: var(--success)`
     - `--accent-fm / --accent-psg / --accent-dac`: retuned blue/green/orange (data colors).
       Sourced as new entries (either literal retuned hex in the generator, or — preferred —
       added to the vendored `tokens.json` under a `data`/`channel` group so they too trace to
       a token file).
  3. The non-token global rules currently at the bottom of `tokens.css` (box-sizing reset,
     `html/body`, `button`, `input/select`) are preserved — either kept in a separate
     hand-authored `src/theme/base.css` imported alongside, or appended verbatim by the
     generator. **Decision:** split static base rules into `src/theme/base.css` (hand-authored,
     imports nothing token-specific beyond vars) so the generated file is *purely* `:root`
     variables. `main.tsx` imports both.

### Channel data colors

FM/PSG/DAC retuned to harmonize with the deep-space base while staying clearly distinct.
Added to the vendored `tokens.json` (e.g. a `color.channel` group) so they are generated, not
hardcoded in the script. Exact hex chosen in implementation against the new surfaces; intent:
blue / green / orange, slightly desaturated and lightened to read on `#12151E`.

### Fonts

- `src/assets/fonts/` holds woff2 files: Inter (regular 400, medium 500, semibold 600) and
  JetBrains Mono (regular 400, medium 500).
- `src/theme/fonts.css` (hand-authored) declares `@font-face` rules; imported in `main.tsx`.
- Font stacks come from tokens: `--font-ui` and `--font-mono` (already specified in
  `tokens.json` with fallbacks).

### Amber accent application

Targeted edits (not a sweep):
- **Focus ring:** a global `:focus-visible` rule in `base.css` using `--accent`.
- **Active tab underline:** wherever tab strips exist (editor panel tabs, bottom panel tabs) —
  active state uses an amber bottom-border / underline.
- **Primary buttons:** the primary action in dialogs (New Project, Add Track, Import) and the
  welcome screen actions use amber; secondary buttons stay neutral surface.

### Title-bar mark

- Inlined as a small React component `src/assets/SeraphMark.tsx` (cleanest for `currentColor`
  tinting + sizing; no asset-import config).
- Rendered in `TopBar` at ~20–24px, `color: var(--accent)` (the SVG uses `currentColor`).
- Note: the ≤24px simplified tier is upstream-TBD; the full mark is acceptable at ~24px per the
  contract (fine linework reads to ~32px). We render it at the largest size that fits the bar.

### Bottom status bar

- New `src/components/StatusBar.tsx` + `StatusBar.module.css`, mounted at the bottom of the app
  shell in `App.tsx`.
- Shows `Aether ◇ offline` (static placeholder; the diamond + label become "connected" in the
  bus phase). May also show small contextual info (e.g. selected driver) — kept minimal.

## Components / files touched

| File | Change |
|---|---|
| `src/theme/tokens.json` | **new** — vendored copy of contract tokens (+ channel colors) |
| `scripts/gen-theme.mjs` | **new** — generator json → css |
| `package.json` | **new script** `gen:theme` |
| `src/theme/tokens.css` | **regenerated** — Empyrean tokens + alias layer (`:root` only) |
| `src/theme/base.css` | **new** — static global rules (reset, html/body, button, inputs, focus ring) |
| `src/theme/fonts.css` | **new** — `@font-face` declarations |
| `src/assets/fonts/*.woff2` | **new** — Inter + JetBrains Mono |
| `src/assets/SeraphMark.tsx` | **new** — the mark, inlined for `currentColor` |
| `src/main.tsx` | import `fonts.css`, `tokens.css`, `base.css` |
| `src/components/TopBar.tsx` (+ css) | add amber-tinted mark |
| `src/components/StatusBar.tsx` (+ css) | **new** — `Aether ◇ offline` |
| `src/App.tsx` (+ css) | mount StatusBar in shell |
| 13 `*.module.css` files | replace hardcoded hex with tokens |
| tab strips / primary buttons / focus | apply amber accent |

## Testing / verification

This is a visual/CSS change with one small build script. Verification approach:
- **Generator:** `npm run gen:theme` runs clean and produces a `tokens.css` whose `:root`
  contains every expected variable (spot-check key tokens: `--accent` = `#FBBF24`,
  `--surface` = `#12151E`). A lightweight assertion in the script (or a follow-up check) that
  all `color.*` keys were emitted.
- **No hardcoded hex left:** `grep -r "#[0-9a-fA-F]\{3,6\}" src --include="*.module.css"`
  returns only intentional exceptions (documented), ideally empty.
- **Build:** `npm run build` (frontend) succeeds.
- **Visual smoke:** run the app (`npm run tauri dev` or the project's run path) and confirm:
  deep-space surfaces, amber active tab / focus ring / primary button, Inter/JetBrains Mono
  rendering, the mark in the title bar, the status bar reading `Aether ◇ offline`.

## Out of scope (future chunks)

- Ctrl-K command palette (feature).
- Live Aether bus connection + the SFX-as-data contract (bus phase).
- Simplified ≤24px icon tier (upstream contract follow-up).
