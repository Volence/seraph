# S1 — Aeon-Native Model + MEV Compiler Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:subagent-driven-development (or executing-plans). Checkbox steps. Repo: /home/volence/sonic_hacks/seraph, branch `feat/s1-native-model`. Read the spec (`../specs/2026-07-03-s1-native-model-compiler-design.md`), the S0 manifest (`empyrean/contract/memra-manifest.json` — S0 MUST be executed first), and the author-surface inventory before Task 1. Each task: research the named files first, TDD (cargo test / vitest), commit per task with exact paths.

**Goal:** Manifest-driven project model + Rust MEV compiler, byte-parity-verified against aeon's song_packer.

**Tech stack:** Rust (serde, tauri-specta added), TS/React, existing Tauri IPC.

---

### Task 1: tauri-specta adoption (type-sync debt paid first)
Files: `src-tauri/Cargo.toml`, `src-tauri/src/lib.rs`, `src-tauri/src/ipc/commands.rs`, generated `src/bindings.ts`; delete duplicated interfaces from `src/types/model.ts` as bindings replace them.
- [ ] Add `tauri-specta` + `specta` crates; annotate every existing IPC command; wire codegen in `src-tauri/build.rs` or a dev command emitting `src/bindings.ts`.
- [ ] Migrate `src/api/ipc.ts` wrappers to the generated types; `npm run build` green; `cargo test` green.
- [ ] Commit: `feat: tauri-specta codegen replaces hand-mirrored TS types`.

### Task 2: Manifest loading + `MemraProfile`
Files: create `src-tauri/src/driver/memra.rs`; modify `src-tauri/src/model/driver.rs`, `src-tauri/src/lib.rs` (registry), `src-tauri/src/ipc/commands.rs` (`get_manifest_constraints`).
- [ ] Vendor the released manifest: copy `empyrean/contract/memra-manifest.json` to `src-tauri/assets/memra-manifest.json` (include_str! at build; a `--manifest-path` dev override for testing newer aeon builds). Record the pinned driverCompat in the profile.
- [ ] Deserialize into `Manifest` structs (serde; field names exactly per S0 schema — write from the schema, validate by loading the real file in a unit test).
- [ ] Implement `DriverProfile` for `MemraProfile`: `channel_layout()` from `channels`, `supports_feature()` from `features` (status=="shipped" only unless a dev "preview reserved features" flag), validate_* from manifest ranges.
- [ ] New IPC `get_manifest_constraints(driverId)` returning opcodes/limits/rules for UI binding. Commit.

