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

> **THIS SECTION IS A SNAPSHOT AND THE LOG IS NOT — when they disagree, the Log
> wins.** A boot doc ages while the queue's Log accumulates, so the one file every
> cold session reads *first* is the one most likely to hand it pre-ruling state and
> present it as current. This is not hypothetical: on 2026-08-22 a session booted
> from the text below, reported S0 to a peer as an open owner call, and had it
> escalated to the owner — who had **already ruled on it the day before**. The same
> session separately dispatched a parcel for a finding the owner had deprioritized.
> Both rulings sat mid-Log; both boots read the Log's head and tail only.
> **Before acting on any status below, and before funding any parcel off the
> DAW-feel audit, grep the Log for the identifier:**
>
> ```sh
> grep -nE "S0|F15|DEPRIORITIZED|owner ruling" docs/superpowers/2026-07-03-seraph-banking-queue.md
> ```
>
> A severity number is a property of the CODE; a hold or a deprioritization is a
> property of the OWNER's intent, and only the second can make a ready or critical
> item not-next. Nothing in a status line or a findings table can tell you which
> applies.

Current state (last reconciled against the Log 2026-08-22):

- **S0 is PARKED** (2026-07-15 ruling) and it gates S1 Tasks 2–8 and everything
  downstream. Park condition: aeon's sound format was still moving (MEV changes,
  the sigil DSM migration, `sound_constants.asm` on the .emp conversion path).
