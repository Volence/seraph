# Seraph Overseer

**Boot prompt** (paste into a fresh session opened in this repo):

> You're the overseer for this repo. Read `docs/OVERSEER.md` first, then
> `empyrean/docs/OVERSEER-PROTOCOL.md` if you haven't. Work the queue. Peers may or
> may not be running — check `ListAgents`; coordinate if present, proceed solo if not.

The shared protocol (role, delegation discipline, review bars, landing-lane rules,
peer protocol) is `empyrean/docs/OVERSEER-PROTOCOL.md`. **Read it at a committed
revision, never through the sibling directory path:**

```sh
git -C ../empyrean fetch origin main -q && git -C ../empyrean show origin/main:docs/OVERSEER-PROTOCOL.md
```

`../empyrean/` is the empyrean overseer's **live working tree** on this shared
machine, so the plain path serves whatever is on disk mid-edit — including
uncommitted or unpushed rules no other lane can see. This is not hypothetical: on
2026-08-22 this session booted from that path while empyrean held sixteen unpushed
commits, and spent the morning enforcing bars that existed nowhere but one peer's
directory. It looked identical to a correct boot, because nothing fetches, nothing
fails, and a lane that booted on a mid-edit tree goes on to cite it *correctly* —
sound citation discipline over a bad source yields a more convincing artifact, not
a less convincing one. Reading at `origin/main` makes a peer's mid-edit tree
unreachable by construction rather than by their happening to be at a quiet point,
and makes "which revision am I running on" answerable, which the path form does not.
Known cost, accepted: `origin/main` can lag a peer's legitimately pushed work — that
trades an invisible failure for a visible one. This is the **recovery** direction
(reach a known artifact), so it wants freshly-fetched tip, never a pinning revision.

This file adds only what is
seraph-specific. **Seraph is usually the least-concurrent repo in the suite — assume
solo. Everything below is executable by a lone session with no peers up.**

## What seraph is

The suite's DAW: a Tauri 2 app (Rust backend in `src-tauri/`, React 19 + Vite
frontend in `src/`) authoring music/SFX for the **Memra** driver (aeon's Z80 sound
driver — "Memra" is the docs/UI name). Standing decisions: driver-in-the-loop
guarantee; Aeon-native model, wide import, narrow export; capability manifest lives
in **empyrean**; correct-by-construction UI over export-time validation.

## Queue — plan of record

Canonical status record: `docs/superpowers/2026-07-03-seraph-banking-queue.md`
(read its Log and EXECUTION HANDOFF sections in full before dispatching anything).
The **MEMRA execution sessions S0→S6** are the banked plan of record — each has a
cold-executable plan under `docs/superpowers/plans/2026-07-03-s*.md`, each
self-contained (paths, code, commands, gates).

Current state (2026-08-19):

- **S0 is PARKED** (2026-07-15 ruling) and it gates S1 Tasks 2–8 and everything
  downstream. Park condition: aeon's sound format was still moving (MEV changes,
  the sigil DSM migration, `sound_constants.asm` on the .emp conversion path).
