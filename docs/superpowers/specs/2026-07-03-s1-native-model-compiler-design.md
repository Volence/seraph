# S1 — Aeon-Native Project Model + MEV Compiler — Design

**Date:** 2026-07-03 · **Status:** BANKED (designed under standing approval of the
master design; key decisions flagged in the queue log for user review)
**Depends:** S0 (manifest). **Grounding:** master design §S1, the author-surface
inventory, the Seraph consumption map (2026-07-03 research pass).

## Goal

Redesign Seraph's project data model around the Memra manifest superset and
compile projects to MEV blobs — the "narrow export, lossless for Aeon" half of
the profile decision.

## Key decisions

1. **Rust MEV compiler, cross-verified against song_packer.py.** Three options
   considered: (a) port the packer to Rust and retire the Python one — rejected,
   aeon's build must stay self-contained; (b) shell out to Python at export —
   rejected, a desktop app can't depend on the user's Python; **(c) CHOSEN:**
   Seraph compiles MEV natively in Rust, enforcing rules from the loaded
   manifest, and a **byte-parity harness** in CI feeds identical scores through
   both compilers and diffs the blobs. The manifest guarantees rule parity;
   the harness guarantees byte parity. Divergence fails CI, never ships.
2. **Tick-native timeline for the Aeon profile.** The DAW grid is driver ticks
   (S3K TempoWait model), not free BPM: a note's start/length are integer ticks;
   header `tempo_mod` + `MEV_TEMPO` automation set real-time speed; the UI shows
   a derived BPM readout (~59.92 Hz × (256−mod)/256 / ticks-per-beat). No
   quantization surprises at export — what's on the grid IS what plays.
3. **Model = manifest superset, typed in Rust, serde-versioned.** Existing
   SMPS-shaped projects get a `projectVersion` bump with a best-effort migrator
   (notes/regions/instruments carry over; SMPS-specific effects flagged in a
   migration report, not silently dropped).
4. **tauri-specta adopted first** (consumption-map Phase 1) to end the manual
   Rust↔TS type mirror; manifest types generated from the S0 schema.

## Model (Rust, `src-tauri/src/model/`)

- `Project { version, kind: Song|Jingle|Sfx, driverProfile, songSettings, tracks, instruments, dacSamples, markers }`
- `SongSettings { fm6Mode: Dedicate|FullFm6|TimeShare, looped: bool, tempoMod: u8, pitchTableOverride: Option<...>, patchBank }`
- `Track { route(s): from manifest channel classes, lanes: Vec<Lane> }` — a
  Lane = sub-voice (instrument + regions); monophony across a track's lanes is
  a MODEL invariant (validated on edit, not just export).
- `Note { startTick, lenTicks, pitch: 0..0x5E | RawFreq{a4,a0}, vel→Vol, gate: Option<u8> /*NOTEFILL*/, pitchEnv: Option<Vec<u8>> /*1-5*/, porta/detune/vibrato props, morph: Option<MorphCurve> /*→REGDELTA*/, laneSwitch→Patch }`
- `AutomationLane` → slot[1] macro stream (typed points → TAG_MAC_* events);
  `Marker` → MEV_EXT COMM (feature-gated).
- Jingle/Sfx kinds carry their class constraints (≤3 voices, no FM6/DAC, no
  loop / SFX header fields) as model-level validation from manifest `rules`.

## Compiler (`src-tauri/src/compiler/mev/`)

Pipeline: model → per-channel event list (init-order injection: Patch+Vol /
Vol first) → duration encoding (SetDur runs + NoteDur exceptions) → structural
passes (repeat detection optional/deferred; LoopPoint/Jump vs all-End from
`looped`) → operand validation against the manifest → blob assembly (header,
BE offsets, macro body back-patching mirroring `pack_song`) → `.asm` emitter
(`dc.b`, even-terminated, labeled — byte-format identical to `emit_asm`).
Every rule enforced here cites a manifest rule id; violations carry the rule id
+ prose to the UI (correct-by-construction means these mostly can't fire —
they're the compiler's last line, not the UX).

## Byte-parity harness

`tools/parity/` (aeon repo): a JSON "score interchange" format both sides can
read (Seraph exports it; a thin Python shim builds the same song via
song_packer API). CI: N corpus scores (incl. HCZ2/MT re-expressed via importers
once S5 lands; until then, synthetic corpus covering every opcode) → both
compilers → `cmp` blobs. Runs in Seraph CI; aeon unaffected.

## Out of scope

Playback of the new model beyond current chip-preview (S3 replaces preview);
authoring UX for the esoteric features (S4 — S1 only makes the model able to
REPRESENT them); importers (S5). SMPS export keeps working via a
model-downconversion shim (lossy, warns) — small task, included.

## Verification

Model invariant tests (lane monophony, jingle class); compiler golden tests
(hand-built scores → expected bytes, transcribed from test_song_packer.py
cases); byte-parity harness green on the synthetic corpus; migration
round-trip (old project loads, report generated); specta types compile in TS.
