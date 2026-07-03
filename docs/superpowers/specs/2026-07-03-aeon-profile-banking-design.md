# Seraph Aeon-Profile Banking — Master Design

**Date:** 2026-07-03
**Status:** APPROVED (user, 2026-07-03) — this is the master design for the Seraph
banking queue; each package below still gets its own spec (where marked) + cold-
executable plan, following the pattern of `aeon/docs/superpowers/2026-07-03-sound-banking-queue.md`.
**Companion:** `../2026-07-03-memra-author-surface-inventory.md` — the exhaustive,
spec-cited inventory of every author-facing driver feature and constraint. Every
package spec session MUST read it; it is the grounding that keeps lower-model
execution honest.

## What this is

Seraph (the Genesis DAW, ex-MegaDAW) becomes the faithful authoring frontend for
the Aeon engine's sound driver — named **Memra** as of this design (see Naming).
The promise: **music or SFX authored in Seraph's Aeon profile WILL play correctly
in-engine**, and the driver's esoteric capabilities (FM3 special mode, FM6/DAC
arbitration, ghost/echo voice, mid-note voice morphs, tone-coupled noise, …) are
usable through modern-DAW idioms without reading a single engine spec.

This mirrors the 2026-07-03 sound design-banking session: bank spec + plan per
package now (while high-model access is available) so any future session can
execute cold.

## Decisions made (user, 2026-07-03)

1. **Guarantee = driver-in-the-loop.** Seraph's end-state preview embeds a Z80
   core running the REAL Memra driver blob against its emulated YM2612 + SN76489;
   "play" means compile-to-MEV and play that. Staged: exporter + oracle A/B gate
   ships first (S2), embedded driver second (S3). The guarantee is structural,
   not aspirational — and Z80 budget overruns surface while composing.
2. **Aeon-native data model, wide import funnel, narrow export.** The project
   model is redesigned around the Memra capability manifest (the superset).
   Importers (SMPS, VGM, Zyrinx; GEMS as a new addition — it is a Z80-resident
   driver with a documented format) all convert INTO the native model. Export
   targets: Aeon/Memra (primary, lossless) and SMPS (retained as a clearly-marked
   LOSSY down-conversion for legacy projects). Nothing round-trips losslessly
   except Aeon itself. VGM import stays (VGM → author → export-as-Aeon workflow).
3. **The driver is named Memra** (Aramaic: "the Word" — the divine voice as a
   standalone agent that speaks on behalf of what cannot speak directly; the
   68k never touches the chips, Memra speaks for it from the Z80). **Docs /
   contract / UI level ONLY** — no symbol renames. `MEV_*` is retconned as
   "Memra EVent." `SND_*`, `Seq_*` etc. stay as-is. Zero code churn.
4. **Plugin framing for effects-shaped features** (user instinct, formalized in
   S4): features that behave like DAW inserts/sends/offline renders are
   manifest-declared plugins; features that change what a track IS are track
   types/modes enforced by the timeline itself.
5. **Three project types: Song / Jingle / SFX** — same instruments, piano roll,
   and compiler; different header metadata, channel rules, and workflow (S6).
6. **Approach A (full queue) chosen** over minimal banking: UI/UX design is
   exactly where high-model design effort matters most; a lower model can
   execute a pinned UX spec far better than it can invent one.

## Architecture overview

```
empyrean/  — Memra capability manifest (machine-readable contract)
             generated-from / build-validated-against aeon's
             sound_constants.asm + tools/song_packer.py  → drift impossible
aeon/      — the Memra driver itself (source of truth); engine packages 1-6
             banked 2026-07-03, not yet executed (specs are normative)
seraph/    — consumes the manifest: data model, compiler, plugin registry,
             UI enable/disable flags all derive from it
```

