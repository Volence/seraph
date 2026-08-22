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
  ruling (ANCHORED aeon-side: `0062e8ce`, docs/DEFERRED_WORK.md §"Seraph
  coupling anchor" — verified to match this transcription, which closes the
  transcription-not-quotation flag): (1) aeon sound packages 5/6 are
  still open, and the 2026-08-13 sound-lens sweep has an unmerged packet (two
  live findings: multi-slot SFX cap; a DAC/DMA wedge class) — any MEV/constants
  change from that work arrives as explicit notice BEFORE landing, contract-style.
  (2) The MEV_EXT registry invariants (slots 0/1/2) are load-bearing aeon-side —
  extending the registry is a cross-repo ask via the demand-doc flow, never a
  unilateral read. (3) aeon's streaming arc may later couple to the driver's
  DMA-survival design (max-contiguous-DMA-stall) — if S0 work touches DAC/DMA
  timing assumptions, notify aeon and coordinate. S0 is now READY TO EXECUTE;
  actually opening the execution session is an owner call.

- 2026-08-21: **S0 HELD (owner ruling)** — offered to open the unparked S0; owner
  chose "hold S0, other work" and reported a major gap: no way to create a measure
  to write music. **COMPOSE-FROM-SCRATCH PATH SHIPPED** same session (merged
  `3c6ee0d`; lanes on merged tree: cargo 214/0, vitest 18/18, build clean, zero
  bindings drift). Research finding (verified firsthand): the gap was pure UI
  omission — model (`Region`) + IPC (`add_track`/`add_region`/`add_note`) fully
  supported composing, but the frontend had ZERO call sites for addTrack/addRegion
  (`AddTrackDialog` existed since `14c1fa4`, never mounted), `create` seeded no
  tracks, and the `+ FM/+ PSG/+ DAC` buttons added instruments while claiming to
  add tracks. Shipped: (1) create seeds one instrumentless track per driver
  channel, derived from `DriverProfile::channel_layout()` (owner ruled: seed full
  roster); instrument add now binds to the lowest empty lane of its kind and
  delete unbinds (lane + regions survive — also removes a latent data-loss path);
  (2) `+ Track` mounts AddTrackDialog; buttons renamed `+ FM Patch`/`+ PSG Env`/
  `+ DAC Sample`; (3) empty-lane double-click creates a bar-snapped one-bar
  region, auto-selects (piano roll opens), reloadSequence for audibility;
  (4) `src/utils/grid.ts:ticksPerBar(meta)` is the ONE derived bar-length seam
  (PianoRoll/useArrangementZoom/region default de-hardcoded from 480×4) — S1's
  tick-native grid lands there. S4 checked: its noun is *region*; no design fork.
  DEFERRED (booked here): drag-to-length region creation (empty-lane drag is
  marquee-select; UX call for S4); stale lane name after instrument unbind;
  `get_playback_state` returns hardcoded `playing:false/loop:None`
  (commands.rs:950-965); PianoRoll "1/1" snap now means one bar (ratified).
  OWNER GATE OPEN: visual/audible pass — fresh project shows full lane roster,
  + Track works, double-click creates a measure, a placed note plays after
  reload; also confirm empty 11-channel project plays silent/stable.

- 2026-08-21 (cont.): **PIANO ROLL SELECT+TRANSPOSE SHIPPED** (owner-reported gap;
  merged, lanes on merged tree: cargo 214/0, vitest 47/47, build clean, no
  bindings drift). Marquee left-drag select (Shift additive), click selects
  before drag (3px threshold), group drag moves the whole selection
  (interval-clamped), Arrow=±1 semitone / Ctrl+Arrow=±octave transpose
  (block-whole-move at range edge, intervals intact), pan moved to
  middle-button/Alt+drag. Pure helpers in `src/utils/pianoRollEdit.ts`.
  **OWNER RULINGS (banked):** piano-roll mouse grammar = Ableton-style
  (double-click draws, drag marquees, middle/Alt pans); Stop semantics =
  pause-in-place + double-tap returns to playback start + Home=zero; undo v1
  scope = song edits (notes/regions/tracks), instrument knobs excluded.
  **FULL UX AUDIT BANKED** (41 gaps G1–G41, priorities, S4 collision map) —
  see Log context; headline bugs: G1 piano-roll Delete also deletes the region
  (two live window handlers); G2 shortcuts fire while typing in inputs; G3 no
  undo anywhere; G4 no dirty flag/confirm-on-close; G5 velocity lane drawn
  offset by the key-column width; G6 region single-click select possibly
  swallowed (stale closure). Next waves dispatched: A = safety/transport/
  playhead (G1,G2,G5,G6,G28,G22,G29,G30,G37); B = undo/redo + dirty flag
  (G3,G4, new IPC + bindings regen). Remaining backlog for later waves:
  note/region clipboard (G11,G23), loop-region drag + snap control (G25,G26),
  tempo/time-sig editing (G34, needs new IPC), track rename/delete (G27),
  right-click/draw grammar completion per Ableton ruling (G13,G14), QWERTY
  step entry (G36, owner call pending). S4-deferred: per-note detune UI (G20),
  track reorder (G27-reorder), velocity mapping changes.

