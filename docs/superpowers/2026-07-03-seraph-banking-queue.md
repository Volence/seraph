# Seraph Banking Queue — 2026-07-03

**What this is:** the canonical status record for the Seraph Aeon-profile
banking effort (same pattern as `aeon/docs/superpowers/2026-07-03-sound-banking-queue.md`).
Master design: `specs/2026-07-03-aeon-profile-banking-design.md` (APPROVED,
user, 2026-07-03). Grounding inventory:
`2026-07-03-memra-author-surface-inventory.md`. Each package gets a research
pass, a spec (where marked), a user review gate, and a cold-executable plan —
so any future session (any model) can execute without re-deriving.

Standing decisions: driver named **Memra** (docs/UI only); driver-in-the-loop
guarantee; Aeon-native model / wide import / narrow export; manifest lives in
**empyrean**; correct-by-construction UI over export-time validation.

## Package queue (execution order)

| # | Package | Deliverables | Status |
|---|---------|-------------|--------|
| S0 | **Memra contract** — capability manifest in empyrean, generated-from/validated-against aeon source; Memra naming pass; budget-gate feature flags | spec + plan | **BANKED** — spec APPROVED (user, 2026-07-03) + plan `plans/2026-07-03-s0-memra-contract.md` |
| S1 | Aeon-native project model + compiler (project → packer input → MEV) | spec + plan | **BANKED** — spec + plan (`133ff06`) |
| S2 | Verification gate (export-time oracle/VGM A/B; permanent CI-style) | plan (+ spec §) | **BANKED** — plan w/ spec § (`2ffd5c2`) |
| S3 | Driver-in-the-loop preview (embedded Z80 + real Memra blob) | spec + plan | **BANKED** — plan w/ spec § (`6e87d0e`) |
| S4 | Aeon-profile authoring UX (4 strata + plugin rack; approved in master design) | spec + plan | **BANKED** — spec + plan (`72c270f`) |
| S5 | Import funnel (SMPS/VGM/Zyrinx retarget + GEMS; SMPS export = marked lossy) | spec + plan | **BANKED** — plan w/ spec § (`6e87d0e`) |
| S6 | SFX Workshop (Song/Jingle/SFX project types; approved in master design) | spec + plan | **BANKED** — plan w/ spec § (`6e87d0e`) |

**Engine dependencies (aeon queue, banked 2026-07-03, not yet executed):**
S6 ⇐ engine package 2 (Stage B/C header fields); S4 ghost/echo + ExtCh3 UX ⇐
engine package 5 Tier 2 budget gates; COMM markers ⇐ engine package 1. Seraph
designs target the banked specs (normative); manifest flags carry the gates.

## Log

- 2026-07-03: Master design + inventory committed (fd14c4b). Queue doc created.
  S0 spec session opened.
- 2026-07-03: S0 BANKED — spec (c56e1aa) + plan (44520a4). Research: 3-agent pass
  (aeon extractability ~70% generated / 30% curated; prior art Furnace/LV2/CLAP/
  MIDI-CI/MDSDRV/Echo/XGM2/Vulkan/Wayland/buf; Seraph DriverProfile trait exists).
  Plan decisions: curated overlay = Python module (no PyYAML dep); song-header
  compat byte DEFERRED (zero engine changes); parity-baseline + ROM byte-identity
  gate the packer refactor.
- 2026-07-03: SECOND WAVE — S1 through S6 ALL BANKED same session (usage-driven
  sprint). S1 spec+plan (Rust MEV compiler, byte-parity vs song_packer, tick-native
  grid, tauri-specta first). S2 plan (blob-driven A/B render; semantic-gap list
  feeds S3 acceptance). S3 plan (vendor floooh chips z80.h cycle-stepped C shim,
  Nuked-OPN2 pattern; Timer-A-clocked, no vblank INT; aeon exports blob+symbol
  artifact; budget meter real tier). S4 spec+plan (component-level: ChannelRack/
  NoteInspector/PluginRack/LintPanel; offline plugins = Rust ports corpus-tested
  vs aeon Python). S5 plan (importer retargets + GEMS via gems2mid-as-spec;
  ImportReport everywhere). S6 plan (SFX kind, sfx_transcode byte-parity, header
  form manifest-gated, audition-in-context). QUEUE FULLY BANKED — next: execution
  sessions in order S0->S6 (S0 gates all; engine packages 1-6 still to execute in aeon).
- 2026-07-15: EXECUTION ORDER REVISITED. S0 DEFERRED (not started): the MEV
  format is still moving upstream (aeon `MEV_PORTA $F5` landed ~07-08; an active
  "sigil sound migration" (DSM series) is converting the sound pipeline toward
  binary/.emp, and `sound_constants.asm` is on the .emp conversion path). Freezing
  the manifest contract now would only buy repeated re-releases + a parser rework
  when the asm→.emp move lands. Decision: park S0 (and every manifest-dependent
  package) until aeon's sound format + sigil migration stabilize. Instead executed
  the one fully-decoupled task: **S1 Task 1 (tauri-specta adoption)** — Rust is now
  the single source of truth for IPC types; all 56 `#[tauri::command]`s
  specta-annotated; generated `src/bindings.ts` retires the hand-mirrored TS
  (`model.ts` 222→86). Caught+fixed a serde-vs-specta camelCase divergence
  (`d1r`/`d2r`, would have broken FM edits) and added a mechanical parity guard
  test. Two-stage review (spec ✅ + quality ✅). Merged to `main` (`437841e`),
  cargo test 180 pass, `npm run build` green. S1 Tasks 2–8 remain DEFERRED
  (blocked on the parked S0 manifest). Resume S0 when the sound format settles.