**Correct-by-construction first, lint second (governing S4 principle).** Memra
is trust-the-packer: NOTHING downstream validates; an invalid blob hangs or
corrupts the Z80 silently. Therefore Seraph makes illegal states unrepresentable
wherever possible (route-illegal events can't be placed; channel monophony is
enforced by the piano roll; mode exclusions grey the affected lanes), and live-
lints only what can't be structural (porta needs a prior note; repeat bodies
must advance time). Export never fails with an error list — the project is
valid continuously while composing. The full normative rule list (the "exporter
contract", package-0 validity rules) is in the companion inventory and in
`aeon/docs/superpowers/specs/2026-06-23-music-expression-engine-design.md`.

**Budget-gated features are manifest flags.** Engine package 5 Tier 2 (ghost
voice, ExtCh3 op-tracks) may never ship (Z80 budget gates). In Seraph each is a
plugin/track-mode whose manifest flag is off until the engine lands it: greyed
with a tooltip, never a trap.

## Package queue (execution order)

| # | Package | Deliverables | Depends on |
|---|---------|-------------|------------|
| S0 | **Memra contract** — capability manifest in empyrean (machine-readable; generated-from/validated-against `sound_constants.asm` + `song_packer.py`); Memra naming pass (docs/UI); feature flags for budget-gated items | spec + plan | aeon master specs (banked) |
| S1 | **Aeon-native project model + compiler** — data model redesigned around the manifest superset; project → packer input → MEV blob; existing SMPS-shaped model migrated | spec + plan | S0 |
| S2 | **Verification gate** — export-time A/B: Seraph chip render vs oracle/VGM render of the compiled song; CI-style, stays after S3 | plan (small spec §) | S1 |
| S3 | **Driver-in-the-loop preview** — embedded Z80 core + real Memra blob; play = compile + play MEV; budget meter becomes ground truth | spec + plan | S1; useful before S4 |
| S4 | **Aeon-profile authoring UX** — the four strata below | spec + plan | S1 (S3 strongly preferred first) |
| S5 | **Import funnel** — SMPS/VGM/Zyrinx importers retargeted to emit the native model; GEMS importer added; SMPS export kept as marked-lossy | spec + plan | S1 |
| S6 | **SFX Workshop** — SFX project type + workflow (below) | spec + plan | S1 + engine package 2 (Stage B/C header fields); parallel-ok with S5 |

Each package plan header must declare its engine-package dependencies
(e.g. S4 ghost/echo UX ⇐ engine package 5 Tier 2; S6 header form ⇐ engine
package 2) so execution order vs the aeon queue is explicit.

## S4 — Aeon-profile UX (detailed, approved)

### Stratum 1: Track types & the channel rack (timeline-structural — NOT plugins)

Track header area = hardware rack: 6 FM + 3 PSG + noise + DAC lanes.

- **FM6 ↔ DAC is one linked pair.** Song-level three-way mode (dedicate /
  full-FM6 / time-share, = `SH_F_FM6_FM`/`SH_F_FM6_ADAPTIVE`): dedicate greys
  FM6's note lane; full-FM6 greys the DAC lane; time-share shows both and
  renders FM6 notes with visible "duck notches" under each drum hit — the
  interruption is seen, not discovered on hardware.
- **FM3 special mode is a track view-switch** (budget-gated flag): flipping it
  splits FM3 into four op-pitch lanes (alg-7 chord mode, one shared patch,
  TL-as-volume per lane) and removes the plain FM3 lane — engine forbids both,
  UI never shows both.
- **Noise track "pitch source" mode:** rate-3 tone-coupled noise commandeers
  PSG3's frequency and silences its tone — the UI links the lanes and greys
  PSG3's note lane while coupled, mirroring the hardware exactly.
- **Sub-voice lanes:** any FM/PSG track stacks instrument lanes sharing the
  physical channel. Piano roll enforces channel-monophony ACROSS lanes;
  lane switch on a note boundary compiles to instant `MEV_PATCH`; overlapping
  notes are impossible to place.
- **SFX-steal shading:** FM1/FM2 badged "protected" (never stolen — leads/bass
  here); FM3–5, PSG1–3, noise badged "stealable"; optional preview toggle
  simulates a steal so you hear what survives.

### Stratum 2: Note gestures (piano roll properties)

Per-note, engine-exact: gate length (`MEV_NOTEFILL`, FM-only) as note-length vs
sounding-length; pitch envelope (1–5 point trill/arp, `MEV_PITCHENV`) as a
note-attached stepped curve; **patch-swap vs REGDELTA morph as two distinct
gestures** — swap = new instrument at key-on; morph = a drawn timbre curve
inside a held note compiling to minimal register deltas, visibly never
re-attacking; portamento (lint: requires prior note), detune, vibrato
depth/rate/onset (`MEV_MODSET`); per-operator brightness (`MEV_OPBIAS`);
raw-frequency notes (`MEV_NOTE_RAW`) for sub-C0 bass / microtuning. Init-
ordering rules (patch+vol before first time-advancing event) are compiler-
inserted, invisible to the author.

### Stratum 3: The plugin rack (manifest-declared)

Three plugin kinds:

- **Runtime inserts/sends** (compile to MEV/MEV_EXT/macro events):
  **Sidechain Pump** (trigger-drum picker + depth knob → `MEV_EXT PUMPSET`);
  **Autopan** (rate → macro `TAG_MAC_PAN` tags); **Ghost/Echo send** and
  **Unison** presented as two plugins compiling to ONE mechanism
  (`MEV_EXT GHOSTSET`; delay=0+detune = unison) — the send visibly reserves a
  spare FM channel in the rack (packer-validated: not in the score's roster),
  greying it for normal use; per-channel automation lanes compile to the
  slot[1] macro spine.
- **Global rack:** hardware LFO ($22) is ONE global unit — a single LFO whose
  per-track depth is AMS/FMS in the pan byte; never per-track LFOs that would
  lie. Tempo automation (`MEV_TEMPO`) lives here.
- **Offline/export plugins** (engine package 5 Tier 0): drum mastering chain
  (EQ/comp/saturate/gated-verb, seed-deterministic); humanize/ghost-note/flam
  variation (seeded; flams bake to pre-mixed composite samples — never runtime
  mixing); TL-filter-sweep (a "filter envelope" on FM patches compiling to
  modulator-TL macros — the 303 trick without knowing it's a trick); PSG
  periodic-noise sub-bass helper. These render at export, and the
  driver-in-the-loop preview plays the POST-processed result — offline ≠
  inaudible.

Every plugin's manifest entry names its engine dependency (package + tier +
budget gate); gated-off = greyed with tooltip.

### Stratum 4: Song-level declarations & game integration

Song settings panel: loop vs finite ending (compiles `MEV_LOOP_POINT`/`MEV_JUMP`
vs all-channel `MEV_END` — the song-finished contract), FM6 mode, header tempo
(S3K TempoWait units, with a real-BPM readout), patch bank, optional pitch-table
override. **Marker lane** → `MEV_EXT COMM` cue events (intro-done, loop-hit,
stinger points — game polls `SND_STAT_COMM`). **Jingle project type** enforces
the validity class as a template: ≤3 voices, FM4/FM5+PSG only, no FM6/DAC, no
loop. **DAC sample manager:** import any WAV → auto-resample to engine rate →
per-sample mastering chain → composite baking; enforces 9-byte descriptor
constraints, <32 KB, no $8000-window straddle.

**Cross-cutting budget meter:** per-frame Z80 write-cost readout in the
transport (one FM patch load per frame; REGDELTA/macro traffic is additive) —
soft warnings while composing; the driver-in-the-loop preview is the hard truth.

## S6 — SFX Workshop (detailed, approved)

Project types: **Song / Jingle / SFX**, sharing instruments, piano roll, and
compiler. SFX = a very short score with different header metadata and channel
rules. The mode:

- **"New SFX" → single-screen editor:** 1–3 short lanes (SFX voice cap), a few
  bars max, instant re-audition on every edit (space replays through the
  driver-in-the-loop; sub-second loop). Sound design is iteration speed.
- **Archetype templates:** jump, ring, explosion, splash, skid, charge —
  pre-wired patch + sweep + envelope combos to tweak, not blank grids.
- **Sweep/burst generator plugins:** pitch sweep (start/end/curve → porta or
  PITCHENV), noise burst (mode/rate/decay → PSGNOISE + envelope), zap arp
  (REGDELTA flutter) — compiling to the same opcodes songs use.
- **The SFX header as a form:** priority (the ≥$C0 "ducks music" threshold
  visible), `sfh_gain`/`sfh_duck`/`sfh_cap` (engine package 2), one-shot vs
  continuous class (`SHF_CONTINUOUS`: spindash charge, drowning warning — with
  re-ping countdown semantics simulated in preview), instance cap.
  `MEV_SPINREV` exposed ONLY here (music-illegal).
- **Audition-in-context:** load any song, fire the SFX over it through the real
  driver — hear the channel steal, duck ramp, and restore before hardware does.
  Falls out of driver-in-the-loop nearly free; killer feature of the mode.
- **Table integration:** export assigns/updates the SFX id in the engine's
  generated tables (`gen_sound_tables.py` twin) — no manual bookkeeping between
  "make sound" and "hear it in game."

## Verification strategy

- **S2 gate (permanent):** compiled blob rendered via oracle/VGM → audio A/B
  (energy + spectrum, per `feedback_verify_real_output_not_proxy`) against
  Seraph's own render. Runs at export and in CI.
- **S3 onward:** preview IS the driver — divergence between "what Seraph plays"
  and "what the ROM plays" is structurally impossible for the sound path;
  remaining risk narrows to the 68k-side command layer (covered by S2's gate).
- **Manifest drift:** S0's build check regenerates/validates the manifest
  against aeon's source on every aeon sound change; mismatch fails the build,
  not the composer.
- **Golden round-trips:** HCZ2 + Moving Trucks imported → re-exported must be
  byte-identical (or documented-equivalent) MEV blobs — the regression anchor.

## Out of scope / doors

- Engine-side changes: none. Seraph consumes banked engine specs; anything the
  UX wants that the engine lacks becomes a proposal against the engine queue,
  not a silent Seraph shim.
- CSM formant mode, PSG 3-ch PCM, looped DAC: engine design-doors; appear in
  Seraph only if/when their manifest flags exist.
- Live MIDI input: user is mouse+keyboard (Ableton-style workflow); not in any
  package.
- Aurora/level-editor integration (music placement per act): later, own design.

## Next steps

1. User reviews this design doc (gate).
2. Per-package spec sessions (S0 first — the manifest schema is the contract
   everything else consumes), each ending in a cold-executable plan, recorded
   in a Seraph banking queue doc mirroring aeon's.
3. Execution sessions (any model) follow the queue.