- 2026-08-21 (cont.): **UNDO/REDO + DIRTY FLAG SHIPPED** (Wave 1 parcel B; merged
  `0fc06d6`; lanes on merged tree: cargo 226/0, vitest 59/59, build clean,
  bindings regenerated + committed). Snapshot-stack in ProjectManager
  (Vec<Track>, MAX_UNDO_DEPTH=100, validate-first so failed mutations push
  nothing), begin/end_undo_group coalescing (one drag/batch = one undo step),
  new IPC: undo/redo/begin_undo_group/end_undo_group/get_undo_state; Ctrl+Z /
  Ctrl+Shift+Z / Ctrl+Y (input-guarded via new src/utils/keyboard.ts), dirty
  dot on Save, confirm before New/Open when dirty, onCloseRequested guard
  (needs live-app confirm — `core:window:allow-destroy` added). Scope per
  owner ruling: song edits only; instrument ops stay out of the stack but do
  mark dirty. DRIVE-BY FIX (ratified): move_region validated destination only
  AFTER removing the region — a bad destination silently lost it; now
  validate-first. Known v1 flags: undo/redo always mark dirty even landing on
  the saved state; a final in-flight drag update can theoretically land after
  end_undo_group (one extra step; flagged, not hidden). Wave 1 parcel A
  (safety/transport/playhead, branch fix/ux-safety-transport) still in flight
  — both Wave 1 agents were killed mid-flight by a session usage limit and
  resumed in their worktrees; parcel B completed post-resume.

- 2026-08-21 (cont.): **WAVE 1 PARCEL A SHIPPED — UX safety/transport/playhead**
  (merged `fc4ffc2` + cross-feature test-mock fix `9b07410`; lanes on merged
  tree: cargo 226/0, vitest 91/91 across 15 files, build clean, no bindings
  drift). Fixed: G1 Delete scoping (piano-roll note selection defers region
  delete via src/utils/noteSelection.ts; Backspace now also deletes notes —
  ratified); G2 input guards on all window-level key handlers (Ctrl+S
  deliberately above the guard — ratified); G5 velocity-lane alignment
  (key-column width lifted + VelocityLane offset, tracks DAC resize); G30
  reloadSequence on region move/resize/delete (commits are mouseup-only, no
  debounce needed); G28/G22 follow-playhead scroll (80%→10% paging,
  2s manual-scroll suspend, arrangement + piano roll); G29 seek-cursor sync
  on stop (seekTick lifted to App, generation counter vs stale syncs; Home
  button same-class fix); G37 stop semantics per owner ruling
  (STOP_DOUBLE_TAP_MS=400, window consumed on use, new play clears pending —
  both ratified). G6 DOES NOT REPRODUCE (React 18 microtask flush; regression
  tests kept) — but the probe found a real adjacent defect, BOOKED DEFERRED:
  after a region drag-move, the browser click reaches onRegionClick and
  re-selects/opens the region (needs a did-drag ref). Landing note: parcel A
  merged after undo/redo — conflict resolution unioned both features in
  App.tsx/ArrangementView/PianoRoll/keyboard.ts; the add/add App.test.tsx was
  split (undo suite keeps App.test.tsx, safety suite → App.safety.test.tsx);
  merged App needed cross-feature IPC mocks in all three App suites
  (`9b07410`). OWNER GATE OPEN (visual/audible, cumulative): velocity bars
  under notes (melodic + DAC resize), live follow-scroll both views, undo
  coalescing feel (one Ctrl+Z per drag), dirty dot + close confirm, stop
  double-tap, marquee + transpose, seeded roster + measure creation.

