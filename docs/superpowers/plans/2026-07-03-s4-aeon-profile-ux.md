# S4 — Aeon-Profile Authoring UX Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Checkbox steps. Repo: seraph, branch `feat/s4-aeon-ux`. Depends: S1 executed (S3 strongly preferred first — the budget meter + audition steps degrade gracefully without it). Read the spec (`../specs/2026-07-03-s4-aeon-profile-ux-design.md`) + author-surface inventory before each phase; every constraint value comes FROM the loaded manifest, never hardcoded.

**Goal:** The four UX strata: channel rack, note gestures, plugin rack, song declarations — correct-by-construction.

Phases = tasks; each ends with vitest/RTL component tests green, `npm run build` green, commit. UI work follows existing component patterns (`src/components/*.tsx`, CSS Modules, Canvas piano roll).

### Task 1: Channel rack
Files: create `src/components/ChannelRack.tsx` (+module.css); modify `src/components/AddTrackDialog.tsx` (slot binding), arrangement view (slot rows).
- [ ] Fixed slots from manifest `channels`; steal badges; track↔slot binding replaces free channel dropdown when profile==memra.
- [ ] `Fm6ModeControl` + lane disable/dim logic + duck notches (compute from DAC lane notes + sample lengths via IPC `get_dac_sample_info`); rejection toasts name the mode.
- [ ] `NoisePitchSource` + PSG3 link/disable behavior.
- [ ] Tests: mode exclusion matrix (3 FM6 modes × drop attempts), coupled-noise disables PSG3. Commit.

### Task 2: Sub-voice lanes + piano roll gestures
Files: piano roll components (find via `src/components/` — the Canvas roll), create `src/components/NoteInspector.tsx`; model IPC already carries fields (S1).
- [ ] Multi-lane paint (color per lane) + cross-lane monophony block with snap-to-free ghost preview; patch chips at lane switches.
- [ ] NoteInspector: gate sub-bar (FM-only per manifest), pitch-env mini editor (1–5 pts), porta/detune/vibrato fields, raw-freq entry, velocity lane→Vol.
- [ ] Morph tab: per-parameter curve editor inside a note; compile preview shows resulting REGDELTA count (soft-warn vs budget heuristic).
- [ ] Tests: monophony interaction, gate hidden on PSG tracks, morph→REGDELTA compile snapshot (fixture note + curve → expected event JSON via interchange format). Commit.

### Task 3: Plugin rack
Files: create `src/components/PluginRack.tsx`, `src/plugins/registry.ts`, one module per plugin under `src/plugins/` (pump, autopan, echo, unison, automation, lfoGlobal, tempoLane, drumMastering, humanize, filterEnv, psgSubBass); Rust compile hooks in `src-tauri/src/compiler/mev/plugins.rs` (new).
- [ ] Registry keyed by manifest feature flags; greyed+tooltip when reserved; param ranges from manifest. Compile ownership check (two plugins claiming a stream slot → RuleViolation surfaced in the lint panel).
- [ ] Runtime plugins first (pump/autopan/echo/unison/automation) — compile to PUMPSET/TAG_MAC_PAN/GHOSTSET/slot[1] via S1 event enum. Echo/Unison reserve a spare FM slot (rack shows GHOST state).
- [ ] Global rack: LFO single-unit control + per-track AMS/FMS knobs on the track pan inspector; tempo lane → MEV_TEMPO points.
- [ ] Offline plugins: UI + param persistence + export-time invocation stubs calling the aeon tool equivalents ported into Rust OR shelling to nothing — DECISION pinned: implement mastering/humanize/filter-env as Rust ports inside Seraph (aeon's Python tools remain the build-side authority; Seraph's ports are for preview/export parity and are corpus-tested against the Python outputs when python3 present, same skip rule as the S1 parity harness).
- [ ] Tests: registry gating fixture (flags on/off), pump param→event bytes, ownership conflict. Commit.

### Task 4: Song declarations + DAC manager + lint panel
Files: create `src/components/SongSettingsPanel.tsx`, `src/components/MarkerLane.tsx`, `src/components/DacManager.tsx`, `src/components/LintPanel.tsx`; Rust: `src-tauri/src/dac/import.rs` extensions (resample/encode preview, composite baker).
- [ ] SongSettings (kind/loop/tempo+BPM readout/FM6/patch bank/pitch table); markers→COMM (gated); loop marker on timeline.
- [ ] DacManager: import→resample→(optional chain)→size/descriptor checks; composite baker; id sync at export.
- [ ] LintPanel: persistent, rule-id-cited entries (porta-seed, morph budget, jingle class, reserved-feature use); budget meter in transport (heuristic tier; S3 wires the real tier).
- [ ] Tests: jingle-kind lints fire on violations; DAC import rejects >32 KB post-encode. Commit.

### Task 5: E2E + closeout
- [ ] Scripted E2E (tauri dev): build the spec's 4-track everything-score via UI-level drivers where feasible, export MEV, run the S1 parity check.
- [ ] Merge → main; queue doc S4 → DONE (+log any deferrals, e.g. offline-plugin corpus gaps). Commit.
