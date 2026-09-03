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

## Two committed files the contract requires, beyond the queue

`docs/decisions.jsonl` and `docs/lane-log.jsonl` are required and committed, per
`contract/DECISIONS.md` and `contract/LANE_LOG.md` in empyrean, ratified 2026-08-23
and read here at `origin/main` = `94ea23982df466c04b125e44cf6513a3267741ee`
(`ls-remote`-verified). **Those documents govern; this is a pointer, not a copy.**

Why it needs saying here at all: that rule reaches a lane only through the
`/overseer` skill, which lives outside every repo and outside version control — a
gap `DECISIONS.md`'s own Status section names as open and parked with the owner. So
a cold boot that reads only this file would never learn either file exists.

- **`docs/decisions.jsonl` EXISTS here** (seeded `9cdbb4a`, four entries `d-1`..`d-4`).
  Append-only. Every `blockedOnOwner` row in `docs/lane-status.json` carries a
  matching `id` pointing at it. Correct an entry by appending one with `supersedes`,
  never by rewriting the line. Drop the blocker when the decision is settled and not
  before — that disappearance is the only receipt the console ever gets.
  **Closing an answered one has exactly one legal shape, `DECISIONS.md` rule 8c**
  (empyrean `829d3ac`, located with `git log -S` over the file and confirmed as the
  carrier, not merely the tip it also happens to be): append an entry with `supersedes`
  set to the settled id, carrying the **identical** `question`, `options` and
  `recommend`, and what he chose plus what you did in `detail`. No card re-renders,
  because the blocker is already gone. This repo's `d-5` is the worked example.
  **SUPERSEDED IN PART, 2026-08-30: the `answered` field LANDED and this paragraph's
  prohibition is now stale for it.** The text above still holds for an invented key; what
  changed is that `answered` is no longer invented. `DECISIONS.md` **rule 8d** (read at
  empyrean `origin/main`, verified in force from 2026-08-30T01:58:05Z, Dominion's reader
  parsing it at dominion `7a8a9b3`) makes it first-class, on the SAME appended entry 8c
  already requires: `{"at","by","chose","said","did"}`, where `by` is `owner` | `hub` |
  `lane`, `chose` is validated against the entry's own option keys and is `null` when he
  answered freehand, `said` quotes his words and is never linted, and `did` is one
  sentence of lane prose. **A malformed `answered` costs the field, not the line.**
  Still true and load-bearing: any OTHER invented top-level key is dropped silently by a
  consumer that rebuilds from a fixed key set, so `outcome` remains barred.
  **Two things 8d does not do.** It does not make an entry settled — settled-ness still
  derives from the live `blockedOnOwner` list under rule 9, so an entry carrying
  `answered` that a blocker still claims renders as an OPEN card with a marker. And it is
  **not a licence to backfill**: entries closed before 8d stay exactly as written, this
  repo's `d-5` among them.
  **This repo's worked example of the new shape is `d-11`** (F37, the silent drum entry),
  which is also the first entry here filed with its answer already recorded rather than as
  an open card — legitimate because no blocker was ever owed on it: it was ruled by the hub
  in the owner's place under his standing delegation, with `by: "hub"` saying exactly that.
  When you file one of those, put the provenance and its overturnable-on-read-back status in
  `detail`, and leave `said` off — he did not speak, and quoting a peer's reasoning into a
  field defined as his words would forge one.
- **`docs/lane-log.jsonl` EXISTS here** — opened at F28's landing (`639a68b`, 2026-08-29),
  which is exactly where the rule said it should open. **The paragraph this replaces said
  the file did not exist and that its absence was correct; that was true when written and
  had been false for a day when a boot read it.** Kept as a worked example of this file's
  own top-of-section warning: a snapshot ages while the thing it describes moves.
  `contract/LANE_LOG.md` **rule 8**: open the log at your next landing, never backfill.
  A lane that has landed nothing writes nothing — the file records landings, it is not a
  heartbeat. **Anchor: empyrean `2e50643`**,
  the commit that actually adds rule 8 to `LANE_LOG.md` (`--stat`-checked; the hub cited
  the branch tip `2c587f2`, which carries rule 10 and does not touch that file — a path
  has a time, so cite the commit that carries the rule, not the tip that can read it).