- 2026-08-21 (cont.): **WAVE 2 PARCEL C SHIPPED — clipboard/nudge/duplicate**
  (merged `f1a5a08`; lanes on merged tree: cargo 228/0, vitest 127/127 across
  16 files, build clean, bindings regenerated+committed). Note Ctrl+C/X/V
  (module clipboard src/utils/clipboard.ts, two slots + lastCopiedKind()
  arbitration since region-open implies region-selected; paste anchored at
  seek cursor when inside the open region else region start; overhanging
  notes clamped, out-of-range skipped with console.warn count); Arrow
  Left/Right nudge by grid step, Ctrl+Arrow = 1 tick (block-whole-move);
  new `duplicate_region` IPC (validate-first, undoable, returns new id);
  region Ctrl+D duplicates after source, region Ctrl+C/V pastes at
  bar-snapped cursor (server duplicate with payload-replay fallback if the
  source was deleted); drag click-through defect FIXED (did-drag one-shot
  ref in TimelineCanvas; the booked wart from the G6 probe). Ratified:
  clipboard arbitration; payload-replay fallback; paste drops note
  detune/modulation (add_note IPC lacks them — same as pre-existing Ctrl+D;
  becomes moot when S4's NoteInspector extends update_note). Parcel D
  (loop/snap/tempo/track-ops, branch feat/loop-snap-tempo) still in flight.

- 2026-08-21 (cont.): **WAVE 2 PARCEL D SHIPPED — loop/snap/tempo/track-ops**
  (merged `f195e32`; lanes on merged tree: cargo 231/0, vitest 153/153 across
  19 files, build clean, no bindings drift). Preview loop: drag the ruler's
  upper half sets a bar-snapped loop range drawn as a bracket (lower half
  keeps scroll/seek; zero-move click = one-bar loop at that bar); `l`/loop
  button re-arm the LAST range (both hardcoded bars-1–4 sites gone; App is
  the single loop owner). Arrangement snap selector Bar/Beat/Off via
  grid.snapUnit (loop drag + region create/move/resize honor it; create
  duration default stays one bar). New `update_project_metadata` IPC
  (tempo 20–300, num 1–16, den 2/4/8/16); TopBar inline tempo/time-sig edit;
  FLAGGED: metadata is outside the undo snapshot — dirty but NOT undoable in
  v1 (commented at the mutation site). Track rename (double-click name) +
  delete (hover ✕ + confirm) in TrackHeader; no reorder (S4 ChannelRack owns
  ordering). All six design choices ratified (ruler split, snap-coupled loop
  min, degenerate-click loop, one-bar create default, 20–300 BPM bounds,
  loop ownership moved to App). **WAVE 2 COMPLETE. UX-basics arc: 6 parcels
  landed today** (compose path, marquee/transpose, undo/dirty,
  safety/transport/playhead, clipboard/nudge/duplicate, loop/snap/tempo).
  Remaining audit backlog (Wave 3 candidates, owner to prioritize): G17
  multi-note velocity paint, G16 vertical zoom, G19 stable note IDs, G21
  region auto-extend on note drag, G36 QWERTY step entry (owner call open),
  G35 metronome, G7 instrument rename/delete UI (dead Sidebar), G31/32 zoom
  polish + wheel-axis fix, G39/40 keymap panel + Ctrl+N/O, G41
  get_playback_state honesty, song-end/loop-point modeling (design Q,
  touches export + SMPS loop points). CUMULATIVE OWNER GATE still open
  (visual/audible pass across all six parcels — checklist in the parcel A
  entry plus: loop bracket + audible loop cycling, snap modes, tempo edit,
  rename/delete, clipboard paste anchor, nudge feel).

- 2026-08-21 (cont.): **WAVE 3 OPENED (owner-directed)** — owner stopped the
  backlog queue with 7 feel/bug items + asked for a deeper feel/use audit.
  **PARCEL F SHIPPED — loop-wrap follow-scroll + live marquee preview**
  (merged `2b3ed88`; lanes on merged tree: cargo 231/0, build clean, vitest
  167/167 ×3 runs, no bindings drift). `followScrollLeft` gains prev-playhead
  param: backward jump (loop wrap/seek-back) with playhead off-view snaps to
  the 10% anchor; no-ops when still visible or when the user had scrolled
  ahead (jitter guard); both views wired via tick refs. Marquee: live preview
  set drives draw() during drag via `marqueeRectFromView`/
  `marqueePreviewSelection` (pianoRollEdit.ts) — commit path now shares the
  same rect helper. RATIFIED: backward-jump rule; preview = WYSIWYG selected
  style (plain drag live-previews deselection too). FLAKE WATCH: one unnamed
  vitest failure (1-of-9 agent-side runs, name lost); 3 merged-tree runs + 8
  agent runs green — unresolved, watch for recurrence.
  **TRANSPORT DIAGNOSIS BANKED (item 7, no code change):** resume==restart
  because `recordPlayStart` fires on EVERY `transportPlay` incl. resumes
  (`src/utils/transportMemory.ts` `lastPlayStartTick`), so the first resume
  discards the original launch point; backend `current_tick` is the single
  resume authority; stop double-tap (400ms) is Space-consumed only. RULING
  PARKED (owner explicitly undecided): proposal = don't re-record play-start
  on a resume (play after pause with no seek between); double-Space then
  returns to the launch/seek point as owner suggested.
  **VOICE-PER-TRACK RESEARCH BANKED (item 5, read-only):** model/playback/
  both exporters already fully support per-note `instrument_id` (3-level
  note>region>track cascade in `build_snapshot`, per-event patch reprogram;
  SMPS emits SetVoice + banks note-level voices; importers stamp it) — the
  gap is IPC (`add_note` hardcodes None; nothing can set it) + UI (only
  track-level swap exists). DEFECTS FOUND: (a) library drag-to-track
  deliberately CLEARS all per-note/region voices (manager.rs
  `assign_library_instrument_to_track` tail) — silently destroys imported
  songs' mid-track voice changes; (b) a second track on the same channel is
  allowed and plays, but SMPS export emits duplicate per-channel labels +
  wrong header count, uncaught by validate_for_export; (c)
  `get_channel_overlaps`/OverlapWarning exists backend-side with ZERO UI call
  sites. Three UX shapes written up (A: set-voice-on-selected-notes, thin,
  S4-compatible down-payment; B: S4 sub-voice lanes as a view over per-note
  ids; C: multi-track-per-channel — recommend AGAINST, collides with S4
  ChannelRack). RECOMMENDATION: Shape A now + drag-wipe confirm; RULING
  PARKED for owner. Overlap prevention belongs at the mutation site
  (validate-first in ProjectManager), per correct-by-construction.
  **AUDITS MERGED:** `docs/superpowers/2026-08-21-daw-comparator-idioms.md`
  (merged `c74cc3b`; Furnace/Deflemask/FL/Ableton vs seraph, 8 scenarios,
  10-idiom adopt/adapt/reject shortlist — headline adopts: linked-region
  reuse + Make Unique, gapless edit-while-looping guarantee, QWERTY musical
  keyboard, audition-on-every-pitch-gesture) and
  `docs/superpowers/2026-08-21-daw-feel-audit.md` (merged `da7304f`;
  scenarios A–G, findings F1–F24, keyboard inventory, ranked top-10,
  15-minute owner play-test script — CRITICAL: F1 note-level edits are
  inaudible while transport runs (PianoRoll has zero reloadSequence; region
  ops got G30 but notes were missed), F2 instrument-less seeded lanes
  silently drop all notes + audition no-ops, F15 zero view-state persistence
  on reopen). STILL IN FLIGHT: ruler parcel (piano-roll ruler, drag-zoom,
  loop handles — items 1/2/6), quiet-voice-151 rendered-audio diagnosis
  (item 3).

- 2026-08-21 (cont.): **WAVE 3 PARCEL E SHIPPED — piano-roll ruler + drag-zoom
  + loop handles** (items 1/2/6; merged `fba0e3b`; lanes on merged tree:
  cargo 231/0, build clean, vitest 211/211 across 23 files, no bindings
  drift). New `PianoRollRuler` (absolute bar numbers consistent with the
  arrangement + "Bars N-M" header, beat ticks by zoom, dimmed past region
  end, click=seek clamped to region, h-drag=scroll); vertical ruler drag
  zooms BOTH rulers (FL convention drag-down=in, anchored at grab x via
  `zoomAroundPixel`); loop bracket gains edge handles (±6px, snap-rounded,
  min one unit) + draggable body (length-preserving) + zone hover cursors.
  Shared pure helpers: `src/utils/rulerMarks.ts` (bar-label thinning,
  8px beat floor), `zoomDrag.ts` (dominant-axis lock, 4px slop, ties
  horizontal), `loopHandles.ts`. All bar math via `ticksPerBar(meta)`;
  red-first tests use non-4/4 meta. RATIFIED (all 7 agent calls):
  dominant-axis lock over simultaneous; drag-down=zoom-in; click ON the
  loop band is a no-op (only outside-band clicks reset a one-unit loop —
  regression-guarded); label thinning; degenerate click = one-SNAP-UNIT
  loop (pre-existing, kept); piano-roll ruler h-drag=scroll/click=seek;
  loop feedback begins after the 4px slop. TAGGED for the owner gate:
  zoom sensitivity (~1%/px), slop, handle width, cursor affordances —
  feel-check in the live app.
  **ITEM-4 RULING REVISED (owner, mid-session):** loop wrap must not move
  the piano-roll view AT ALL — the parcel-F wrap-snap (2b3ed88) is
  OVERTURNED. New rule dispatched to the same lane (branch
  fix/loop-follow-marquee, revision in flight): while a preview loop is
  active, follow-playhead is suppressed entirely in both views; backward
  playhead jumps never scroll, loop or not (followScrollLeft returns to
  forward-only). Marquee preview work stands.

- 2026-08-21 (cont.): **QUIET-VOICE DIAGNOSIS LANDED — item 3 was a seraph
  bug, not the voice** (merged + cross-parcel test fix `468072e`; lanes on
  merged tree: cargo 235/0, vitest 212/212 across 23 files, build clean, no
  bindings drift). Voice 151 is FINE (algo 5, all carrier TLs 0 — loudest of
  five comparators by rendered RMS; the zyrinx extractor forces carrier TL 0
  so no pack voice can be quiet-by-data). ROOT CAUSE: velocity/track-volume
  are TL-denominated engine-wide (sequencer adds `(127−vol)+(127−vel)` to
  carrier TLs, 0.75 dB/step; audition applies none), but two stray literals
  assumed MIDI-100: PianoRoll placed notes at vel 100 and manager seeded/
  added tracks at vol 100 → every hand-placed FM note ~35 dB under audition.
  FIX: both defaults → 127 (comments carry the unit convention). DURABLE
  HARNESS: `src-tauri/src/audio/rendered_rms.rs` renders the real chain
  (Sequencer→AudioEngine→Nuked-OPN2) and gates playback-vs-audition <0.5 dB
  (shown red at −34.6 dB under sabotage) — first standing enforcement of the
  "rendered audio, never register proxies" bar. PARKED for owner: (a)
  existing saved projects are NOT auto-healed (old notes keep vel 100 /
  tracks vol 100 — raise manually or ask for a one-shot migration); (b)
  design Q: TL-linear curve gives FM ~95 dB control range vs PSG ~30 dB —
  perceptual curve would need SMPS import's fm_effective_velocity co-updated;
  (c) zyrinx song import places notes at vel 100 (−20 dB) and drops
  channel-volume events — possibly intentional headroom, untouched.

- 2026-08-21 (cont.): **ITEM-4 REVISION LANDED — loop never moves the view**
  (merged; lanes on merged tree: cargo 235/0, vitest 213/213 across 23
  files, build clean, no bindings drift). `followScrollLeft` reverted to
  forward-only (all wrap-snap machinery deleted); new shared gate
  `followAllowed(playing, loopActive, lastManualScrollAt, now)` in
  followPlayhead.ts — false while a preview loop is armed, absorbing the 2s
  manual-scroll suspend for both views; `loopEnabled` threaded App →
  BottomPanel → PianoRoll → PianoRollCanvas `suppressFollow` (optional
  props, wiring regression-tested). Behavior: loop armed = follow fully off
  in both views; loop off = forward paging as before, backward jumps never
  scroll. RATIFIED: optional-prop threading (wiring test mitigates
  silent un-wire); suppression trigger = loop ARMED (not playhead-inside-
  range) — flagged as the plain reading of the ruling. **ALL WAVE 3
  DISPATCHES NOW LANDED** (items 1,2,3,4,6 merged; 5,7 parked on owner
  rulings; audits merged). Cumulative owner gate additions: bar ruler +
  drag-zoom feel (~1%/px, 4px slop, 6px handles), loop handles, live
  marquee preview, loop-armed = static view across wraps, full-volume
  defaults (new track + new note ≈ audition loudness).

- 2026-08-21 (cont.): **OWNER RULINGS BANKED + WAVE 3B DISPATCHED.** Rulings:
  (1) item 7 APPROVED — Space pauses in place and pause/resume must NEVER
  move the double-Space return point; the return point updates only on an
  explicit seek (mouse/ruler/Home/the return-jump itself) followed by play.
  (2) Voice-per-track: owner approved BOTH shapes — Shape A now (set voice
  on selected notes, same piano roll) AND, with S4, per-voice viewing;
  ratified direction: S4's one-grid colored lanes + a per-voice filter
  toggle ("piano roll per voice" as a lens, not a separate editor), storage
  truth stays per-note instrument_id. (3) F15 view-state persistence
  DEPRIORITIZED (owner: current behavior matches how they work); next audit
  pick after in-flight parcels = F3/#4 live knob-tweak audibility (shares
  the F1 reload seam — deliberately sequenced after it lands). (4) No
  migration for old 100-velocity projects (throwaway test data).
  IN FLIGHT (3 parcels): `fix/live-edit-audibility` (audit F1+F2+stale-lane-
  name wart), `feat/note-voice-set` (Shape A: set_note_instrument IPC with
  validate-first kind gate + correct-by-construction different-voice overlap
  rejection at the mutation site, add_note voice passthrough + paste
  preservation, drag-voice-onto-selection, per-voice note colors + patch
  chips, drag-to-track wipe confirm), `fix/play-start-memory` (item 7
  ruling, transportMemory pure-function redesign).