- **S0 UNPARKED 2026-08-19** (aeon overseer ruling, verified firsthand at aeon
  `236c306b` — full ruling, verification, and three standing caveats in the queue
  doc's Log entry for that date). Technically READY TO EXECUTE. Do not re-derive
  the S0 plan — it's banked; its
  aeon-facing inputs must be re-grounded against `sound_constants.emp` + the
  `emit_sound_blob` contract at the pinned SHA (the plan predates the asm→.emp
  move), parsing constants from source at use time — never transcribing them
  into seraph-side constants that can drift.
- **S0 IS OPEN AGAIN (2026-08-22) AND RETARGETED — AND THE BANKED S0 PLAN NO
  LONGER DESCRIBES THE TASK. Read this before opening `plans/2026-07-03-s0-memra-contract.md`.**
  The owner reportedly re-opened S0 while redirecting it **off Memra**:
  *"We can start this but like not with memra engine yet I don't think, maybe s2
  clone driver, zyrinx driver, flamewing driver, then like s1/s2/s3k driver?"*
  **That plan and `specs/2026-07-03-s0-memra-contract-design.md` are both written
  Memra-first**, so designing the manifest against four established drivers is
  real design work, not a parameter swap — and S0 is now materially LARGER than
  when it was banked.
  **Both the re-opening and the retarget reached this repo RELAYED through the
  empyrean lane; no seraph session witnessed the granting act.** The register was
  hedged (*"I don't think"*, ending in a question mark) — direction with the
  reasoning open. **So: get this ruling first-hand from the owner before
  designing to it, and do not treat this bullet or the queue Log entry as the
  authority.** Full entry, with the argument for the retarget attributed to the
  lane that made it rather than to the owner, is in the queue Log for 2026-08-22.
  Sequencing he *did* give directly: **F25 + F26 first, then S0.**
- **S0 HELD BY THE OWNER, 2026-08-21 — superseded by the re-opening above, kept
  because it is why the re-offer had to carry its history.** A session offered to
  open the unparked S0; the owner chose
  *"hold S0, other work"* and named the gap he wanted closed instead (no way to
  create a measure and write music). That gap was closed the same session
  (compose-from-scratch path, merged `3c6ee0d`), and most of the work he
  redirected the lane to has since landed. **So S0 is re-offerable — but as a
  REVISIT carrying its history ("you held this on 08-21 because X; X is now
  closed"), never as a fresh open decision.** Re-asking an answered question
  reads to the owner as nobody having recorded his answer.
- **F15 (view-state persistence) is DEPRIORITIZED by owner ruling** (2026-08-21,
  "current behavior matches how they work") despite still being severity-critical
  and rank #3 in the audit. Branch `feat/view-state-persistence` holds an
  erroneously-dispatched, unreviewed partial implementation — preserved unmerged
  in case the ruling reverses; it must not be landed on the strength of a status
  line.

### Live state a fresh boot needs (2026-08-22)

- **Nothing is in flight.** Every dispatched agent's work is landed, verified on
  the merged tree, and pushed; `origin/main` was `ls-remote`-verified at each
  push. The one preserved branch is `feat/view-state-persistence` (F15,
  deprioritized — see above), unreviewed and not to be landed without a ruling.
- **Last merged-tree lanes** (at the F25/F26 landing): cargo **264/0**, vitest
  **352/352 across 33 files**, `npm run build` clean with zero warnings, no
  `src/bindings.ts` drift. All exit codes read from the runner directly, never
  through a pipe.
- **Owner gates OPEN — these need ears and a fresh session cannot close them.**
  From F25/F26: does picking a sample in the DAC lane's new header picker sound
  right, immediately? Can a kick/hat/snare kit built from scratch on one DAC lane
  be played back with the per-note samples as authored? Does auditioning *feel*
  faster while painting a run across rows (the half of F26 no test can report)?
  Still open from F6: the Draw toggle's legibility, and whether a paint-drag over
  a running transport is audible at audition loudness. Also unresolved from F6 —
  in Draw Mode a left-click on an existing note **selects** rather than deletes,
  a deliberate deviation from the banked Ableton ruling, ratified provisionally
  as a data-loss guard and reversible in one line.

- **Emulator cutover costs this lane nothing.** Seraph's audio path is its own
  Rust engine, not an emulated ROM, so there is **no oracle/emulator dependency
  today** — current measurement work uses the in-repo `rendered_rms` /
  `live_edit_audibility` harnesses. Do not go looking for one. The ask becomes
  real only at **S3** (driver-in-the-loop), which wants per-channel audio
  isolation, driver-state readout at Memra's symbols, and deterministic VGM
  capture hooks from oracle. Forward notice only — nothing is owed and nothing
  should be scheduled until S3 opens.
- **KNOWN FLAKE, unidentified.** The vitest suite failed **1 test in 1 of ~6 full
  runs**; five runs since were clean at 336/336. **The name was not recovered**,
  because that run was piped through `tail -20` and the `FAIL` lines were
  discarded. Per this repo's standing rule a flake gets made deterministic, not
  watch-listed — so if you see it, **capture the full output** and kill it.
  Two transferable lessons from that command: `npm test | tail` reports **tail's**
  exit status (it exited 0 with a test failing), and truncating a log is the same
  defect class as `2>/dev/null` — it destroys the artifact that would name the
  problem.
- **BOOKED PRODUCT DEFECT — `$2B` / DAC-vs-FM6 divergence.** Seraph's preview
  engine keeps the DAC as an independent stream summed into the mix and **never
  writes register `$2B`**, so an FM6 track and a DAC track sound together in the
  app and **cannot** on real hardware. Verified firsthand with a control grep
  (`0x2b`/`$2b` → exit 1 across `audio/`, `sequencer/`, `dac/`; control `0x28` →
  9 hits in `engine.rs`). Same class as the overlap last-note-priority fix that
  already landed: the preview must not promise what the driver cannot do. Now
  booked as audit finding **F27** (severity high, unsized) as of `d668619`; the
  fix is a design call — silent steal, a visible warning, or an authoring-time
  gate like `check_voice_overlap` — and **the driver's exact behaviour should be
  confirmed against aeon rather than assumed.**
- **Seven more booked defects** from the README pass (dead `export_vgm` UI path,
  hardcoded 60 s WAV export, unreachable `extract_library --help`, phantom
  `"binary"` export format, unimplemented `Fm3SpecialMode`, a stale "Draw Mode
  (F6)" comment where the binding is `B`, uncalled `get_channel_overlaps`) — all
  in the queue Log for 2026-08-22 with evidence, none fixed.
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