- **S0 UNPARKED 2026-08-19** (aeon overseer ruling, verified firsthand at aeon
  `236c306b` — full ruling, verification, and three standing caveats in the queue
  doc's Log entry for that date). S0 is READY TO EXECUTE; opening the execution
  session is an owner call. Do not re-derive the S0 plan — it's banked; its
  aeon-facing inputs must be re-grounded against `sound_constants.emp` + the
  `emit_sound_blob` contract at the pinned SHA (the plan predates the asm→.emp
  move), parsing constants from source at use time — never transcribing them
  into seraph-side constants that can drift.
- Done so far: S1 Task 1 (tauri-specta, merged `437841e`); instrument library
  shipped independently of the S-queue (merged `1799c3c`, deferrals listed in the
  queue Log entry for 2026-07-16).
- Engine dependencies (aeon packages 1/2/5) gate only feature-flag flips and S6's
  full header form; S0–S4 target the SHIPPED driver with unshipped features marked
  `reserved` in the manifest.

## Verification lanes

All three must be green before any merge to main; run them on the merged tree,
not just branch-side:

- **Rust:** `cargo test` in `src-tauri/` (~180 tests as of 07-16; treat the count
  as monotonic — aggregate totals with failing names, never a tail).
- **Frontend types + bundle:** `npm run build` (tsc + vite).
- **Frontend tests:** `npm test` (vitest run; RTL infra exists since the library
  work).
- **Bindings drift check:** `src/bindings.ts` is generated from the specta-annotated
  commands (regeneration lives in `src-tauri/src/lib.rs`; a parity guard test in
  `model/instrument.rs` catches serde-vs-specta divergence). After the Rust lane,
  `git status` must show no unexpected `src/bindings.ts` diff — if it does, the
  agent changed IPC types and must commit the regenerated file with them.

Repo-specific quality bars (each has caught a real defect here):

- **Verify rendered audio, never register proxies.** The FM preview played every
  voice wrong for months because two register tables disagreed
  (`OP_REG_OFFSETS` vs `PACKED_OP_SLOTS`, now one authority in
  `src-tauri/src/model/instrument.rs`); a register-level check would have passed.
- **By-ear passes are USER gates** (S3 Task 5, S6 Task 4, and any new audible
  surface). PARK for the owner; everything else is mechanical against written gates.
- **Byte-parity gates are against artifacts**, not arithmetic: S1's compiler gate is
  byte-parity vs `song_packer` output; S6's is `sfx_transcode` byte-parity.
- **Two-stage review** on substantive parcels: spec-compliance pass, then quality
  pass (this is how S1 Task 1 landed).

## Worktree / environment quirks

- **Wayland/WebKit:** the app crashes with Wayland Error 71 unless
  `WEBKIT_DISABLE_COMPOSITING_MODE=1`. `main.rs` sets it automatically since
  `aa08151` — but a worktree checked out at an older SHA needs it in the
  environment. Any "app dies instantly on launch" report from an agent: check this
  first before reading it as signal.
- **Fresh worktrees have no `node_modules`** — frontend lanes need `npm install`
  first (Rust lanes don't). First `cargo build` per worktree is slow (own target
  dir); that's expected, not a hang.
- **`cargo run` runs the app** (`default-run = "seraph"`); the extraction CLI is
  the separate `extract_library` bin (`cargo run --bin extract_library`).
  Extraction is idempotent (sha256 content-hash identity) — safe to re-run.
- **Oracle emulator: ONE instance, controller-session only — never in subagents.**
  (Standing suite norm; it bound every banked plan.)
- **Sound-driver builds on the aeon side** need `SOUND_DRIVER_ENABLED=1 DEBUG=1`
  (+ `SOUND_DEBUG_HOTKEYS=1` for sound testing) — relevant when a seraph lane
  rebuilds aeon to produce blob/manifest inputs.
- Commit exact paths, never `-A`; never leave main broken; no Claude co-author
  trailers on commits.

## Cross-repo coordination point — aeon (and sigil) sound coupling

Seraph↔aeon sound work coordinates through **aeon's sound constants and the
`emit_sound_blob` contract**, and that coupling has tightened since the queue was
banked: the constants are now `aeon/engine/sound/sound_constants.emp` (.emp, owned
by the sigil migration) and the blob is emitted by **sigil's** `emit_sound_blob`
release binary (aeon's `SIGIL_EMIT`). Concretely, seraph consumes: the capability
manifest (generated from aeon source, released to **empyrean**), and — for S3 —
the driver blob + symbol artifact aeon's build exports.

**Before landing anything that pins, parses, or re-releases those artifacts
(S0 manifest work, S3 blob/symbol pinning, any manifest re-release after an aeon
package lands), check `ListAgents` and ping the aeon session if one is up** — it
may be mid-flight on the very format you're freezing. If no peer is up, ground
against aeon's committed HEAD (a clean checkout of the SHA, per the protocol's
landing rules) and record the SHA you grounded against in the queue Log.

## Landing lane

Seraph lands solo: merge → run all three verification lanes on the merged tree →
push. The one cross-repo landing is a **manifest re-release to empyrean** (after
any aeon engine package lands: regenerate via `gen_capability_manifest.py` — a
tool S0 *creates*, it does not exist until S0 executes — re-release, flip feature
statuses in the curated overlay) — that touches two repos and follows the
protocol's one-owner lane rule. Update the queue doc's Log on every landing; it is the canonical
record a cold session boots from.