### Task 3: Project model v2
Files: modify `src-tauri/src/model/song.rs` (or create `model/project.rs` and re-export), `src-tauri/src/model/instrument.rs`; migration in `src-tauri/src/project/migrate.rs` (new).
- [ ] Types per spec §Model (Project/SongSettings/Track/Lane/Note/AutomationLane/Marker; `kind: Song|Jingle|Sfx`). `projectVersion: 2`. All specta-annotated.
- [ ] Model invariants as methods + unit tests: lane monophony across a track (`Track::validate() -> Vec<RuleViolation>` where RuleViolation carries the manifest rule id), jingle class rules (voices ≤3, forbidden routes, no loop), note field ranges from manifest.
- [ ] Migrator: v1 project JSON → v2 (tracks/regions/notes/instruments map 1:1 to a single-lane track; SMPS-only effects collected into a `MigrationReport { warnings }` returned to the UI). Round-trip test with a fixture v1 project file (create one from the current model's serde output). Commit.

### Task 4: MEV compiler core
Files: create `src-tauri/src/compiler/mev/{mod.rs,events.rs,encode.rs,header.rs,asm_emit.rs}`; tests `src-tauri/src/compiler/mev/tests.rs`.
- [ ] `events.rs`: an `Event` enum mirroring OPCODE_SPECS entries 1:1 (29 variants; operands typed; encode() -> Vec<u8> byte-identical to song_packer's encode bodies — transcribe each from `aeon/tools/song_packer.py`, which the executor MUST read).
- [ ] `encode.rs`: model→events lowering — init-order injection (FM: Patch+Vol; PSG: Vol before first time-advancing event), SetDur run-length choice (emit SetDur when ≥2 consecutive notes share a duration, NoteDur otherwise — deterministic rule, document it), gate→NoteFill, morph→RegDelta groups, automation→slot[1] tags, looped→LoopPoint/Jump else End on every channel.
- [ ] `header.rs`: SongHeader assembly per manifest limits (flags from fm6Mode, tempoMod, chcount, BE offsets, macro body back-patching — mirror `pack_song` exactly; read it).
- [ ] `asm_emit.rs`: `dc.b` emitter matching aeon's `emit_asm` output format (label, even-termination).
- [ ] Golden unit tests: transcribe ≥6 cases from `aeon/tools/test_song_packer.py` (same input events → assert identical bytes, expected arrays pasted into the Rust tests). Commit per sub-file if large.

### Task 5: Byte-parity harness (cross-repo)
Files: create `src-tauri/src/compiler/mev/interchange.rs` (score JSON in/out); create in AEON: `tools/parity_shim.py`, `tools/testdata/parity_corpus/*.json`; create seraph `scripts/parity-check.mjs` (or a cargo test behind `--features parity` invoking python3 when available).
- [ ] Score-interchange JSON: `{header:{...}, channels:[{route, events:[{op:"Note", args:[...]}, ...], macro:[...]}]}` — 1:1 with Event names. Serializer+deserializer both sides (Rust + a 60-line Python shim building song_packer objects by name via getattr).
- [ ] Synthetic corpus: one score per opcode exercising boundaries (generate programmatically; ~29 files) + one "everything" score (reuse S0's `gen_exercise_song.py` shape).
- [ ] Parity test: for each corpus file, Rust blob == Python blob (skip cleanly when python3/aeon checkout absent; REQUIRED in dev env). Any diff = bug in the Rust side by definition (packer is authority). Commit both repos.

### Task 6: Export path + SMPS downconversion shim
Files: modify `src-tauri/src/ipc/commands.rs` (`export_song` dispatch), create `src-tauri/src/export/mev.rs`; modify `src-tauri/src/export/smps.rs` entry to accept model-v2 via a downconversion (`compiler/smps_downconvert.rs` new: v2→v1-shaped structures; unrepresentable features → `ExportWarning` list, marked LOSSY in the UI result).
- [ ] MEV export writes `<name>.asm` (+ optional raw `.bin`) + a `manifest-compat` stamp (driverCompat major.minor) in the header comment.
- [ ] UI: export dialog shows target (Aeon (Memra) / SMPS-lossy / VGM) with warnings surfaced. Commit.

### Task 7: UI constraint binding (minimum for S1)
Files: `src/components/FmEditor.tsx`, `src/components/PsgEditor.tsx`, `src/components/AddTrackDialog.tsx`, new `src/api/manifest.ts`.
- [ ] Fetch constraints once per project open; knob min/max + channel lists driven by manifest (replace hardcoded); tick-native grid setting when profile==memra (tempo readout: `59.92*(256-mod)/256` events/sec displayed alongside).
- [ ] `npm run build` + smoke the app (`npm run tauri dev`) — create project, add track, edit instrument, export MEV of a 4-note score; verify the .asm appears and parity-checks. Commit.

### Task 8: Closeout
- [ ] Full: `cargo test`, `npm run build`, parity harness green.
- [ ] Merge `feat/s1-native-model` → main. Update the queue doc S1 → DONE (+log). Commit exact paths.

**Deferred inside S1 (recorded):** repeat-block detection in the compiler (emit unrolled first; REPEAT compression is an optimization pass banked as a follow-up note in the queue log — packer parity corpus must then gain repeat cases); Jingle/Sfx export headers (S6 wires them; model carries them now).
