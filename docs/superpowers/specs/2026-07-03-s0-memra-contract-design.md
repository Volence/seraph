# S0 — Memra Contract (Capability Manifest) — Design

**Date:** 2026-07-03
**Status:** APPROVED (user, 2026-07-03)
**Package:** S0 of the Seraph banking queue (`../2026-07-03-seraph-banking-queue.md`);
master design `2026-07-03-aeon-profile-banking-design.md`.
**Grounding:** `../2026-07-03-memra-author-surface-inventory.md` (the full author
surface) + three research passes 2026-07-03: aeon extractability analysis,
cross-ecosystem prior art (Furnace, DefleMask, LV2, CLAP, MIDI-CI, MDSDRV,
Echo, XGM2, Vulkan, Wayland, buf), Seraph consumption mapping.

## Goal

A machine-readable contract describing everything the Memra driver accepts —
opcodes, operands, ranges, route legality, channel classes, structs, limits,
feature flags — generated from the aeon source so it cannot drift, versioned so
consumers can trust it, and consumable by both Rust and TypeScript. Memra is
trust-the-packer (invalid data hangs the Z80 with no error); this manifest is
the single source every validator and UI constraint derives from.

Prior-art conclusion: no Genesis driver ecosystem has a machine-readable
contract (MDSDRV/Echo/XGM2 are prose specs synced by single-maintainer
discipline; every one that grew third-party tools got drift and duplicated
hand-coded validators). Nothing to copy; three failure modes to avoid (§7).

## Artifacts (by repo)

| Repo | Artifact |
|---|---|
| empyrean | `contract/schema/memra-manifest.schema.json` (JSON Schema, draft 2020-12) — follows the existing `bus-protocol.schema.json` convention |
| empyrean | `contract/memra-manifest.json` — the released manifest instance (copied from aeon build output on release; tagged) |
| aeon | `tools/song_packer.py` refactor: declarative `OPCODE_SPECS` table (see §3) |
| aeon | `tools/gen_capability_manifest.py` (new) — extractor/merger |
| aeon | `tools/sound_manifest_curated.yaml` (new) — hand-curated semantic overlay |
| aeon | `tools/validate_manifest_drift.py` (new) — build-time drift check, hooked into `build.sh` |
| aeon | Memra naming pass — docs-level only (§6) |

Seraph consumption (typify/json-schema-to-typescript codegen, DriverProfile
integration) is **S1's** job; S0 ends at a validated, versioned, published
manifest.

## 1. Manifest shape

Single JSON file, sectioned (LV2's tiny-manifest-plus-links pattern rejected:
Seraph loads exactly one known manifest; splitting adds resolution logic for
nothing):

- `meta` — `formatVersion` (int), `driverCompat: {major, minor}`, driver build
  hash, generation timestamp + source commit.
- `channels` — channel classes (`fm`, `fm6_adaptive`, `fm3_special` [flagged],
  `psg_tone`, `psg_noise`, `dac`) with per-class: route ids, count, pitch
  range, volume range, SFX-steal exposure (protected/stealable), instrument
  kind accepted, mode exclusions (FM6↔DAC tri-mode; rate-3 noise silences
  PSG3; FM3-special forbids plain FM3).
- `opcodes` — array of records: byte value, mnemonic, operand list (each:
  name, bytes, encoding, min/max or enum ref, semantic units string),
  `legalOn: [channelClasses]`, `tickModel: time_advancing|zero_tick`,
  `feature` (gating flag id or null), structural rules refs (init-order,
  repeat, macro classes).
- `enums` — shared operand value sets: noise ctrl bytes, LFO rates, MEV_EXT
  sub-op registry (0=COMM, 1=PUMPSET, 2=GHOSTSET), macro tags, header flags.
- `features` — map of namespaced string ids (`memra.ghostvoice`,
  `memra.ext.pumpset`, `memra.fm3.optracks`, `memra.comm`, …) →
  `{status: shipped|experimental|reserved, since: "major.minor",
  enginePackage: n, budgetGated: bool}`. Unshipped/budget-gated features are
  PRESENT as `reserved` so Seraph greys them out instead of not knowing them.
- `limits` — structural budgets: blob ≤ $FFFF, channel count 1..11, repeat
  single-level, patch/env/sample table formats + sizes (FmPatch=32 B,
  DacSample=9 B), DAC sample constraints (<32 KB, no $8000 straddle), YM
  write-pacing facts for the budget meter (≥1 frame between FM patch loads),
  song header layout (field offsets).
- `rules` — the validity rules that aren't per-opcode data expressed as typed
  rule records (id, class, params, prose): init-ordering, stream termination,
  loop/repeat body must advance time, macro body termination, porta-needs-
  prior-note, jingle class (≤3 voices, no FM6/DAC, no loop). Typed so Rust
  can dispatch enforcement per rule id; prose is for humans.

