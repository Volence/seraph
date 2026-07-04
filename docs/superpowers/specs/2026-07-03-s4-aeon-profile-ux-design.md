# S4 — Aeon-Profile Authoring UX — Design

**Date:** 2026-07-03 · **Status:** BANKED (strata approved in the master design
2026-07-03; this spec pins components + interactions). **Depends:** S1 (model),
S3 strongly preferred (preview truth). Feature-gated items follow manifest flags.

Governing principle (master design): **correct-by-construction first, lint
second**; illegal states unrepresentable; export never fails with an error list.

## Components by stratum

### 1. Channel rack (track headers)

- **`ChannelRack`** replaces the flat track list when profile==memra: fixed
  hardware slots FM1–6, PSG1–3, NOISE, DAC (manifest `channels`). Tracks BIND
  to slots; empty slots render dim. Steal badges: FM1/FM2 `protected`, others
  `stealable` (tooltip explains SFX stealing; from manifest `steal`).
- **`Fm6ModeControl`** (in SongSettings + inline on the FM6/DAC slot pair):
  three-state dedicate/full-FM6/time-share. Dedicate → FM6 note lane disabled
  (visually dimmed, drops rejected with a toast naming the mode); full-FM6 →
  DAC lane disabled; time-share → both enabled, and the FM6 region view draws
  **duck notches** (computed from DAC lane note starts + sample lengths from
  the DAC manager) under each drum hit.
- **`Fm3SpecialToggle`** (manifest flag `memra.fm3.optracks`; greyed w/ tooltip
  while `reserved`): switches the FM3 slot between one normal lane and a
  4-lane op-pitch view (shared patch selector spans the 4 lanes; per-lane
  TL-as-volume fader). Plain-FM3 regions and op-track regions cannot coexist:
  toggling with existing regions prompts convert-or-cancel.
- **`NoisePitchSource`** on the NOISE slot: `fixed` | `psg3-coupled` (rate-3).
  Coupled: PSG3 slot shows a link glyph + its note lane disabled for the
  coupled range; noise notes carry pitch (driving PSG3's divisor). Uncoupling
  restores PSG3.
- **Sub-voice lanes:** any FM/PSG track: "+ lane" adds an instrument lane.
  The piano roll paints all lanes of a track in one grid (color per lane);
  note placement that would overlap ANOTHER lane's note is blocked with a
  ghost-preview snap-to-nearest-free (model invariant from S1 enforces too).
  A lane switch at a note boundary is drawn as a patch chip on the note.
- **`GhostSendIndicator`:** when the Echo/Unison plugin (stratum 3) is active,
  its reserved spare FM slot renders as "GHOST (of FMn)" and refuses tracks.

### 2. Piano roll note properties

- **`NoteInspector`** panel (selection-scoped) + on-note handles:
  - Gate (`NOTEFILL`): drag the note's sounding-length sub-bar (tracker-style);
    numeric frames field. FM tracks only (hidden otherwise, per manifest).
  - Pitch envelope: 1–5 step mini-editor on the note (validates point range
    from manifest); renders as a zigzag glyph.
  - **Morph vs Swap:** dragging a DIFFERENT instrument onto a note boundary =
    swap (Patch chip); opening the note's "morph" tab = draw per-parameter
    curves (op TL, D1R, …) inside the note → compiled to minimal REGDELTA
    steps at a chosen resolution (default: 1 step/tick, coalesced); the UI
    labels it "morphs, never re-attacks."
  - Porta (rate + on/off; lint badge if no prior note on the lane), detune,
    vibrato (wait/speed/depth/steps) — per-note overrides of lane defaults.
  - Raw-frequency note: pitch field accepts `raw:$A4,$A0` entry mode (FM only).
- **Velocity lane** maps to `Vol` events (linear 0–127, manifest range).

### 3. Plugin rack

- **`PluginRack`** per track + a **`GlobalRack`**; registry driven by manifest
  `features` (+ a static registry of Tier-0 offline plugins). Each plugin:
  id, feature flag, params (ranges from manifest/limits), compile fn, and
  `status` chip (greyed when flag ≠ shipped, tooltip: "engine package N").
  - Runtime: **Sidechain Pump** (trigger sample picker + depth 0..$7F →
    `MEV_EXT PUMPSET`), **Autopan** (rate → TAG_MAC_PAN loop), **Echo** /
    **Unison** (two UI plugins, one GHOSTSET mechanism; params delay ticks /
    level drop / detune / pan mode; selecting a spare FM channel is part of
    the plugin UI and reserves the slot), **Automation** (generic macro-lane
    editor → slot[1] stream with the NEXT-before-LOOP rule enforced
    structurally: the editor emits frames, not raw tags).
  - Global: **LFO** (single global unit; per-track AMS/FMS depth knobs live on
    the TRACK (pan inspector), the rack shows one enable+rate control with a
    wiring diagram making the globality visible), **Tempo automation**
    (MEV_TEMPO points on a global lane; BPM readout).
  - Offline (Tier 0): **Drum Mastering** (per-sample chain editor; seed field;
    renders at export + into S3 preview), **Humanize/Ghost/Flam** (seeded;
    flam pairs trigger composite-sample baking via the DAC manager), **Filter
    Env** (per-FM-patch cutoff envelope → modulator-TL macro), **PSG Sub-Bass**
    helper (arms rate-3 pattern under a chosen FM bass lane).
- Compile order + ownership: each plugin owns disjoint event kinds; the
  compiler rejects (rule id) if two plugins claim the same stream slot (e.g.
  Autopan + a manual pan automation lane on the same track).

### 4. Song declarations & DAC manager

- **`SongSettingsPanel`:** kind (Song/Jingle/SFX — S6 owns the SFX form),
  looped toggle (loop marker appears on the timeline when on), tempo_mod
  (slider + BPM readout), FM6 mode, patch bank, pitch-table override (file
  picker, advanced).
- **`MarkerLane`:** named markers → `MEV_EXT COMM` values (feature
  `memra.comm`; greyed until shipped); marker id map exported alongside the
  song for game-side use.
- **`DacManager`:** sample list w/ import (any WAV → resample to engine rate →
  optional mastering chain → encode preview), constraints enforced at import
  (<32 KB post-encode, descriptor fields), composite baker (pick 2 samples +
  offset → new baked sample), slot/id assignment synced with the engine
  tables at export.
- **Budget meter** (transport bar): per-frame event-cost estimate from
  manifest write-pacing facts (patch loads/frame=1; REGDELTA+macro additive);
  yellow = heuristic exceeded, red = S3 preview reported a real overrun.

## Lints (the non-structural residue)

Small persistent panel (never a modal, never blocks): porta-without-prior-note,
morph curve busier than budget, jingle-class violations on kind=Jingle,
COMM markers present while feature reserved. Each cites the manifest rule id.

## Out of scope

SFX workshop UI (S6); importer UI (S5); any new engine features (manifest
flags gate everything; nothing here requires engine work to LOOK correct —
reserved features are visibly parked).

## Verification

Component tests (vitest/RTL) for: slot binding + mode exclusion logic,
lane-monophony interaction, plugin registry gating from a manifest fixture
with flags toggled, morph→REGDELTA compile snapshot, duck-notch computation.
E2E smoke (tauri dev): build a 4-track score using every stratum, export,
parity-check (S1 harness), and — once S3 lands — audible preview.