- 2026-07-16: INSTRUMENT LIBRARY SHIPPED (independent of the parked S-queue;
  spec + plan in docs/superpowers/{specs,plans}/2026-07-16-instrument-library*).
  Merged to main (`1799c3c`, 26 commits). Shipped: default pack (571→606
  entries: Sonic 2 123 FM, S3K 229 FM incl. the 35-voice UVB, Batman & Robin
  212 FM via the zyrinx importer, 42 shape-tagged PSG presets); full browser
  panel (search/filters/tags/favorites/warnings, detail card w/ op grid + PSG
  sparkline, per-note PianoKeys audition); sha256 content-hash identity +
  idempotent extraction CLI (smps/uvb/gyb/zyrinx/psg-table); drag-to-track
  swap (hash-reuse); import recognition (imported voices auto-named from the
  library by hash); save-from-project; first frontend test infra (vitest/RTL).
  NOTABLE: by-ear testing root-caused and fixed a PRE-EXISTING audio bug —
  the FM preview path wrote operators in reverse slot order vs the sequencer
  (`OP_REG_OFFSETS` vs `PACKED_OP_SLOTS`, now one shared authority in
  model/instrument.rs) — every FM audition/preview before this played wrong;
  also fixed drag-swap inaudibility (per-note instrument_id precedence) and
  a Wayland Error 71 crash (compositing disabled in main.rs). Deferrals
  recorded: LBZ1.asm voices (channel-label parse failure, 1 song),
  Sub-Terrania/Red Zone parser extension, DAC samples (library v2),
  PSG preset name curation, keyboard/touch audition, list virtualization.

- 2026-08-19: **S0 UNPARKED** — the aeon overseer session ruled the park condition
  satisfied, and this session verified the claims firsthand at aeon `236c306b`
  (master HEAD): `engine/sound/sound_constants.emp` is the constants authority,
  `sound_constants.asm` no longer exists anywhere in the aeon tree, and sigil's
  `emit_sound_blob` release binary (`SIGIL_EMIT`, hard-required by aeon's
  build.sh) is the production blob path. Last sound-touching aeon commit:
  `8b39969d` (2026-08-11, SFX content import — no format change). **Re-grounding
  required before executing S0**: the banked plan predates the asm→.emp move and
  still references `sound_constants.asm`/`song_packer.py`; re-ground its inputs
  against `sound_constants.emp` + the `emit_sound_blob` contract at pinned aeon
  SHA `236c306b`, and parse constants from source at use time — never transcribe
  them into seraph-side constants that can drift. Three caveats from the aeon
  ruling (transcribed; aeon side to anchor): (1) aeon sound packages 5/6 are
  still open, and the 2026-08-13 sound-lens sweep has an unmerged packet (two
  live findings: multi-slot SFX cap; a DAC/DMA wedge class) — any MEV/constants
  change from that work arrives as explicit notice BEFORE landing, contract-style.
  (2) The MEV_EXT registry invariants (slots 0/1/2) are load-bearing aeon-side —
  extending the registry is a cross-repo ask via the demand-doc flow, never a
  unilateral read. (3) aeon's streaming arc may later couple to the driver's
  DMA-survival design (max-contiguous-DMA-stall) — if S0 work touches DAC/DMA
  timing assumptions, notify aeon and coordinate. S0 is now READY TO EXECUTE;
  actually opening the execution session is an owner call.

## EXECUTION HANDOFF (cold start — read this first)

For any future session executing this queue:
1. **Start with S0** — it gates everything Seraph-side. Open its plan, follow
   the subagent-driven-development skill per the plan header. Each plan is
   self-contained: exact paths, code, commands, gates.
2. **Interleaving with the aeon engine queue** (`aeon/docs/superpowers/2026-07-03-sound-banking-queue.md`):
   Seraph **S0–S3 are executable IMMEDIATELY** — they depend only on the
   SHIPPED driver (manifest flags mark unshipped features `reserved`).
   S4 is executable too (reserved features render greyed); only flipping those
   flags to `shipped` — and S6's full header form — waits on aeon packages
   1/2/5. After any aeon package lands, regenerate the manifest
   (`python3 tools/gen_capability_manifest.py`), re-release to empyrean, and
   flip the affected feature statuses in the curated overlay.
3. **Standing verification norms** (from repo memory — they bind every plan):
   sound builds need `SOUND_DRIVER_ENABLED=1 DEBUG=1` (+`SOUND_DEBUG_HOTKEYS=1`
   for sound testing); oracle = ONE instance, controller-session only (never
   subagents); verify rendered audio, never register proxies; commit exact
   paths, never `-A`; never leave master broken.
4. **User gates:** by-ear passes in S3 Task 5 and S6 Task 4; everything else
   is mechanical against the written gates.