- 2026-08-21 (cont.): **WAVE 3B PARCEL 1 SHIPPED — live-edit audibility +
  silent-lane cues (audit F1+F2)** (merged; lanes on merged tree: cargo
  237/0, vitest 228/228 across 23 files, build clean, no bindings drift).
  F1: every piano-roll note commit point (draw, delete/cut, transpose,
  nudge, paste, Ctrl+D, velocity click, move/resize via gesture-end hook)
  now reloads the running sequence — G30 pattern, unconditional, with a
  gestureMutatedRef so click-selects don't churn; undo/redo already
  reloaded (verified). F2: instrument-less lanes get a dimmed name + "no
  voice" pill on TrackHeader and a "silent — no voice assigned" badge in
  the piano-roll header (tooltips state the fix); F2c: deleting an
  instrument resets lanes named exactly after it to the channel-layout
  default name (`default_lane_name()` derives from the driver's
  ChannelLayout; user renames preserved via name-inequality heuristic —
  exact-match renames also reset, accepted edge). RATIFIED: unconditional
  reload (matches region path); rename heuristic; unknown-driver keeps old
  name. TAGGED for owner gate + BOOKED BACKEND FOLLOW-UP: reload_snapshot
  does silence_all+reprogram, so a note commit mid-loop momentarily cuts
  sustained notes on other channels (same cost region ops already pay) —
  if objectionable by ear, a targeted per-channel reprogram is the fix.
  Remaining in flight: `feat/note-voice-set`, `fix/play-start-memory`.