**Everything is data.** No rule may require executing code to interpret
(Furnace's lambda-transform failure). Operand transforms limited to
offset/scale/clamp/enum/table-ref. If a rule can't be expressed as data,
that's a design smell escalated to the queue, not hacked around.

## 2. Generation: generated core + curated overlay

- **~70% generated** from aeon source: opcode values/operands/ranges/route
  legality from the refactored `OPCODE_SPECS` (§3); struct layouts, header
  offsets, flag bits, hardware constants, build asserts parsed from
  `sound_constants.asm` (regular equ/struct/if-error syntax); pitch/envelope
  table facts imported from `gen_sound_tables.py`.
- **~30% curated** in `sound_manifest_curated.yaml`: semantic descriptions,
  units, tick model annotations (cross-checked against extracted data where
  possible), feature flag statuses, rule prose. Version-pinned, reviewed.
- `gen_capability_manifest.py` merges both → `memra-manifest.json`; fails if
  any extracted opcode lacks a curated entry (no silent undocumented surface).
- `validate_manifest_drift.py` runs in `build.sh` (with the existing sound
  generation steps): re-extracts, compares to the committed manifest +
  overlay, fails the build on undocumented deltas. Also asserts
  packer-constants == asm-constants (the two hand-mirrored definition sets
  today have no sync test).

## 3. The packer refactor (aeon-side; user-flagged and approved)

`song_packer.py`'s per-Event imperative `validate()` methods become a
declarative `OPCODE_SPECS` dict (opcode → operands/ranges/routes/tick model);
`validate()` becomes a generic interpreter of that table. The packer then
enforces from the same data the manifest is generated from — contract and
enforcer cannot drift by construction.

**Behavior-preserving, golden-guarded:** HCZ2 + Moving Trucks + drum-test +
all shipped SFX blobs must be byte-identical before/after the refactor
(regression harness exists; extend if any blob is uncovered). Error-message
text may change; acceptance/rejection behavior may not — a rejection-matrix
test (every opcode × every route × boundary operands) is part of the task.

## 4. Versioning & compatibility

1. `formatVersion` (manifest schema shape): integer, append-only rule
   (Wayland) — additions only; breaking schema change increments it and
   consumers refuse unknown majors.
2. `driverCompat {major, minor}` (MDSDRV's proven rule): packed song data is
   compatible iff same major; minor is additive-only. Stamped in the manifest;
   optionally echoed into the packed song header (1 byte) in a DEBUG-checked
   load assert — decided at plan time, zero-cost if deferred.
3. **Feature flags for everything else** — consumers ask "is
   `memra.ghostvoice` shipped," never "is version ≥ N" (browser lesson).
   Namespaced string ids (CLAP); unknown ids ignorable.
4. Named profiles (Vulkan/MIDI-CI): explicitly NOT built now; escape hatch if
   flag combinatorics grow.
5. CI contract-diff (buf lesson): aeon job diffs manifest vs last tagged
   release; fails on breaking classes — opcode byte reuse, operand range
   narrowing, route-legality removal, feature status regression
   (shipped→anything), limit tightening. "Never reuse an opcode byte."

## 5. Consumption contract (binding on S1+, stated here)

- Schema-first: the JSON Schema is the cross-repo artifact. Rust types via
  typify (or schemars round-trip), TS types via json-schema-to-typescript.
- **One validator: Rust.** TypeScript gets types for UI binding only — never a
  second validation implementation to drift. (Seraph mapping confirmed the
  manual Rust↔TS mirror is already the codebase's biggest skew risk.)
- Slots into Seraph's existing `DriverProfile` trait
  (`src-tauri/src/model/driver.rs`) — layout/features/validation derive from
  the manifest instead of hardcoded consts.

## 6. Memra naming pass (docs/UI level ONLY)

- aeon: name the driver Memra in `ENGINE_ARCHITECTURE.md` §sound, the sound
  spec headers (one-line retcon note), and CLAUDE.md's engine summary line.
  `MEV_*` documented as "Memra EVent". NO symbol renames.
- empyrean: `contract/README.md` gains the Memra manifest entry; ROADMAP
  mention.
- seraph: UI naming lands with S1/S4 (profile picker string "Aeon (Memra)").

## 7. Pitfalls this design exists to avoid

1. Executable capability definitions (Furnace) → all-data rules (§1).
2. Prose-synced spec + hand-copied validators (DMF/Echo/XGM2) → generation +
   build-time drift check (§2) + one-validator rule (§5).
3. Version numbers without accept/reject semantics (DMF) → §4's explicit
   rules; flag explosion (early Vulkan) → namespaced flags now, profiles
   reserved.

## 8. Verification / acceptance

- Golden byte-identity: all shipped song/SFX blobs unchanged across the packer
  refactor.
- Rejection matrix: table-driven validate() rejects exactly what the old code
  rejected (opcode × route × boundary operands).
- Manifest completeness: every opcode in `sound_constants.asm` $80–$FF space
  appears (generated or explicitly reserved); every feature in the banked
  engine specs has a flag entry with correct status.
- Schema validity: manifest instance validates against the schema (jsonschema
  in CI); schema itself lints.
- Drift check: mutating a packer range or an asm constant without regenerating
  fails `build.sh` (negative test).
- Conformance smoke: packer emits a maximal-legal exercise song touching every
  shipped opcode/route; oracle plays it without hang (FOREGROUND controller
  step); manifest-vs-`s4.lst` symbol cross-check at driver build.

## Out of scope

Seraph-side codegen/integration (S1); driver-in-the-loop (S3); any Z80/engine
code change (the optional song-header compat byte is DEBUG-only and decided at
plan time); MEV_EXT allocations beyond the registered 0/1/2.
