# S5 — Import Funnel Implementation Plan (incl. spec §)

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Checkbox steps. Repo: seraph, branch `feat/s5-import-funnel`. Depends: S1 (model v2). GEMS research 2026-07-03 is embedded below.

## Spec § (design)

Wide import funnel into the native (Aeon-superset) model: retarget the three
existing importers (SMPS, VGM, Zyrinx) from the v1 SMPS-shaped model to model
v2, add a GEMS importer, keep SMPS export as the marked-lossy path (done in
S1 Task 6). Every importer emits a **MigrationReport**-style ImportReport:
what mapped, what was flattened, what was dropped — never silent loss.

**GEMS (new; research verdict feasible-high):** banks = sequences +
instruments + envelopes + DAC samples; tracks are bytecode streams ($00–$5F
note-on, $60–$7F commands, $80–$BF duration, $C0–$FF delay, 6-bit
accumulating). Maps losslessly: notes/durations/patch/tempo/pitch-bend/
counted loops ($64/$65 → repeats or unroll); FM patches are $26-byte near-raw
YM register dumps → native FM instruments. Flatten: pitch/mod envelope bank →
automation curves; backward-jump/cond-jump loops → song loop point (cap
iterations). Drop with report: mailbox branching ($70/$71), song-trigger
($6B), priority/mute/SFX-timebase. Decoder reference: ValleyBell `gems2mid.c`
(read as spec — no license header, do NOT copy code); bank round-trip
reference: realmonster/GEMS (LGPL — reference only); ground-truth renderer
for A/B: GEMSPlay. Pointer width autodetect (2-byte LE v2.0–2.5 / 3-byte
v2.8).

**Existing importers:** mechanical retarget — they currently build v1
structures (`src-tauri/src/` import modules + `driver/flamedriver.rs`
voice parsing); post-S1 they build model v2 (single-lane tracks). VGM import
additionally gains "quantize to driver ticks" (VGM is register-timeline;
current importer already reconstructs notes — verify + keep, add tick
quantization with a report of timing shifts >½ tick).

---

### Task 1: ImportReport plumbing
Files: `src-tauri/src/import/mod.rs` (or create), IPC + UI surface (reuse the S1 MigrationReport dialog).
- [ ] Shared `ImportReport { mapped, flattened, dropped: Vec<Entry{kind, count, detail}> }`; every importer returns one. Commit.

### Task 2: Retarget SMPS + Zyrinx + VGM importers to model v2
Files: locate current importers (`grep -r "smps\|zyrinx\|vgm" src-tauri/src --include=*.rs -il`) and port their output stage; tick quantization for VGM.
- [ ] Round-trip tests: each importer's existing fixture imports clean; quantization report test (synthetic VGM with off-grid timing). Commit per importer.

### Task 3: GEMS importer
Files: create `src-tauri/src/import/gems/{mod.rs,banks.rs,sequence.rs,patches.rs}`; fixtures under `src-tauri/testdata/gems/` (author tiny synthetic bank fixtures — do NOT commit game ROMs).
- [ ] banks.rs: 4-bank container parse + pointer-width autodetect.
- [ ] sequence.rs: bytecode decoder (event ranges above; 6-bit duration accumulation; loop stack 16 deep; backward-jump loop detection with iteration cap → loop point).
- [ ] patches.rs: $26-byte FM reg dump → FmInstrument (reg order verified against the YM register layout already used by flamedriver.rs's packer); PSG tone/noise + DAC types.
- [ ] Envelope bank → automation curves; drops reported.
- [ ] Tests: synthetic fixture with every construct → expected model JSON snapshot + ImportReport counts. Commit.

### Task 4: UI + closeout
- [ ] Import dialog gains GEMS; report dialog shown post-import; profile auto-set to memra with kind=Song.
- [ ] Optional A/B sanity (manual): import a GEMS track from a homebrew/donor bank, play via S3 preview vs GEMSPlay render — by-ear note in the queue log.
- [ ] Merge → main; queue S5 → DONE (+log). Commit.