- **Appending to either `.jsonl` has a RECIPE and it is not `>>` alone.** `LANE_LOG.md`
  **rule 7** and `DECISIONS.md` **rule 8**, anchor empyrean `a7b91b4d`
  (`-S 'has a recipe'`-located, verified an ancestor of `origin/main` here): heal a missing
  final newline first, append one newline-terminated line, then parse the WHOLE file — a
  parse failure is a stop, not a note. Read the recipe there rather than from any copy; the
  hazard is that a file not ending in `0a` glues the new record onto the last one and
  destroys BOTH, in a file whose contract forbids rewriting, with nothing in the format
  detecting it. **Measured here 2026-08-30, between 10:13Z and 10:19Z:
  `decisions.jsonl` 12 lines, `lane-log.jsonl` 21 lines, both ending `0a`, every line
  parsing.** Re-measure rather than trusting that line.
  *(The first version of this sentence said `09:5xZ`. That was written from my head, anchored
  to a clock read taken about twenty minutes earlier at boot, and it is the exact defect the
  status contract names for `updatedAt` arriving in a place nobody thought to apply the rule.
  The bounds above are evidence rather than a second guess: the measurement ran in the same
  tool batch that verified `a7b91b4d`, so it cannot precede that commit's 10:13:10Z, and the
  next real clock read was 10:18:46Z. A timestamp is the field that lets a reader discount
  everything else, so a fabricated one is worse than an absent one.)*

- `at` and `updatedAt` come from `date -u +%Y-%m-%dT%H:%M:%SZ` and nowhere else.
- No em dashes or en dashes in either file's prose fields (standing owner
  instruction, 2026-08-23). Note the contrast with this document, which is written
  for a lane rather than for him.

## Standing owner rulings (newest first; the queue Log carries the full entry)

- **2026-09-03T05:21:01Z — REPORT TO THE HUB WHENEVER YOU FINISH WORK OR STOP.** Owner,
  verbatim, banked by the hub at empyrean `f04afe3` (reachable from `origin/main`, verified
  here 2026-09-03; `git -C ../empyrean show f04afe3:docs/OVERSEER.md | grep -n 'loosk like aeon'`):
  *"tell the agents any time theyy finish work or stop to report to you please, loosk like
  aeon's stopped right now"*. Applied here: whenever a landing, a boundary, a block, an owner
  question, or an agent returning leaves this lane with nothing running, send the hub
  (address it by repo, empyrean) ONE message: what landed (SHA from git output, never typed),
  or why you stopped, and what you need. Relayed by the hub, verified at the commit; the
  granting act was not witnessed here.