- 2026-08-21 (cont.): **ITEM 7 SHIPPED — pause never moves the double-Space
  return point** (merged; lanes on merged tree: cargo 237/0, vitest 236/236
  across 23 files, build clean, no bindings drift). transportMemory owns
  the record decision: `launchPointStale` flag (starts true; reset at
  project boundaries via resetSeekCursor→resetTransportMemory);
  `recordPlayStart` records only when stale; `noteSeek()` re-arms — hooked
  once in App.handleSeek, which covers ruler/mouse seeks, Home (key +
  button), and the double-Space return-jump. Resume no-ops on the tick.
  Double-tap mechanics (400ms, consumed on use) unchanged. Red-first: the
  exact bug shown failing (resume tick 2400 overwrote launch 960).
  RATIFIED (3 agent calls): seek-then-quick-Space inside the 400ms window
  still jumps to the launch point (pre-existing, rare; one-line change in
  noteSeek if ever wanted); seek-while-playing re-arms literally (next
  play — usually the following resume — records); project-boundary reset
  (scope addition, prevents cross-project launch-point leaks).
  Remaining in flight: `feat/note-voice-set` only.

- 2026-08-22: **LOCATION MEMORY SHIPPED** (owner ask; merged; lanes on
  merged tree: cargo 237/0, vitest 254/254 across 25 files, build clean, no
  bindings drift). New `src/utils/recentLocations.ts` — localStorage MRU,
  key `seraph.recentProjectLocations.v1`, cap 8, trailing-slash-insensitive
  dedup, corrupt-JSON/storage-failure safe. **This is the app's FIRST
  localStorage use and the designated persistence seam — audit F15
  (workspace persistence, deprioritized) should extend this pattern
  (versioned key + typed module), not scatter raw calls.** NewProjectDialog
  + ImportDialog prefill from MRU, custom dark suggestions dropdown (chosen
  over <datalist>: WebKitGTK styles it poorly), Browse gets defaultPath,
  record on success; App's Open seeds defaultPath and records the PARENT of
  the opened project dir. RATIFIED: Location input made editable (was
  readOnly — owner's screenshot showed a typed path); ImportDialog included
  (same create semantics); parent-dir recording on open.
  **RULER BUG FOUND BY OWNER (in flight, `fix/pianoroll-ruler-scale`):**
  PianoRollRuler renders ~1281 bars across ~1280px on a 4-bar region while
  the note grid below is correctly scaled (~640x off; label-thinning step
  64 is the symptom, dimmed past-region overlay invisible). Suspect wrong
  ticksPerPixel/width prop or DPR double-application. Reproduce-first
  ordered. SECOND DEFECT in scope: the piano-roll header read "Bars 1-3"
  for a 4-bar region — verify the header's range math independently.

- 2026-08-22: **ITEM 5 SHIPPED — set voice on selected notes (S4 Shape A)**
  (merged; lanes on merged tree: cargo 241/0, vitest 270/270 across 26
  files, build clean, bindings regenerated+committed, no drift). New
  `set_note_instrument` IPC (validate-first: track/region/indices, kind
  gate mirroring assign_library_instrument_to_track, THEN voice-overlap
  gate, THEN record_song_edit; one undo step per batch; None clears to
  region/track default). **Correct-by-construction overlap gate:**
  `for_each_conflicting_span` refactored out of build_snapshot's sweep —
  ONE authority now serving both the post-hoc OverlapWarning diagnostic and
  the edit-time rejection; groups by the same `channel_key`. `add_note`
  gained optional instrument_id; clipboard paste + region payload-replay
  preserve per-note voices (kind-gated). UI: drag a library voice onto the
  piano-roll note canvas with notes selected sets their voice (kind-gated,
  no-selection = hint only, never silent whole-track retarget); notes whose
  voice differs from the track default draw in a deterministic per-voice
  color (`src/utils/voiceColor.ts`, derived from instrument id) with a
  patch chip; TrackHeader drop now CONFIRMS before wiping mid-song voice
  changes (`countVoiceOverrides`) — closes the silent data-loss path.
  RATIFIED (7 agent calls): conflict = both effective voices Some and
  unequal (None = silent, no conflict); gate fires only on pairs involving
  an EDITED note (imported projects with pre-existing conflicts stay
  editable); gate ignores mute/solo; extra `library_ensure_project_instrument`
  IPC (drag payload carries a library hash, set_note_instrument takes a
  project instrument id); add_note validates an explicit voice strictly;
  errors surface as an auto-clearing inline notice in the piano-roll header
  (no toast system exists — first error surface, flagged as the seam);
  backend clear-on-assign kept (confirm is frontend-side). TAG for owner
  gate: drag-drop ergonomics + audible per-note voice switching.
  **WAVE 3/3B COMPLETE except the ruler-scale + header-range bugs
  (`fix/pianoroll-ruler-scale`, in flight).** Next audit pick per owner:
  F3/#4 live knob-tweak audibility (shares the F1 reload seam).

- 2026-08-22: **RULER SCALE BUG FIXED — root cause was the SHARED zoom, not
  the ruler** (merged `d518c33`, one import conflict with the voice parcel
  unioned by hand; lanes on merged tree: cargo 241/0, vitest 278/278 across
  27 files, build clean, no bindings drift). The ruler was the only HONEST
  surface: PianoRollCanvas masks a broken scale (gridlines skipped when
  denser than 4px, note widths floor at 2px), so the grid looked fine while
  the ruler correctly drew ~1281 one-pixel bars. Two real paths, both
  reproduced: (1) stale zoom across region switches — BottomPanel renders
  ONE persistent PianoRoll (no key), so useState(defaultTpp) never refit
  when opening a small region after a zoomed-out large one; (2) the
  zoom-out clamp was a flat `ticksPerBar * 2` = half-pixel bars. FIX: refit
  ticksPerPixel+scroll on `region.regionId` change; new derived helper
  `maxPianoRollTicksPerPixel(durationTicks, barTicks)` =
  max(duration,bar)/MIN_REGION_VIEW_PX (400px floor) replaces the flat
  clamp. HEADER CHECK: no defect — `floor(start/bar)+1 .. ceil(end/bar)`
  agrees with the true overlapped-bar span in every constructible case
  (3 new pinning tests); "Bars 1-3" on an owner-counted 4-bar region is
  consistent with a region spanning ≤3 bar-slots of current metadata
  (mid-bar start, or meter/content mismatch) — flagged, unresolved,
  re-check at the owner gate. RATIFIED: MIN_REGION_VIEW_PX=400;
  refit-on-switch drops per-region zoom memory (Cubase-style persistence
  would need a keyed store). **TEST BLIND SPOT CLOSED:** jsdom has no
  canvas 2D context so `draw()` never ran under test — the new
  `PianoRollRuler.scale.test.tsx` recording-context harness is the pattern
  to reuse (TimelineRuler is a candidate). **BOOKED DEFERRED (same
  staleness family, out of scope):** note SELECTION also survives region
  switches — indices point into another region's notes.

- 2026-08-22: **REGION-SWITCH STALENESS CLASS SWEPT** (branch
  `fix/region-switch-staleness`, commits `4a8c535` + this doc; lanes:
  cargo 241/0, vitest 286/286 across 28 files, build clean, no bindings
  drift). Closes the booked selection deferral above AND sweeps its
  siblings. New `src/components/PianoRoll.regionSwitch.test.tsx` (8 tests,
  6 red-first).

  **THREE REAL DEFECTS, all fixed in `src/components/PianoRoll.tsx`:**
  1. *Note selection survives the switch* (the booked item). `selectedNotes`
     is a Set of INDICES; carried across a switch the next Delete /
     transpose / nudge / cut / voice-drop rewrites arbitrary notes of the
     region just opened. Repro pins region B with MORE notes than A so
     every stale index is IN RANGE — with a smaller B the range guards
     refuse the edit and the test passes for the wrong reason (this
     actually happened on the first draft: the transpose case went green
     until B was enlarged, then showed `updateNote` called 3x on B's
     notes). Fix: `setSelectedNotes(new Set())` in the existing
     `region.regionId` effect. Transitively releases the G1 cross-tree
     note-selection signal (`noteSelection.ts`), which otherwise kept
     ArrangementView's Delete deferring to a meaningless selection.
  2. *Stale `notes` during the fetch window.* `notes`/`defaultVoiceId`/
     `hasInstrument` arrive asynchronously, so between the switch and the
     reply the component held region A's notes while every IPC call it
     made already carried region B's ids (Ctrl+A then Delete in that
     window deleted B's notes by A's count). Fix: the fetched payload is
     now ONE state object TAGGED with its `regionId`; a mismatch renders
     as no-notes and cannot be edited.
  3. *Out-of-order replies.* A `listTracks` reply for the region the user
     left could overwrite the open region's notes, or reach the
     close-on-missing path and CLOSE THE REGION JUST OPENED because a
     different one had vanished. Fix: `openRegionIdRef` guard drops
     replies whose region is no longer open. (Pre-existing bug, not
     introduced by the ruler fix.)
  Also cleared on switch: the inline voice hint (+ its timer) — it names
  the previous region's channel kind ("Only PSG voices can be dropped on
  this lane"), so over another region it is actively misleading. Judgment
  call, flagged for ratification.

  **DESIGN CALL — RESET, NOT KEYED REMOUNT (flagged for ratification).**
  Keying `<PianoRoll key={regionId}>` in BottomPanel would drop everything
  wholesale, but the enumeration below shows the subtree holds state that
  MUST survive: the grid-size selector, the DAC key-column width, and
  BottomPanel's own height/collapsed. It would also re-init two canvases
  and re-run the ResizeObserver on every region open. The reset path also
  matches the ratified zoom fix. Two new tests PIN the survivors
  (clipboard across a same-instance switch, grid selector across a switch)
  so a future remount cannot silently throw them away.

  **FULL ENUMERATION (method: enumerated every `useState`/`useRef` in the
  non-remounted subtree — PianoRoll, PianoRollCanvas, PianoRollRuler,
  PianoRollKeys, VelocityLane, BottomPanel — plus every module-level
  `let` in `src/utils/`, then grepped each name for CONSUMERS and COPIERS,
  not just its declaration site.)**
  - WRONG (fixed): `selectedNotes`; the fetched `notes` / `defaultVoiceId`
    / `hasInstrument` trio; out-of-order refresh replies; `voiceHint` +
    `hintTimer`.
  - CORRECT to survive: `ticksPerPixel`/`pianoScrollLeft` (already reset,
    d518c33); `gridIdx` (tool setting); `keysWidth` (already keyed on the
    DAC/melodic flip, and a user-resized DAC width SHOULD carry between
    DAC regions); `clipboard.ts` note+region slots and `lastCopied`
    (cross-region paste is the entire point — documented at the top of the
    module); `transportMemory.ts` (transport-scoped, region-agnostic);
    `seekTick` (App-owned, absolute song ticks, and the paste anchor
    already tests `cursorInRegion`); BottomPanel `collapsed`/`height` and
    its resize refs (panel chrome, above the region); `onCloseRef`,
    `canvasRef`/`containerRef`/`animRef` (identity plumbing).
  - HARMLESS, banked not changed: `voiceDropOver` (a dragover cue; a
    region switch cannot interleave with a drag on one pointer);
    `fmPreviewTimer` (a pending preview STOP — firing late is correct);
    `gestureMutatedRef` (reset by the next `handleGestureStart`; a stray
    late reload is region-agnostic and cheap); PianoRollRuler's
    `gestureRef`/`dragging`/`dragAxis` (zoom/scroll/seek only, no note
    indices); and — see the CORRECTION below — PianoRollCanvas's marquee
    and pan state.
    **CORRECTED 2026-08-22 (overseer review), see the follow-up entry:**
    this bucket originally also held PianoRollCanvas's `drag`,
    `moveDrag.targets` and `drawingRef`, justified by "switching regions
    requires a double-click in the arrangement, so the arrangement's own
    mousedown→mouseup tears the gesture down first — unreachable with a
    single pointer". **That frame is wrong and must not be reused: a
    region switch needs no pointer event at all.** Those three were real
    defects and are fixed.
  - NOT IN THE SUBTREE: TimelineRuler's `hoverZone`/`gestureRef` (loop
    handles) live in the ARRANGEMENT, which is not affected by a piano-roll
    region switch; the zone is recomputed from pointer position on every
    mousemove anyway.

  **DEFERRED (recorded, not fixed):**
  - *Vertical view is not refit on switch.* `scrollTop` (PianoRoll state
    mirroring `PianoRollCanvas`'s `.container` DOM scrollTop) survives,
    while the horizontal view now refits — an asymmetry. Self-corrects
    downward (the browser clamps on content shrink and fires `scroll`,
    which syncs the state and the key-column offset), but switching to a
    TALLER region keeps the old vertical position. Not fixed because
    neither surviving NOR resetting to 0 is right: the correct behavior is
    scroll-to-the-region's-notes, which is a feature, not a bug fix.
  - *Clipboard is stale across a PROJECT switch, not a region switch.*
    `noteClipboard` entries carry `instrumentId`s from the old project and
    `regionClipboard` carries old `trackId`/`regionId`s; App's open/new/
    import paths clear `selectedRegions` but never the module clipboard.
    Paste then fails LOUDLY (backend `add_note` validates an explicit
    voice strictly; the region path's `duplicateRegion` throws and its
    `addRegion` fallback throws too — as an unhandled rejection, which is
    the real wart). Different axis from this parcel; the fix is a
    `resetClipboard()` call on the three project-change paths.
  - *Doc-claim check:* `PianoRoll.test.tsx`'s existing "clipboard survives
    switching regions (module state, not component state)" test UNMOUNTS
    and re-renders, so it never exercised the real no-remount path it
    describes. The new file's pin does.

  **TAGGED for foreground confirmation (no emulator/app launched here):**
  open a region, select notes, open a different region — the header
  selection readout must clear and Delete must do nothing until you
  re-select; and switching regions must no longer flash the previous
  region's notes.

- 2026-08-22 (cont.): **IN-FLIGHT GESTURES TAGGED WITH THEIR REGION**
  (same branch, commit `5fd8988`; lanes: cargo 241/0, vitest 290/290
  across 28 files, build clean, no bindings drift). Overseer review
  overturned one verdict in the enumeration above — recorded here in full
  because the WRONG FRAME is the reusable part.

  **What the frame got wrong.** The HARMLESS verdict on PianoRollCanvas's
  index-bearing gesture state rested on: *"switching regions requires a
  double-click in the arrangement, and those gestures' mouseup listeners
  are on `window`, so the arrangement's own mousedown→mouseup tears the
  gesture down before `onSelectRegions` fires."* The window-listener half
  is true. The premise is false: **a region switch needs no pointer event
  at all.** Grepping every caller of `onSelectRegions` (which is what the
  standing "enumerate by what TOUCHES the state, not what declares it" bar
  demanded, and which I applied to declarations and then dropped for the
  reachability argument) finds two KEYBOARD paths in ArrangementView's
  window keydown effect: **Ctrl+D duplicate** ("the duplicates become the
  selection") and **Ctrl+V region paste** ("the pasted regions become the
  selection"). Neither involves the mouse, so a held drag survives.
  Method lesson: I reasoned from ONE entry path instead of enumerating the
  callers. A hedge on a hypothetical ("goes live if touch input is added")
  reads as a certificate that the live case was checked to the same
  standard — it was not, and it steered scrutiny away from the real path.

  **Reachability, per gesture (derived from the guards, which differ):**
  - Ctrl+D carries the G1 guard `if (pianoRollNoteSelectionActive()) return`.
  - Ctrl+V carries NO G1 guard — only the clipboard arbitration
    `lastCopiedKind() === "regions"`. **That is the documented design**
    (you paste whatever you copied last, and the two window-level paste
    handlers are mutually exclusive), so the fix does NOT belong in a new
    guard there.
  - note MOVE drag: mousedown selects the pressed note ⇒ G1 true ⇒ Ctrl+D
    blocked, **Ctrl+V reachable**.
  - RESIZE drag: near-edge mousedown does NOT select ⇒ with an empty
    selection G1 is false ⇒ **both keys reachable**. (Not in the review's
    reading; found on re-derivation.)
  - DRAW (double-click): never touches the selection ⇒ **both reachable**.
  - MARQUEE: commits only on mouseup, so from an empty selection G1 is
    false ⇒ **both reachable**.
  - ArrangementView Delete also switches, but to `[]` ⇒ the roll unmounts
    and the gesture's window listeners go with it ⇒ safe.

  **THREE REAL DEFECTS (red-first, each assertion showing the actual
  cross-region write, with region B sized LARGER than A so every stale
  index is in range):**
  - move: `updateNote("track-1","region-2", 0, 960, 60, 100, 240)` —
    region A's pitch 60 onto region B's note 0, which lives at tick 1920
    pitch 72. Fires ONCE PER MOUSEMOVE.
  - resize: `updateNote("track-1","region-2", 0, 1920, 72, 100, 1200)` —
    B's note 0 resized to a duration derived from A's note.
  - draw: `addNote("track-1","region-2", 0, 62, 127, 120)` — a note
    created in B by a gesture aimed at A. **This one reads no notes, so
    the `loaded` tag never masked it at all**; the other two were masked
    only until B's fetch landed.

  **FIX:** `PianoRollCanvas` takes `regionId` as a REQUIRED prop and tags
  `drag` / `moveDrag` / `drawingRef` with it at mousedown, refusing to
  commit once it no longer matches (compared through a ref at event time,
  the same event-time-read pattern `noteSelection.ts` documents). Chosen
  over resetting the gesture on switch, which would leave the canvas
  mid-gesture with the button still down. Move/resize refuse only the
  WRITE and keep the gesture alive so mouseup still closes the undo group
  its mousedown opened; draw drops its preview so the user is not shown a
  note that will never commit. The prop is required rather than defaulted
  because a default silently disables the guard for a caller that forgets
  it — it immediately caught two fixtures in `PianoRollCanvas.test.tsx`.

  **MARQUEE and PAN stay HARMLESS, with the reasoning replaced.** Not
  "unreachable" (they are reachable) but "they write nothing". `draw()`
  renders the band from the very same view pixels `marqueeRectFromView`
  converts, so what the user sees over the new region is exactly what it
  selects — refusing it would leave a visible rubber band that does
  nothing. Pan only offsets `scrollLeft`, is visible, and is corrected by
  the next scroll. The marquee's continue-to-commit behavior is PINNED by
  a test so the choice is deliberate rather than incidental.

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