- **2026-09-02T20:36:03Z — SERAPH IS HELD UNTIL THE ENGINE IS FURTHER ALONG.** Owner,
  verbatim, banked by the hub at empyrean `0689c55` (reachable from `origin/main`, verified
  here 2026-09-03): *"Seraph on hold till we get further with engine."* This supersedes the
  2026-08-30 hold (which rested on seraph being outside the effects project) with a hold in
  his own words and a named trigger: engine progress, his call when. d-9 (S0's driver list)
  is parked with it and is not owed until he lifts the hold. Nothing dispatches; F50 stays
  the front row for the day it lifts. Relayed by the hub, verified at the commit; the
  granting act was not witnessed here.

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
  **CLOSED 2026-08-24 by owner ruling (d-4/d-5): the Draw toggle's legibility and
  paint-drag audibility are settled "leave both alone until they annoy you in real
  use". Do not re-raise them.** But note what that ruling did NOT cover: the queue
  Log's original F6 entry names a **third** gate, whether a painted run RENDERS
  correctly (jsdom has no 2D context, so no test can see it), and this list only ever
  carried two of the three. The card inherited the omission, so his answer cannot be
  stretched to it — it stays open and unfiled. This is the snapshot-versus-Log drift
  this file warns about at the top, caught in this file itself. Also unresolved from F6 —
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
  **Whether an instance EXISTS is a separate question from whether this lane needs
  one, and the guidance above is unchanged either way** *(2026-08-24)*. The hub
  relays that a live oracle server has been up since 2026-08-19 with its socket in a
  private `XDG_RUNTIME_DIR`, invisible to every lane — **their measurement, relayed,
  NOT verified here**, deliberately, because nothing in this queue depends on it and
  verifying a shared instance means touching a resource another lane may be
  mid-measurement on. Recorded only so a future session at S3 does not spend an hour
  concluding there is no emulator: re-check it then, firsthand, rather than trusting
  this line. Note it also predates the cutover, so the standing one-instance norm
  applies to whoever next starts one.
  **Superseded 2026-08-26, relayed by the hub, NOT verified here:** the suite switched to
  the new Oracle (owner ruling, empyrean `3c21183`); a fresh session's `mcp__oracle__*`
  tools now spawn a private oracle-aether (aeon's `s4.debug.bin`, starts paused) and reap
  it on exit rather than dialling the shared socket; `ORACLE_SOCKET=/run/user/1000/oracle.sock`
  attaches to the window the owner watches; do not open `oracle_gui`. Same guidance as
  above: nothing here needs it before S3; re-check firsthand then.
  **CORRECTED 2026-08-26, VERIFIED FIRSTHAND HERE, and the paragraph above is DANGEROUS
  if read alone.** "A fresh session spawns its own emulator" is a property of the **MCP
  shim process**, not of the conversation, and **`/clear` does not restart that process** —
  so a session can be conversationally fresh and still be running a shim from before the
  cutover, which **dials `/run/user/1000/oracle.sock`: the owner's on-screen player**. Any
  `mcp__oracle__*` call from such a session pauses, steps or writes into the game he is
  playing. Found by oracle, corroborated by the hub (`ps` + `ss -xp`); the part verified
  **here** is the one neither of them could check, this session's own shim: PID 287652, a
  child of this `claude` process, started **2026-08-26 00:29:19Z** with **no child
  `oracle-aether`**, while `oracle-old` `07314aa` ("the shim opens its own emulator on
  first use") is dated **2026-08-26 01:09:53Z** — **40 minutes later**. (Both normalised to
  UTC; this machine is EDT and `ps lstart` prints local, so a start time compared against a
  git date straight off the two commands is off by four hours. That mistake was made here
  first and caught by running `date +%z`, which is the whole reason the margin is stated as
  a duration rather than left as two timestamps a reader would have to reconcile.)
  **A 40-minute margin is why start-time is the WRONG discriminator** and the child-process
  check below is the right one: every shim on this machine started within about a day of
  that commit, so eyeballing a clock misclassifies in both directions. This conversation was
  `/clear`ed the following morning and the shim did not move, which makes this lane its own
  worked example.
  **How to tell which kind of session you are, and do it BEFORE the first `mcp__oracle__*`
  call, because the call itself is the harm.** Walk your own ancestry to the `claude` PID,
  then look for a paired emulator:

  ```sh
  ps -eo pid,ppid,lstart,args | awk '$2=='"$CLAUDE_PID"''   # the shim is a child of claude
  ps -eo pid,ppid,args | awk '$2=='"$SHIM_PID"''            # a post-cutover shim has an oracle-aether child
  ```

  A shim with its own `oracle-aether` child is safe; one without it is a dialer. **A shim
  with no live connection right now proves nothing** — it connects on first use, so an empty
  `ss -xp` is bar 16(d)'s ambiguous absence, not an all-clear. Remedy is a Dominion
  clear+reboot (which respawns the MCP process) or your own emulator with `ORACLE_SOCKET`
  pointed at it. **Never kill or reset his player.** Costless for this lane either way,
  since nothing here needs an emulator before S3 — recorded so the session that opens S3
  does not learn it by pausing his game.
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
  **FIXED as of F45 (2026-08-30): `npm test` output is now the same for agents and humans,
  and console output from PASSING tests is visible again.** `vitest.config.ts` pins
  `reporters: ["default"]`, so nothing about the reporter depends on the environment.
  **Do not use `--reporter=verbose` to see warnings any more — plain `npm test` shows them.**
  *The defect this replaced, kept because the mechanism recurs:* vitest 4 picks its reporter
  as `isAgent ? "agent" : "default"`, and std-env's `isAgent` is true whenever `CLAUDECODE`
  or `AI_AGENT` is set, which is every Claude session. `"agent"` is an **alias for
  `MinimalReporter`**, which hard-codes `silent: "passed-only"`; a config-level
  `silent: false` does **not** override it, because the reporter passes its own `silent` and
  the config value is only a `??=` fallback. Reporter *options* do override it
  (`[["agent", { silent: false }]]` works), but an explicit pin was chosen instead so that
  agents and humans see identical output and a fully-skipped file is still distinguishable
  from a passing one. Reproduced firsthand on 4.1.10 here; aurora reproduced the same
  mechanism independently on 4.1.4.
  **A guard test fails if the pin is ever removed or pointed back at `agent`/`minimal`:**
  `src/test/reporterPin.test.ts` (proven to fire in both directions).
  **The summary lines are unchanged in shape** — ` Test Files  N passed (N)` and
  `      Tests  N passed (N)` still print exactly as before, so any landing procedure that
  greps them is unaffected. Pass/fail counts and failing names were never affected by the
  reporter; it was only *warning* claims that were untrustworthy. `npm run build` was never
  affected — that is vite and tsc, not vitest.
  **Cost, measured on the full suite:** pinning `default` adds ~105 lines to a run that was
  already 1250 lines, ~8%. Of those, ~36 are the per-file `✓` listing and ~65 are one
  benign message repeated 13 times (see the jsdom/close-confirm note below).
  **A full `npm test` here is ~99% jsdom noise:** 1237 of 1250 baseline lines are
  `Not implemented: HTMLCanvasElement's getContext()`. That text bypasses the vitest
  reporter entirely (jsdom's own virtual console), so it was never hidden and is not
  something the reporter change introduced — but it is the actual token cost of reading a
  test log here, and it dwarfs everything the reporter debate was about. Booked as F46.
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
- **Converting a process start time to UTC: the obvious correction FAILS SILENTLY, in both
  spellings** *(measured here 2026-08-30 while answering a shared-machine timing question from
  another lane; I made the error twice in consecutive commands)*. This machine is **EDT
  (UTC-4)** and `ps lstart` prints **local**, which this file already warned about. The part
  that was missing is the repair: `TZ=UTC date -d '<local string>'` parses a local string as
  UTC, and **`date -u -d '<local string>'` does too**, because `-u` applies to *parsing* as
  well as output. Both hand back the number you fed them, so the conversion looks performed
  and does nothing — a four-hour error wearing a normalisation's clothes.
  **Use a timezone-free instrument:** `ps -o etimes=` (elapsed seconds; subtract from
  `date -u +%s`), or put an explicit zone token in the string
  (`date -u -d 'Sun Aug 30 08:38:09 2026 EDT'`). Measured agreement of the two correct forms
  against one live pid: **12:38:09/10Z**, where both naive forms said `08:38:09Z`.
  **Why it is worth a bullet rather than a footnote:** a timing answer is used to include or
  exclude a lane as the cause of something, so a silent four-hour skew turns a true negative
  about the wrong minutes into an alibi or an accusation. Same family as the `ls` probe above
  and as protocol bar 16(d): a command that quietly answers a different question than the one
  asked, and returns something that looks like an answer.

- **`ls <path>` IS A BROKEN EXISTENCE PROBE IN THIS WORKSPACE, and it fails identically
  for paths that exist and paths that do not** *(relayed by the aurora lane 2026-08-30 as a
  hazard notice, anchors empyrean `e159721850d77a64081ad577b3ac1890e5476a2a` and sigil
  `2aa3e0f5`, both reachable and both docs commits — the correct class, since what they carry
  is this text; **reproduced firsthand here**, including the boundary the relay did not state)*.
  `ls` is aliased to `eza --color=always --group-directories-first --icons`, the alias reaches
  **non-interactive Bash tool calls** (the shell is initialised from the profile), and
  `--icons` takes an OPTIONAL value, so it swallows the next argument.
  **The failure is exactly the flagless `ls <path>` spelling — which is exactly the spelling a
  probe uses.** Measured here, all six cases: `ls <file>` exit **2**, `ls <dir>` exit **2**,
  `ls` with no argument exit **0**, `ls -l <file>` exit **0**, `ls -la <dir>` exit **0**,
  `/usr/bin/ls <file>` exit **0**. A leading dash-flag protects everything after it because an
  optional value will not consume a token starting with `-`; that is why this survived
  unnoticed — every ordinary listing works, and only the probe form breaks.
  **What makes it dangerous rather than annoying: the existing path and the missing path
  produce BYTE-IDENTICAL output and the same exit 2.** So
  `ls "$P" >/dev/null 2>&1 && echo present || echo absent` prints `absent` for **everything**,
  and no artifact anywhere in the run says otherwise. It cost the hub a false
  *"freeze completed"* the same day.
  **Use `[ -f "$P" ]`, `[ -e "$P" ]`, `stat`, `git ls-files`, or `/usr/bin/ls`** — and never
  write `ls <path>` into a dispatch brief.
  This is protocol **bar 16(d)** landing in this repo's own shell: an absence with nothing to
  be suspicious of. The bar's standing corrective applies unchanged and is the cheap half —
  **when a probe says ABSENT or FAILED, run it against something known to exist before
  believing it.** Note the protocol's own aeon instance is the WEAKER version of this
  (`ls -t docs/superpowers/`, where the rejected token was a flag); the flagless form
  documented here fails with no flag to blame, which is why it reads as a clean answer.

- **Fresh worktrees have no `node_modules`** — frontend lanes need `npm install`
  first (Rust lanes don't). First `cargo build` per worktree is slow (own target
  dir); that's expected, not a hang.
- **`cargo run` PUTS A WINDOW ON THE OWNER'S SCREEN. There is no headless display on this
  box, so treat launching the app as an act with an audience** *(banked 2026-08-30; the
  environment half verified firsthand here, the toolkit half relayed from the aurora lane,
  their measurement at aurora `3c1639f2`, not reproduced here — reproducing it means
  launching a window, which is the hazard)*. Measured in this session: `WAYLAND_DISPLAY` is
  `wayland-0` with its socket live at `/run/user/1000/wayland-0`, `GDK_BACKEND` is **unset**,
  and **`DISPLAY` is `:0`** — the owner's session, not an Xvfb.
  Aurora's half, relayed: a toolkit that prefers Wayland lands on the owner's compositor
  **even with `DISPLAY` pointed at a fresh Xvfb**, and unsetting `WAYLAND_DISPLAY` does not
  save you (it falls back to the literal `wayland-0`). Tauri on WebKitGTK follows
  `GDK_BACKEND`'s default, so a harness that wants isolation must set **`GDK_BACKEND=x11`
  explicitly** and then **verify the screen size from inside the app** before trusting any
  pixel measurement.
  **Do not run the windowed case while he is logged in.** Nothing in this repo's test suites
  launches the window today — but the line immediately below documents `cargo run` as the
  normal way to start the app, so this is one command away rather than hypothetical, and any
  session following this file could do it without meaning to.
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
