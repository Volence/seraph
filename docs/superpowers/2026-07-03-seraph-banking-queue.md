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
  - ~~*Clipboard is stale across a PROJECT switch, not a region switch.*~~
    **CLOSED 2026-08-22** — fixed on `fix/booked-defect-sweep`; see the
    entry at the end of this log. The "three project-change paths" guess
    below was re-derived from the call sites and confirmed exact.
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

- 2026-08-22 (cont.): **REGION-SWITCH STALENESS LANDED** (merged `e01f6d1`;
  lanes on merged tree: cargo 241/0, vitest 290/290 across 28 files, build
  clean, no bindings drift). **Everything the two entries above describe about
  seraph's own behavior is as of `e01f6d1`** — they name handlers and helpers by
  symbol, never by line, but the code they describe is exactly the code this
  parcel changed, so a later refactor can move or rename it. Re-ground against
  that revision before trusting the narrative; the generalized bar outlives the
  call sites either way (empyrean `9d2d1f1`, protocol bar 13). Covers both entries above — the declaration sweep and the
  gesture-tagging follow-up landed as one parcel. RATIFIED (all agent calls):
  per-state resets over keyed remount (the subtree holds state that MUST
  survive — grid selector, DAC key-column width, BottomPanel height — and a
  remount re-inits two canvases per region open; survivors are now pinned by
  test); `voiceHint` cleared on switch; `regionId` a REQUIRED canvas prop, not
  defaulted; marquee continues to commit.
  **REVIEW NOTE — the miss is the lesson, not the bug.** The first delivery
  bucketed the in-flight gesture state HARMLESS on the frame "a region switch
  requires a double-click in the arrangement, so the arrangement's own
  mousedown→mouseup tears the gesture down first". The window-listener half was
  true; the premise was not — `ArrangementView`'s Ctrl+D and Ctrl+V change the
  open region from a **window keydown, with no pointer event at all**. The
  agent had applied "enumerate the touchers, not the declarers" to state
  declarations (which is what found the untagged fetch and the out-of-order
  reply) and then dropped it for reachability, reasoning from one entry path
  instead of grepping every caller of `onSelectRegions`. Two of the overseer's
  own corrections were themselves incomplete and the agent corrected them back
  (resize is ALSO reachable under Ctrl+D — a near-edge mousedown returns before
  touching the selection, so the G1 guard stays false; and Ctrl+V's missing G1
  guard is not a defect, since clipboard arbitration is the documented design
  and the fix belongs in the gesture). Verified firsthand overseer-side before
  landing: marquee mouseup recomputes hits from the CURRENT notes and view
  rather than committing the stale preview ref; resize/move writes are all in
  mousemove, so the guards sit where the writes are.
  **STANDING BAR, generalized:** a reachability argument is an enumeration
  problem, and gets the same treatment as any other — count the callers, don't
  reason from the entry path you happen to have in mind. The touch-input caveat
  the first delivery volunteered ("goes live if touch input is added") read as
  a certificate that the one-pointer case had been checked to the same standard;
  it had not. **A flagged weak point vouches for nothing but itself.**
- 2026-08-22: **LIVE PARAMETER AUDIBILITY SHIPPED — F3 + F13 + the banked
  `reload_snapshot`/`silence_all` follow-up, all ONE defect** (merged `7f59c18`;
  lanes on the MERGED tree — the numbers that count, since this parcel and the
  region-switch one landed the same day: cargo **254/0**, vitest **304/304**
  across 30 files, build clean, no bindings drift).
  **REPRODUCED FIRST, as rendered audio** — new tests in
  `src-tauri/src/audio/live_edit_audibility.rs` on a harness extension
  (`rendered_rms::render_snapshot_with_edits` drops an `AudioCommand` into the
  stream mid-render, exactly as an IPC command lands between two audio-thread
  `render` calls; plus `stats_window`/`db_ratio`). Every expectation is a
  CONTROL RENDER of the same note played from the start at the edited value,
  never a transcribed dB figure.
  **THE MEASUREMENT CORRECTED THE AUDIT.** All three findings were
  `Sequencer::reload_snapshot` calling `silence_all`, and the failure was far
  worse than "stutter"/"inaudible": every mid-note `ReloadSequence` rendered
  **rms 0.00000** for the rest of the note (key-off on every sounding channel
  + blanket attenuation `0x0F` on all four PSG channels). A volume ride did not
  zipper, it muted the mix for the length of the drag. 8 tests red on main.
  **FIX (backend):** `reload_snapshot` diffs instead of silencing. Each channel
  with an active note is re-identified in the new snapshot by `ChannelType`
  (NOT index — indices are `BTreeMap` positions over non-muted tracks and shift
  on mute/solo/delete) and matched to a NoteOn sounding the same pitch across
  the current tick. A survivor keeps its key-on and gets a live reprogram from
  the NEW snapshot (FM patch regs when changed, always the carrier TLs, pan;
  PSG envelope swapped under the running player keeping its step index) with NO
  key-off/key-on, so the envelope keeps its phase. An orphan gets a targeted
  key-off on that channel alone. Untouched channels now receive nothing.
  `silence_all` stays for stop/seek/loop-wrap. New
  `SequencerOutput::PsgEnvelopeUpdate`; `ChannelType: PartialEq`; the FM patch
  and carrier-TL register tables extracted into `write_fm_patch` /
  `write_fm_carrier_tls` shared by the note-on and live paths (two tables
  drifting apart is the FM-preview bug); FM patch/pan cache preserved across a
  reload (it describes hardware state), so a volume-only edit costs 4 TL writes
  instead of a 27-register reprogram.
  **FIX (frontend):** `FmEditor` AND `PsgEditor` reload after a successful
  commit — **PsgEditor had the identical hole; the audit only inspected
  FmEditor**. New `src/utils/liveReload.ts` coalesces reloads for continuous
  gestures, self-clocked by the IPC round trip rather than a timer (leading
  edge immediate + one trailing reload per in-flight window), so a 20-event
  drag costs 2 snapshot rebuilds. Every input event still commits its value.
  **RED-FIRST DISCIPLINE:** the 4 "must still go silent" regression tests
  (delete note, mute track, delete PSG note, retune) were proven red by
  sabotaging the orphan key-off; the 6 UI wiring tests by reverting each call
  site; the coalescer tests by degrading it to a naive call and to
  leading-edge-only.
  **RATIFIED (all 7 agent calls, overseer review):** channel identity =
  `ChannelType` equality; survivor test = same pitch, `start < tick <
  start+duration`; preserve the FM register cache across reload; coalesce by
  round trip rather than rAF/debounce; keep per-event `updateTrack` and
  coalesce only the reload; PSG noise-mode edits deferred to the next note-on;
  and the `Panic` cache fix taken as adjacent scope (leaving it would have been
  a known-silent path with a test proving it silent — correct call).
  **OVERSEER VERIFICATION (firsthand, before landing):** `db_ratio` asserts on
  a zero reference and returns `-inf` rather than a floor, so the repeated
  `-869.1 dB` across red messages is a real denormal residue, not a sentinel
  standing in for "unmeasurable" — the reason a gate gives is checkable
  separately from its verdict, and here both hold. The four "must still go
  silent" tests compare against a CONTROL RENDER at a −40 dB threshold, not a
  transcribed figure. Orphan key-off correctly runs BEFORE `load_snapshot`,
  while the old channel table is still addressable.
  **OBSERVED, NOT FIXED (overseer, low priority):** a surviving DAC note whose
  SAMPLE changed keeps streaming the old sample to its end — `Dac(_)` has no
  live reprogram arm. This is F3's shape for DAC and is invisible today because
  no UI edits a DAC sample mid-note; it becomes real when one does.
  **DELIBERATELY OUT OF SCOPE (documented in the audit, do not re-derive):** a
  retuned note is not re-articulated (would re-attack during a drag); PSG
  noise-mode edits apply from the next note-on (re-writing the noise register
  resets the LFSR); SSG-EG-only edits do not trigger a live reprogram
  (`last_fm_patch` caches the 25 packed bytes, which exclude SSG-EG — no UI
  exposes it); DAC has no per-note register state. F4 (audition on ch0) is a
  separate parcel and was not touched.
  **OWNER GATE OPEN (by-ear, cannot be checked from an agent):** drag an FM
  knob hard while looping — timbre must move under your hand with no re-attack
  and no gap; ride a track volume while a SECOND lane sustains — the other lane
  must be completely undisturbed; ride it while a PSG envelope voice sustains —
  no re-attack. Play-test script items 3 and 5 in the feel audit were rewritten
  with the new expected behaviour.
  **DRIVE-BY DEFECT FOUND AND FIXED (flagged for ratification):**
  `AudioCommand::Panic` reset both chips without invalidating the sequencer's
  FM patch cache, so the next note-on saw its patch as "unchanged", skipped the
  reprogram and keyed on into a blank YM2612 — **the note after a Panic
  rendered rms 0.00000**. Known-ish (the wart was described in
  `src/api/library.ts`) but its severity was not: it was called "killing FM
  output until the next stop/seek", and stop/seek do NOT clear the cache
  either — only `reload_snapshot` accidentally did, which this parcel removes.
  Panic now calls the new `Sequencer::invalidate_all_fm_cache()`. Guarded by
  `a_note_after_panic_reprograms_its_patch` (red-first). Comment in
  `src/api/library.ts` corrected.
  **ADJACENT, NOT CLAIMED:** F1 (note-level edits mid-loop) had ALREADY LANDED
  when this parcel was written — the agent's brief predated that merge and its
  report calls the branch in-flight; it is not. This change is what makes those
  already-shipped PianoRoll reloads gapless, and it touches none of their call
  sites, so the two compose without rework.

- 2026-08-22 (cont.): **DEAD-CODE WARNING TRIAGE LANDED** (merged `bff898d`;
  lanes on the merged tree: cargo **254/0**, vitest **304/304** across 30 files,
  build clean, no bindings drift, **0 build warnings** — down from 8).
  Dispatched as triage rather than deletion precisely because a never-read field
  in this repo has historically been a dropped-wiring bug; that framing paid for
  itself once (see the parked finding below).
  **REMOVED as genuinely dead (2):** `InstrumentData::PsgEnvelope::period` — all
  four constructors hardcode `period: 0` and all three readers `..`-skip it; the
  real period is derived from note pitch in `program_psg`, and
  `PsgEnvelopePlayer` has no period field at all. `ChannelSequence::modulation` —
  redundant, not dropped: it was sourced from **`tracks[0]` only**
  (`manager.rs:987`), while the per-note fold at `manager.rs:928-934` already
  carries each note's own track params into every `NoteOn`, which the sequencer
  consumes. On a multi-track channel the removed path was an arbitrary track's
  params, i.e. strictly worse than what remains.
  **KEPT with narrow item-scoped `#[allow(dead_code)]` + reason (6):**
  `encode_channel_events`, `ProjectManager::is_open`, `Ym2612::read_status` are
  each referenced only from `#[cfg(test)]` modules (boundary spot-checked
  overseer-side: chip.rs mod at 52 / call at 60, smps.rs 810 / 877, manager.rs
  1537 / 1570) — deleting any would break tests. `read_status` is additionally
  real YM2612 surface. `AudioThread::running` + `stop` are **live machinery
  missing a caller**: the `running_cb` clone is read every buffer at
  `thread.rs:136` and its silence branch works; what is absent is any
  `RunEvent::Exit`/`on_window_event` handler in `lib.rs`. Deleting would have
  taken the working silence branch with it.
  **PARKED FINDING — `SequencerEvent::NoteOff::pitch` (owner ruling needed).**
  The field holds the correct **transposed** pitch (`manager.rs:946-949`) and no
  reader ever consults it: `process_event` matches `NoteOff { .. }` and keys off
  unconditionally (`sequencer/mod.rs:388-391`). Meanwhile `build_snapshot` merges
  **multiple tracks onto one channel** into a single sorted list
  (`manager.rs:903`, `for track in tracks`) and knows it — it emits
  `OverlapWarning`s for exactly this (`:971-979`). So for A(tick 0, dur 480) and
  B(tick 240, dur 480) on one channel the list is On(A,0), On(B,240), Off(A,480),
  Off(B,720), and the **stale Off(A) keys off B at 480**, truncating it 240 ticks
  early. Verified firsthand overseer-side; the guard is already available, since
  `self.active_notes[ch_idx]` is maintained on note-on at `mod.rs:386`.
  **The open question is semantic, not mechanical:** the channel is documented
  monophonic and overlaps are surfaced as warnings, so last-note-priority
  truncation may be intended. Note a pitch guard is only a partial fix — two
  overlapping notes at the SAME pitch still cut each other off. Do not
  "fix" this without a ruling; the field and its FINDING comment stay put so the
  evidence is not erased. Agent correctly declined to fix or to add a gate.
  **PRE-EXISTING, untouched, out of scope:** `unused variable: track_idx` at
  `import/smps_mapper.rs:1062` (+2 `unused_assignments` in the same unit) —
  visible only in the `cfg(test)` build, not in `cargo build`.

- 2026-08-22 (cont.): **OVERLAP FIDELITY — preview now matches the driver**
  (merged; lanes on the merged tree: cargo **259/0**, vitest **304/304** across
  30 files, build clean, **0 warnings**, no bindings drift). Research artifact
  landed separately at `2c6f79c`
  (`docs/research/2026-08-22-memra-note-termination.md`).
  **OWNER FRAMING MADE IT DECIDABLE.** Asked whether overlapping-note behaviour
  should be a taste call, the owner answered that seraph should sound like the
  Genesis — i.e. the driver-in-the-loop guarantee decides, not preference. That
  converted an open design question into a groundable one.
  **GROUNDED AT aeon `1ee8f8e6`** (aeon's default branch is **master**, not
  main), read via `git show <sha>:<path>` — never the sibling directory path,
  which is the aeon overseer's live working tree. Independently confirmed by the
  aeon overseer from their side.
  **THE DRIVER HAS NO NOTE-OFF EVENT AND NO NOTE IDENTITY.** One `sc_note` byte,
  one `SCF_KEYED` bit, 60-byte `SeqChannel` × 11 routes; no note-off opcode in
  the 32-entry `$E0-$FF` table (`seq_opcode_tab.emp:44-83`). Termination has four
  producers: `MEV_REST` ($80), `MEV_NOTEFILL` ($ED), the next note-on, and a PSG
  vol-env `$83` full-rest contour byte. **`sc_dur_count` ends the WAIT, not the
  note** — the most misleading part for a sequencer model.
  **A NOTE-ON ONTO A SOUNDING CHANNEL FORCES A KEY-OFF FIRST, so overlaps
  RE-ATTACK** — `Fm_NoteOnFreqExact.do_keyon` does `bit SCF_KEYED_B` /
  `call nz, Fm_NoteOff` before the `$28` key-on (`sound_fm.emp:1092-1099`), gated
  on *keyed*, not on pitch, because `$28` is edge-triggered. This is the OPPOSITE
  of the bare-chip behaviour and the overseer initially got it backwards; PSG
  re-attacks too, via `Psg_EnvCursorReset`. Cite by SYMBOL as well as line — aeon
  line numbers drift (a live instance found the same day: aeon's own
  `2026-08-07-mdsdrv` doc cites `:982-983` for lines now at `:1046-1047`).
  **VERDICT: seraph DIVERGED.** Hardware = last-note-priority, effective duration
  `min(authored, next onset)`; no event exists at the truncation tick at all.
  Precision correction worth keeping: the off is not "unrepresentable" (a `Rest`
  there is emittable and would truncate in ROM too) — it is that **no note
  identity exists**, so no monophonic serialization would emit it.
  **FIX IS AT EVENT CONSTRUCTION ONLY.** New `emit_channel_events` suppresses the
  note-off when a successor note-on supersedes it. `process_event` is UNCHANGED
  and its unconditional key-off is *correct* modelling of a pitch-blind driver —
  the pitch-guard idea floated earlier was rejected as a nicer-sounding fiction.
  `SequencerEvent::NoteOff::pitch` removed (it only ever supported the absent
  identity). `OverlapWarning`s retained deliberately: the ambiguity is real, it
  resolves at compile time rather than at playback.
  **OVERSEER CHECK THAT MATTERED (the uncited joint):** suppressing the off is
  only faithful if seraph's OWN note-on forces a key-off when keyed — it does
  (`sequencer/mod.rs:332-334`), structurally the same shape as `do_keyon`. Had it
  not, the fix would have turned a wrongly-truncated note into a wrongly-TIED one
  — diverging in the opposite direction and sounding *smoother*, i.e. the failure
  would have been mistaken for success.
  **SECOND DEFECT FOUND IN REVIEW, same class, different door:** a note whose
  instrument fails to resolve emitted **no note-on and a bare note-off**,
  silencing a note it did not own. Pre-existing, and the *default* state of every
  lane — `default_tracks_for_layout` seeds one instrument-less track per channel
  and `resolve_instrument_data` bails at `:1193`. Now contributes no events at all.
  **THE AGENT CORRECTED THE OVERSEER'S ENUMERATION AND WAS RIGHT:** both cases the
  overseer walked had the unvoiced note ending at/after the sounding note, where
  the stray off is harmless — the reachable break needs it ending strictly INSIDE
  (`A(0..960)` + unvoiced `X(240..480)` → A cut at 0.5s, rendered rms 0.00000).
  Adopting the overseer's cases unchecked would have tested arrangements that pass
  either way. The agent replaced case enumeration with a **property** (unresolved
  notes are inert: the emitted list equals what the resolvable notes alone
  produce, in three legs incl. "no successor is ever hidden from the `take_while`
  window"), which does not depend on anyone's imagination being complete.
  **RED-FIRST, RENDERED AUDIO, all expectations control renders:** the re-attack
  test was proven red by implementing the OPPOSITE divergence (dropping the
  successor's note-on to force a tie) — and because the usual sustaining voice
  separates tie from re-attack by only 0.28 dB, it uses a decaying voice and
  **asserts that >6 dB separation itself** before asserting on the render under
  test, so it fails loudly rather than passing vacuously. Non-regression test
  (non-overlapping pair still keys off) was red by forcing "never key off".
  **RATIFIED:** successor must actually emit a note-on to supersede; suppress at
  `end == next.start` (key-off-then-key-on vs. key-on-that-keys-off-first are the
  same two register actions at the same tick, and `advance` drains events before
  rendering, so no sample boundary separates them either).
  **S1 SPEC CLOSED:** last-note-priority is now normative, with carve-outs stating
  that FM6/DAC and portamento are unmodelled and that an unresolved note
  contributes no events.
  **STILL UNMODELLED (own parcel, not claimed):** the FM6/DAC key-on skip while a
  sample owns ch6 (no audible retrigger, `sound_fm.emp:1084-1091`) and armed
  portamento (attack at the slid pitch).
  **OWNER GATE OPEN (by-ear):** overlapping notes on one channel must now
  re-attack rather than drop out.

- 2026-08-22 (cont.): **BOOKED-DEFECT SWEEP — clipboard project boundary +
  an honest `get_playback_state`** (branch `fix/booked-defect-sweep`,
  commits `54ce277` + `e190a0c` + this doc; lanes: cargo **263/0**, vitest
  **309/309** across 31 files, build clean **0 warnings**, no bindings
  drift). Closes the clipboard deferral booked under REGION-SWITCH
  STALENESS and G41.

  **1. CLIPBOARD ACROSS A PROJECT SWITCH (`54ce277`).** `clipboard.ts` had
  no production reset at all — only `resetClipboardForTest`. Added
  `resetClipboard()`; `resetClipboardForTest` is now an ALIAS of it, not a
  second copy of the body (two bodies drift the moment a third slot is
  added, and the test-named export keeps test intent readable).
  **Enumeration (by what TOUCHES the state, not what declares it):** grepped
  every `openProject` / `createProject` / `importSong|importVgm|
  importZyrinxSong` / `closeProject` call site in `src/`. Three UI paths
  change the open project, and all three funnel through App handlers —
  `handleOpenProject` (TopBar "Open" and the MainArea welcome button),
  `handleProjectCreated` (`NewProjectDialog.handleCreate` → `onCreated`),
  `handleImported` (`ImportDialog.handleImport` → `onImported`; it does its
  own `closeProject` + `openProject` first). The booked count of three was
  right. Judged SAFE and left alone: `TopBar`'s `onProjectMetaChange`
  (renames/tempo edits the SAME project — ids stay valid) and
  `LibraryPanel`'s instrument import (adds to the library, does not switch
  projects). The calls are explicit at each of the three sites rather than
  folded into `resetSeekCursor`, so a fourth path is a visible omission;
  the three tests below are the gate that would catch one.
  **The unhandled rejection ("the real wart") is fixed too**, on BOTH paste
  handlers — each ran an async IIFE with no `catch`. PianoRoll routes the
  rejection to the existing auto-clearing header notice (`showVoiceHint`,
  the seam the `set_note_instrument` parcel added). ArrangementView has no
  notice element and the header hint is not reachable from it, so it logs
  `console.error("Region paste failed:", err)` — **flagged judgement call:**
  the least-invasive honest handling, chosen over inventing a toast system
  (that would have been a BLOCKED design call, not a thing to build). A
  region-paste failure is therefore still console-only; that is audit
  finding **F24**, unchanged and still open.
  **Tests (red-first, runner `npm test` / vitest):** new
  `src/App.projectSwitch.test.tsx` — "New Project clears it", "Open Project
  clears it", "Import clears it"; all three failed with
  `expected [ { tick: +0, pitch: 60, …(3) } ] to deeply equal []` with the
  three `resetClipboard()` calls commented out. Plus "a rejected paste shows
  the header notice instead of an unhandled rejection" (PianoRoll.test.tsx)
  and "a paste whose backend calls all reject is reported, not left
  unhandled" (ArrangementView.test.tsx); both failed with the `.catch`
  stripped, and vitest additionally reported the unhandled rejections
  (`Unknown Error: instrument not found` / `track not found`) — which is
  the other half of that gate. Note for future tests here: `vi.clearAllMocks()`
  clears calls but NOT implementations, so a sticky `mockRejectedValue`
  leaks into later cases — use `…Once`. Also, firing several dialog
  `Browse` clicks in one tick races the `@tauri-apps/plugin-dialog` module
  mock (the losers reach the real plugin and reject); click them one at a
  time.

  **2. `get_playback_state` MADE HONEST (G41, `e190a0c`).** It returned a
  real `tick` and real `channel_levels` next to a hardcoded
  `playing: false` / `loop_start: None` / `loop_end: None`.
  **Consumer enumeration first** (`src/`, `src-tauri/`, `src/bindings.ts`):
  `tick` is read by `App` (the G29 stop-sync effect and `startPlayback`'s
  play-start memory) and by `TransportControls.handlePlayStop`;
  `channelLevels` by `ArrangementView`'s 60 ms meter poll. **`playing`,
  `loopStart` and `loopEnd` have NO reader** — every view keeps its own
  optimistic copy (`App`'s `playing`/`loopEnabled` state). So this is not
  the dead-wiring shape the `bff898d` triage found; it is a supply-side
  lie with no consumer yet.
  **Fix:** the truth lives in `Sequencer` (`playing`, `snapshot.loop_start/
  end`) but `AudioEngine` is moved into the cpal callback closure, so the
  command side cannot borrow it. New `TransportPublish` (an atomic block
  next to the existing `position_tick` publish) is republished by
  `AudioEngine::publish_transport` after EVERY command. **Publishing from
  the sequencer rather than from the command that was sent is what makes it
  honest:** `Sequencer::play` refuses on an empty snapshot (the UI still
  flips its button to playing), and `LoadSequence` drops the loop with the
  snapshot that held it while `reload_snapshot` carries it across.
  **Judgement call, flagged:** this is new plumbing, but it is the SAME
  publish channel `position_tick` already uses, widened — not a new
  subsystem. The alternative (remembering in `AudioState` what the frontend
  last asked for) would have made it *look* honest, which the parcel names
  as the failure mode.
  **Tests (red-first, runner `cargo test`):** four
  `audio::engine::tests::transport_publish_*` cases — play/stop, the
  refused-play divergence, the loop range, and reload-vs-load. With
  `self.publish_transport()` commented out: `after TransportPlay the
  sequencer is playing`, `a reload does not stop the transport`, and
  `assertion left == right failed: left: None, right: Some((480, 1920))`.
  No IPC type changed, so `src/bindings.ts` is untouched (verified: `cargo
  test` regenerates it and `git status` is clean).
  **BOOKED — the loop-bound publish can be read torn (overseer FIX 1).**
  `TransportPublish`'s Release/Acquire on `loop_active` gates the bounds
  correctly for UNARMED -> ARMED and for disarming, but it does NOT make an
  armed -> RE-ARMED range change atomic: with the flag already `true` a
  reader can acquire it from the previous publish and then relaxed-load the
  new `loop_start` next to the previous `loop_end` — a torn
  `(new_start, old_end)`. The window is open on every drag of a loop edge.
  Deliberately NOT fixed here: nothing reads `loopStart`/`loopEnd` (that
  enumeration is above), so a seqlock for a field with no reader is
  gold-plating. **CLOSE IT (seqlock, or pack both bounds into one
  `AtomicU64`) BEFORE the first consumer wires either field to anything.**
  The type's doc comment now states the guarantee it actually provides and
  names the window — the original comment asserted the strong version, and a
  stale claim inside a comment outlives every doc that recorded it because
  nobody re-reads a comment to check whether it still holds.
  **NOT DONE, deliberately:** no view was rewired to the now-honest
  `playing` / loop range. Doing so changes transport UI behaviour (the
  button would stop lying about a refused play) and is a separate call.
  The command-level `get_playback_state` itself has no test — it needs a
  live `AudioThread` (a real output device); the engine-level publish is
  the honest gate.

  **3. FEEL-AUDIT CITATIONS RE-GROUNDED (docs only).** See
  `docs/superpowers/2026-08-21-daw-feel-audit.md`: the findings table cited
  `commands.rs:NNN` — a path that does not exist (the module is
  `src-tauri/src/ipc/commands.rs`) — and line numbers that had already
  drifted. Still-open rows now name the SYMBOL instead of a line, per the
  empyrean OVERSEER-PROTOCOL rule that "a correction that carries a line
  number inherits the defect it was correcting". Severities and verdicts
  untouched; this was citation hygiene, not a re-audit.

  **FLAKE KILLED, NOT WATCH-LISTED (overseer FIX 2).**
  `ArrangementView.test.tsx` > "paste replays the copied payload when the
  source region is gone" timed out its 1 s `waitFor` ONCE in ~9 full-suite
  runs. Rather than raise the budget, the assertions were made
  DETERMINISTIC: the paste chain is entirely mocked promises with no timers,
  so one `await act(async () => {})` drains it and the assertions run
  synchronously — no polling, no timeout for a loaded box to blow through.
  Proven to still have teeth: with the expected trackId swapped for a
  sentinel the test fails in **19 ms** with a real argument diff, i.e. the
  drain genuinely completed rather than the assertion being skipped. The
  same treatment was applied to both paste-rejection tests added by this
  parcel, so the parcel adds no new load-sensitive gate. It also now asserts
  `getRegionClipboard()` is non-empty BEFORE the paste: a copy that silently
  no-ops (empty `tracks`) used to surface as a slow timeout that reads like
  a flake instead of the bug it is.
  **Reusable rule:** in this suite, `waitFor` on a chain of mocked promises
  buys nothing but a 1 s failure budget — drain with async `act` and assert
  directly. Keep `waitFor` for genuinely timed or event-loop-deferred work.

  **TAGGED for foreground confirmation (no emulator/app launched here):**
  copy notes in project A, open project B, Ctrl+V — nothing should paste
  and nothing should appear in the devtools console as an unhandled
  rejection; and a paste that the backend rejects should show the
  auto-clearing notice in the piano-roll header.
- 2026-08-22: **ONE-GESTURE NOTE ENTRY + RIGHT-CLICK ERASE SHIPPED** (F6, the
  audit's #5; G13/G14 closed). Branch `feat/note-entry-grammar`, commits
  `9f2dc66` (feature+tests) and this doc entry; merged `8359f75`. **Lanes on
  the MERGED tree** (the numbers that count — this parcel and the defect sweep
  landed the same day): cargo **263/0**, vitest **336/336** across 31 files,
  `npm run build` clean 0 warnings, no `src/bindings.ts` drift. (Branch-side
  figures were cargo 259/0 and vitest 331/331 across 30 files, off `ecb2fcd`
  before the sweep landed.)
  **COMPLETES the banked Ableton ruling rather than forking it:** double-click
  draw survives as the no-mode default; one-gesture entry is a MODE, exactly as
  `2026-08-21-daw-comparator-idioms.md` §1 describes. Grammar now: *no mode* —
  double-click draws (drag sets length), left-drag marquees, Shift additive,
  middle/Alt pans, edge-drag resizes; *Draw Mode* — click paints one
  grid-length note, drag paints a RUN (one cell per grid cell entered, pitch
  follows the pointer row, Shift = Ableton Pitch Lock to the start row),
  double-click-draw suppressed, empty-space marquee unavailable, note
  move/resize/pan unchanged; *both modes* — right-click erases the note under
  the cursor, empty space does nothing (browser menu still suppressed).
  **RATIFIED-PENDING (each an explicit judgement call, listed so they can be
  rejected one by one):** (1) binding = `B` (Ableton's Draw key AND FL's Paint
  key — the same letter in both comparators for the gesture added; `P` would
  have been FL's single-note Draw, which is the no-mode default we already
  have). (2) Right-click erase applies in BOTH modes (FL's rule: the commonest
  correction never costs a tool switch). (3) Right-click on empty space does
  nothing — reserved for a real context menu. (4) Left-click on an existing
  note in Draw Mode still selects/moves it; Ableton would DELETE it, but with
  a dedicated erase button already bound, click-to-delete under a paint gesture
  is a data-loss trap. (5) Paint pitch follows the pointer (FL) rather than
  locking by default (Ableton), Shift inverts it. (6) Draw Mode suppresses
  double-click-draw, so note LENGTH in Draw Mode comes from edge-drag. (7)
  Draw Mode is TOOL state: it survives a region switch (pinned), like the grid
  selector. (8) Paint refuses cells already occupied by a note (a stacked
  duplicate is inaudible under last-note-priority = silent corruption) and
  cells past the region end. (9) Right-click erase REMAPS the surviving
  selection for the index shift instead of clearing it.
  **INVARIANTS HONOURED (all pinned by test):** paint run is region-TAGGED and
  commits nothing after a switch (proven red: without the guard it wrote 3
  notes into the region that replaced it); one paint gesture = ONE undo group
  + ONE `reloadSequence` (F1); new notes derive velocity from the new
  `DEFAULT_NOTE_VELOCITY = MAX_VELOCITY` constant in `pianoRollEdit.ts` (the
  last hand-typed 127 is gone); grid snap honoured (never a hardcoded step);
  `B` sits behind the existing `isEditableTarget` guard (G2 — proven red by
  hoisting it above the guard). Cross-gesture de-dup shadow (`recentlyPainted`)
  kills the double-click-in-Draw-Mode duplicate that the async re-fetch window
  would otherwise allow.
  **RED-FIRST:** all 27 tests proven red — 23 by reverting both components to
  `ecb2fcd` (13 canvas + 10 roll: "expected vi.fn() to be called 1 times, but
  got 0 times", "Unable to find an accessible element with the role button and
  name Draw"), the guard-shaped ones by targeted mutation (occupancy/dedup →
  `paintCellBlocked` returning false: "expected vi.fn() to be called 1 times,
  but got 2 times"; velocity+grouping → per-note group at velocity 100:
  "expected vi.fn() to be called 1 times, but got 3 times"; audition →
  per-cell: "expected [[95],[95],[96]] to deeply equal [[95],[96]]").
  **NOT BUILT (owner call still open):** QWERTY/step entry (F5, G36).
  **DEFERRED (named, not silently dropped):** right-DRAG erase (a swipe
  deleting a run — needs index-stability handling across N deletes mid-drag);
  velocity paint (F21/G17); a real right-click context menu.
  **OWNER GATE OPEN (visual + by-ear, never attempted here):** the Draw toggle
  reads clearly in the header; a paint-drag over a running transport is audible
  on the next pass and sounds at audition loudness; painted-run preview
  rendering (jsdom has no 2D context, so no test can see it).

- 2026-08-22 (cont.): **OVERSEER LANDING NOTE — both parcels merged and pushed**
  (`69873d6` sweep, `8359f75` F6; `origin/main` verified moved by `ls-remote`
  after each). Only the queue doc conflicted (two appended entries, both kept);
  `PianoRoll.tsx` auto-merged and was checked FEATURE-WISE rather than trusted
  textually — the sweep's paste-rejection `.catch`/`showVoiceHint` and F6's
  `drawMode`/`handleNotesPaint`/`handleNoteErase` both verified present in the
  merged file, since a clean textual merge of two parcels editing one component
  is not evidence that both behaviours survived.
  **RATIFIED — all 10 of F6's flagged calls**, plus the sweep's three (the
  `TransportPublish` plumbing, console-only region-paste errors, the delegating
  `resetClipboardForTest` alias). Two were verified firsthand rather than taken
  on the agent's word, because the whole design rests on them: `add_note`
  **pushes** (`project/manager.rs:1409`) so a painted run cannot invalidate a
  live selection, and `delete_note` uses `Vec::remove` (`:1597`) so erase must
  remap later indices — which is exactly what each does. The `B` binding was
  confirmed collision-free with a CONTROL grep (enumerating every existing
  `key === "…"` handler) rather than from an empty result, since an empty grep
  and a broken grep are indistinguishable.
  **ONE DELIBERATE DEVIATION FROM THE BANKED OWNER RULING, surfaced not buried:**
  in Draw Mode a left-click on an existing note still selects/moves it; Ableton
  would DELETE it. Ratified provisionally because a dedicated right-click erase
  is already bound, so click-to-delete under a paint gesture is a data-loss trap
  with no compensating gain. **Owner may overturn at the by-ear gate** — it is a
  one-line change if they want literal Ableton behaviour.
  **REVIEW FINDING, fixed before landing:** `TransportPublish`'s doc comment
  claimed the Release/Acquire flag ordering made the published loop range
  consistent. It does for unarmed→armed and for disarming, but NOT for an
  armed→re-armed range change: a reader can acquire a stale `true` flag and
  relaxed-load `(new_start, old_end)`. Open on every drag of a loop edge. Ruled
  AGAINST building a seqlock — nothing reads those fields (enumerated) and
  concurrency machinery for a field with no reader is gold-plating; the comment
  now states the real guarantee and names the window. **Close it before wiring
  `loopStart`/`loopEnd` to anything.**
  **AUDIT DRIFT BOOKED (found while verifying, deliberately not acted on):**
  two feel-audit findings have partly aged out. **F16** — `NewProjectDialog`'s
  Location field is no longer `readOnly` and recents exist (LOCATION MEMORY
  parcel), so "forced Browse, no recents" is now half true. **F19/F20** — "no
  Esc anywhere" is wrong: three `key === "Escape"` handlers exist on main.
  Neither verdict was rewritten; re-ground both before funding a parcel off
  them. This is the same perishability the citation re-grounding pass addressed
  from the other side — the findings' PROSE ages as well as their line numbers.
  **CROSS-REPO, no action owed:** the aeon overseer closed the sticky pan-gate
  mute finding (`Sfx_UnpauseRestore` handles it; both callers of the mute sweep
  accounted for) and pushed a stale-citation fix at aeon `b16ec612`, verified
  reachable at their `origin/master` from this side. Checked whether the hazard
  class has a seraph analogue: it does not. `silence_all`
  (`sequencer/mod.rs:564-578`) never touches `$B4`, all five cache-reset sites
  reset `last_fm_patch` and `last_fm_pan` together, `reload_snapshot` preserves
  both together, and every mutation of `playing`/the loop bounds is
  command-driven. Seraph gets the driver's repair for free: `stop`/`seek`
  invalidate the pan shadow, so the next note-on re-asserts pan.

- 2026-08-22 (cont.): **FEEL-AUDIT RE-GROUNDED, PASS 2 — all 26 rows, the
  ranking, the prose and the play-test script** (docs-only, branch
  `docs/reground-feel-audit`, base `3d72793`; no code touched). The booked drift
  was the trigger; the pass found the booking was incomplete in both directions.
  **The three booked drifts, confirmed and corrected rather than repeated:**
  **F16** — Location field editable, prefilled from `mostRecentLocation()`, with
  a suggestion listbox and `defaultPath` on both Browse and Open; severity med →
  low, and the surviving half restated precisely (no scratch project, no recent-
  *projects* list, still no Enter/Esc in the dialog). **F19/F20** — the three
  `key === "Escape"` handlers are `TrackHeader.handleRenameKeyDown`,
  `TopBar.handleMetaKeyDown` and LibraryPanel's tag-edit — **all three of which
  the audit's own inventory table already listed**, which is how the claim got
  written: the author read their own table's "Inline edit fields" row and still
  wrote "no Esc anywhere". The load-bearing claim (no dialog handles Enter/Esc)
  survives and was re-verified WITH A CONTROL (`onClick` found in the same four
  files by the same grep shape). **F15** — `Where` said "grep (no localStorage)";
  `src/utils/recentLocations.ts` is the app's first localStorage user. The
  finding itself is untouched (`ProjectFile` is still `{metadata, tracks}`), but
  the premise "no persistence seam exists" is dead and the `Where` now points at
  the seam instead of denying it.
  **SIX VERDICTS CHANGED.** FIXED: **F1** (`54c6082` — all 13 PianoRoll mutation
  paths enumerated, every one reloads), **F6** (`8359f75`). Already-booked FIXED
  confirmed: F3, F13 (`7f59c18`). PARTLY FIXED: **F2** (cues shipped, cure did
  not — severity critical → med), **F7** (per-note voices exist end to end via
  `abb22a9`, so "impossible / no UI or IPC" is dead — high → med), **F16**,
  **F17** (ruler zone cursors killed the "invisible halves" sub-claim — high →
  med). NARROWED but open: F9 (paint auditions per pitch; transpose/move-drag
  still silent), F11 (`zoomAtBy` anchors, `handleWheel` still doesn't — the seam
  exists), F24 (`showVoiceHint` is a working in-app notice channel; three sites
  still bypass it).
  **TWO NEW FINDINGS, NEITHER PREVIOUSLY BOOKED — this is the high-value output,
  and both were found by checking a shipped fix rather than by re-reading a
  finding.** **F25: per-note voice assignment is unreachable for DAC.**
  `PianoRoll.handleVoiceDrop` gates on `kind !== region.channelType`, and
  `LibraryInstrument` has exactly two variants (`Fm`, `Psg`) — `grep -rn "Dac"
  src-tauri/src/library/` exits 1, checked in isolation. So on a DAC lane the
  only per-note-voice gesture in the app *always* fails with "Only DAC voices can
  be dropped on this lane". `set_note_instrument` and
  `resolve_instrument_data_by_id`'s Dac arm both support it; nothing in the UI
  can call them. **This is exactly the name/presence/behaviour trap** — F7 reads
  as closed at the IPC layer while the gesture is dead for the one chip F7 was
  about. Imported songs still carry per-note DAC ids, so the read path is live;
  only authoring is unreachable. **F26: every audition costs a full `listTracks`
  round-trip.** `PianoRoll.handleAudition` opens with `await ipc.listTracks()` —
  the whole track/region/note tree over IPC — before it can send a preview, on
  note press, grid double-click, keys-column click, and (since F6) **once per new
  pitch of a paint run**. Code-certain, not measured.
  **ONE ERROR IN THE AUDIT'S OWN TEXT, corrected:** F12 said WAV export "renders
  a fixed user-supplied duration (default 60 s)". `TopBar.handleExportWav` calls
  `ipc.exportWav(path, 60)` with a literal and there is no duration input
  anywhere — the finding is slightly worse than it was written, not better.
  **RANKING RE-DERIVED, not renumbered.** Four of the original Top-10 shipped
  (F1/F3/F6/F13), two half-shipped (F2/F7). New #1 = **F4, audition on a free
  channel**, promoted from #7 on a stated principle: *a finding whose blast
  radius GREW because a neighbouring parcel landed outranks one that merely
  stayed put.* F1/F3 made edit-while-looping the normal workflow, and F6's paint
  run auditions **per new pitch** — so one drag across five rows now steals ch0
  five times. **F15 was NOT promoted despite still being critical**: the owner's
  deprioritization ruling is honoured in the fundability note, and the severity
  and the ruling are now recorded together so a cold session cannot read one
  without the other.
  **PLAY-TEST SCRIPT REWRITTEN, not amended.** Three of its eight steps were
  instructing the owner to confirm behaviour that had already been fixed — a
  wasted gate that cannot be re-run cheaply. Two steps are now regression checks
  on shipped fixes (F1, F3/F13) because only ears can confirm those; step 2 pairs
  the F6 regression check with the F4 measurement, which is the highest-value ear
  minute in the script. F6's Draw-Mode click-on-existing-note deviation is
  carried into the script as an explicit owner call.
  **METHOD NOTES worth keeping.** Every absence claim has a paired positive
  control (dialog `onKeyDown` vs `onClick`; library `Dac` vs `LibraryInstrument`
  read directly; "no QWERTY map" from enumerating all 11 keydown sites rather
  than guessing a name). One near-miss caught in flight: a `grep -v` filter on
  the Sidebar check would have hidden the answer inside its own exclusion
  pattern — re-run unfiltered. Another: `EXIT=$?` after an intervening `echo`
  reports the echo's status, not the grep's; the DAC absence was re-checked with
  the status isolated. The audit's symbol-not-line-number discipline was
  preserved — **no line numbers were added, and every symbol in a still-open row
  was confirmed to exist at `3d72793`** rather than inherited from the first pass.
  **TAGGED for the controller (never attempted from a background lane):** F4 and
  F8 still need ears or rendered audio. F4 is measurable *without* ears using the
  existing `rendered_rms` / `live_edit_audibility` harness — fire a preview
  mid-note in a two-lane snapshot and assert the non-previewed lane's rms is
  undisturbed. That is the shape that corrected F13's severity, and it should
  precede the F4 parcel, not follow it.
  **DELIBERATELY NOT DONE:** the ~60 `file:line` coordinates inside Scenarios A–G
  were not converted to symbols — their *claims* were re-checked and every false
  one is enumerated in a new "Prose drift" section, but rewriting the narrative
  record buys no funding accuracy. The findings table's `Where` is the
  symbol-grounded address of record; never take an address from the prose.
  **COMPLETENESS, stated at the strength actually earned:** 26/26 rows carry a
  **[V]** mark meaning a symbol or behaviour was read this pass — nothing was
  executed, no suite was run, no audio rendered. No claim is made about findings
  *absent* from the table; F25 and F26 surfaced incidentally, which is evidence
  the table records what has been looked at, not what is wrong with the app. The
  previous pass asserted completeness three times and missed three times.

- 2026-08-22 (cont.): **OVERSEER LANDING NOTE — re-grounding merged (`5585115`);
  an F15 PARCEL WAS DISPATCHED IN ERROR AND STOPPED.**
  **THE ERROR, recorded because the next session boots from this file and would
  repeat it.** This overseer booted, read the Log's *header and tail* plus
  `OVERSEER.md`, ranked F15 (severity critical, audit rank #3) as the front of
  the queue, and dispatched a parcel for it. **F15 was DEPRIORITIZED BY THE
  OWNER** — banked in this very document ("F15 view-state persistence
  DEPRIORITIZED (owner: current behavior matches how they work)"), in a Log
  entry sitting between the two ranges that were read. The agent's report
  correctly cited that ruling as its reason for not promoting F15; the overseer
  initially suspected the agent had fabricated an owner ruling and checked —
  the agent was right and the overseer was wrong.
  **Generalisable, not a one-off slip:** a severity number in a findings table
  is a property of the CODE; a deprioritization is a property of the OWNER's
  intent, and only the second one can make a critical finding not-next. Reading
  a queue's head and tail gets every landing and misses every *ruling*, because
  rulings are recorded where they happened, not where the reader is looking.
  **A cold session must grep this Log for the finding ID before funding any
  parcel off the audit** — `grep -n "F<NN>"` over this file costs one command
  and is the only step that can refute "this is the obvious next parcel".
  **DISPOSITION:** branch `feat/view-state-persistence` (worktree
  `agent-ad49279747a861166`, commits `78597ab` + `cffb634`, plus uncommitted
  `App.tsx` edits and a new `App.viewState.test.tsx`) is **PRESERVED UNMERGED**,
  not deleted — if the owner ever reverses the deprioritization the work is
  most of the way there. It has NOT been reviewed and must not be landed on the
  strength of this entry. Its design premise was independently sound: the
  LOCATION MEMORY entry above had already designated `recentLocations.ts` as
  the persistence seam ("versioned key + typed module, not scattered raw
  calls"), which is the same call the overseer re-derived rather than read.
  **MERGED — `5585115`, docs-only** (2 files, +468/−83; `--stat`-verified to
  touch zero code, which is why the code lanes below cannot be attributed to
  it). Findings table F1–F26 re-grounded, ranking re-derived, prose drift
  enumerated, play-test script rewritten. **Two new findings, both verified
  firsthand by the overseer rather than taken on report:** **F25** — per-note
  voice assignment is DEAD for DAC: `LibraryInstrument` has exactly `Fm`/`Psg`
  (no `Dac` anywhere under `src-tauri/src/library/`, grep exit 1) while
  `handleVoiceDrop` rejects on `kind !== region.channelType`, so on a DAC lane
  the app's only per-note-voice gesture always fails with a hint. The *joint
  the report left implicit* was checked separately, since it was the only step
  that could have refuted the finding: DAC regions really do carry
  `channelType === "dac"` (`PianoRoll.tsx` `isDac`). **F26** — every audition
  opens with `await ipc.listTracks()`, i.e. the whole song over IPC per note
  press, and since F6 once per new pitch of a paint run.
  **LANES on the merged tree:** cargo **263/0**; `npm test` **336/336 across 31
  files**. **A FLAKE WAS OBSERVED AND IS NOT YET IDENTIFIED:** the first run
  reported `1 failed | 335 passed`, four subsequent runs were clean. The
  failing test's NAME IS UNRECOVERABLE because that run was piped through
  `tail -20`, which discarded the `FAIL` lines — the same family of defect as
  `2>/dev/null`: the truncation destroyed the artifact that would have named
  it. **Booked, not watch-listed**, per this repo's standing rule that a flake
  gets made deterministic rather than tolerated. Second lesson from the same
  command: **`npm test | tail` reports `tail`'s exit status, not vitest's** —
  that run exited 0 while a test was failing, so an exit code from a pipeline
  is not a gate.
  **NOT RUN:** `npm run build` — stated rather than implied, since the merge
  provably touches no code.

- 2026-08-22 (cont.): **OWNER RULINGS — F25+F26 BEFORE S0, AND S0 IS RETARGETED
  OFF MEMRA.** Two rulings, and they reached this lane by different routes, which
  is itself recorded because the routes have different evidentiary weight.
  **RULING 1 — SEQUENCING, taken DIRECTLY from the owner in this session
  (granting act witnessed):** asked to choose between opening S0 and clearing two
  small parcels, he chose **"F25 + F26 first, then S0"**. Background: a peer lane
  relayed an owner approval of S0 given as the single phrase *"I guess"*; this
  lane **declined to act on it** and put the question to the owner directly
  instead. Two reasons, and the second is the load-bearing one: a relayed
  approval is not this session's to act on, **and** this lane had authored the
  reframe that produced the answer — so it was the last party that should treat
  the answer as a mandate. The peer independently reached the same
  recommendation. Standing rule, now suite-wide: **never record an approval whose
  granting act you have not seen.**
  **RULING 2 — S0 SCOPE, RELAYED (granting act NOT witnessed by this lane;
  transcription, not quotation, until anchored).** Asked whether to open S0, the
  owner reportedly said: *"We can start this but like not with memra engine yet I
  don't think, maybe s2 clone driver, zyrinx driver, flamewing driver, then like
  s1/s2/s3k driver?"* Read as: **S0's capability manifest is to be designed
  against the S2-clone / Zyrinx / Flamewing / S1-S2-S3K drivers, NOT Memra
  first.** This is a **scope change to a banked plan**, not a go-ahead — the S0
  plan and `specs/2026-07-03-s0-memra-contract-design.md` are both written
  Memra-first. Note the register (*"I don't think"*, ends in a question mark):
  **direction with the reasoning open.** Do not re-derive the S0 plan against
  four drivers on the strength of this entry; re-put the question when S0 is
  actually opened, and get the ruling first-hand.
  **The argument FOR the retarget** (the relaying lane's read, explicitly not the
  owner's stated reasoning, which he did not give — recorded so a later session
  can weigh it rather than inherit it): a manifest designed against one driver is
  a description of that driver wearing a manifest's clothes, and you cannot tell
  which parts are general until a second implementation disagrees with the first.
  Four established drivers make the manifest's shape fall out of real variation
  rather than out of one case plus imagination — abstraction extracted, not
  guessed. It also makes S0 checkable immediately, since those drivers' ROMs
  already exist, which answers the standing objection that S0's payoff was
  suite-integration rather than anything audible.
  **CONSEQUENCE for whoever opens S0:** it is now a LARGER and more open-ended
  piece of work than when it was banked, which strengthens rather than weakens
  the case for clearing small parcels first. Re-ground the plan's aeon-facing
  inputs as always, and re-confirm the driver list with the owner before
  designing to it.

- 2026-08-22 (cont.): **README RE-GROUNDED** (owner directive, verbatim: *"let's
  quickly have everything update their readmes correctly. Doesn't have to be
  super in depth"* — accuracy over depth). Merged `f5eb86f`, README-only
  (+36/−18). It had not been touched since `b22b782` (2026-06-28) and a great
  deal had shipped under it. Corrections included: **"cycle-accurate" deleted**
  (true of Nuked-OPN2, false of the hand-written SN76489 and of the DAC path);
  the channel roster replaced with the literal `channel_layout()` names; FM
  import narrowed to the four extensions the match arm actually accepts
  (`.tfi/.vgi/.y12/.gyb`); Zyrinx reworded as ROM-extraction rather than a song
  format; VGM demoted to core-only. Added: the instrument library (606 committed
  entries, counted with `git ls-files`), live-edit/live-parameter audibility,
  Draw Mode, per-note voice override, and the four dev commands — **all four
  executed, exit codes read directly rather than through a pipe**.
  **BOOKED DEFECTS found while verifying the README — none fixed, all evidenced.**
  A README pass reads a lot of surface at once and is unusually productive of
  these:
  1. **`extract_library --help` cannot print usage** — `let out = …get("out")`
     is evaluated before the subcommand match, so any invocation without `--out`
     dies first; the `usage()` arm is unreachable. Confirmed by running it.
  2. **`export_vgm` is dead from the UI** — command + typed wrapper exist, no
     `.tsx` caller (control grep on `exportSong` returns a hit, so the empty
     result is evidence).
  3. **WAV export duration hardcoded** — `ipc.exportWav(path, 60)` at the call
     site regardless of song length; the IPC takes the duration as a parameter,
     so this is purely a UI gap. (This is audit **F12**'s real shape: the audit
     said "user-supplied duration, default 60 s" — there is no duration input at
     all.)
  4. **DAC does not steal FM6 in the preview engine** — `AudioEngine` keeps the
     DAC as an independent stream summed into the mix, and register `$2B` is
     never written by `audio/`, `sequencer/` or `dac/`. **Verified firsthand by
     the overseer with a control** (`0x2b`/`$2b` → grep exit 1 across all three
     trees; control `0x28` → 9 hits in `engine.rs`). So an FM6 track and a DAC
     track sound together in Seraph and **cannot** on hardware — a
     preview-vs-driver divergence of the same class as the overlap
     last-note-priority fix, and the README now discloses it rather than
     implying the conflict is modeled. Distinct from F25 despite both being DAC.
  5. **`export_formats()` advertises `"binary"`** with no implementation.
  6. **`DriverFeature::Fm3SpecialMode` declared supported**, implemented nowhere.
  7. **Stale comment** — `PianoRoll.tsx` says "Draw Mode (F6)" where `F6` is the
     audit item number and the binding is `B`; it reads as a keybinding.
  8. **`get_channel_overlaps`** is backend + wrapper only, no `.tsx` caller.
  **UNDETERMINED, deliberately not guessed:** whether a debug `cargo run` serves
  the built `dist/` or expects a live Vite dev server (`devUrl` is set). The line
  was **removed** rather than documented wrongly; `npm run tauri dev` covers the
  need. TAGGED for foreground follow-up.
  **NAMING, flagged for the owner rather than resolved:** the README's closing
  line described dropping output into the Z80 **Flamedriver**, which is the
  S3K/skdisasm path, not the active `aeon` engine whose driver is **Memra**. The
  code is unambiguous (zero `memra` identifiers; the only driver registered is
  `FlamedriverProfile`), so the README correctly says Flamedriver today. The
  sentence was narrowed to "the SMPS-based ROM-hacking projects" rather than
  re-pointing it at aeon — **that is an intent question, not a code question.**
  Note this now interacts with RULING 2 above: if S0's manifest targets the
  S2-clone/Zyrinx/Flamewing/S1-S2-S3K drivers, Flamedriver-first is closer to
  where the suite is heading than Memra-first was.
- 2026-08-22 (cont.): **F25 + F26 SHIPPED — per-note DAC samples are
  authorable, and auditioning stopped shipping the song** (branch
  `fix/dac-voice-and-audition-cost`, two commits so either reverts alone:
  `ea1adcf` F25, `b7eb13f` F26. Lanes on the branch: cargo **264/0**
  (263 at `f5eb86f` + 1 new), `npm test` **352/352 across 33 files**
  (336/31 at `f5eb86f` + 16 new), `npm run build` clean with **zero
  warnings**, `src/bindings.ts` regenerated by `cargo test` and committed
  with the change, `git status` otherwise clean. The two parcels were the
  owner's pick over a larger piece of work, on the grounds that he hits both
  while composing.)

  **F25 — THE DESIGN CALL, and why the obvious option is the wrong one.**
  The overseer's option space was (a) add a `Dac` variant to
  `LibraryInstrument` so the existing drag-drop just works, (b) a per-note
  sample picker for DAC lanes, (c) per-pitch sample mapping. **(b) shipped**,
  and the code — not taste — is what rules out (a): `FmInstrument` and
  `PsgInstrument` are self-contained parameter structs, which is exactly why
  the library can hash them (`fm_canonical_bytes` / `psg_canonical_bytes`)
  and store them as JSON. **`DacInstrument` is a POINTER**: `pcm_file`,
  `original_file`, `target_sample_rate`, with the audio itself living in the
  project's `instruments/dac/` directory. A library DAC kind therefore is not
  "one more enum arm" — it is PCM asset storage, a content-hash scheme over
  sample data, an extraction path, and a decision about whether library
  entries can carry megabytes. That is a package, not a parcel, and it is not
  what F25 was about. (c) is bigger still AND would set up a second authority
  fighting `note.instrument_id`. So the overseer's guess was right, for a
  reason he flagged he had not checked.
  **What shipped:** a Sample picker in the piano-roll header, rendered on DAC
  lanes only, fed by `list_dac_instruments` (the PROJECT's bank — which is
  already populated for imported songs, since `smps_mapper::resolve_dac_sample`
  creates one `DacInstrument` per SMPS sample byte) and applying to the note
  selection through the existing `set_note_instrument`. **ZERO backend
  change** — the IPC, the kind gate (`check_instrument_kind` already has its
  DAC arm) and the resolver were all already there. This really was
  connecting paid-for work.
  **What F25's fix does NOT fix, stated plainly:**
  1. **F7's row labels.** DAC pitch still selects nothing —
     `process_event`'s `ChannelType::Dac` arm plays the resolved instrument
     and ignores pitch — so the 29 `DAC_SAMPLE_NAMES` rows remain a
     convention. They are NOT arbitrary: `smps_mapper` writes
     `pitch = 36 + (sample_byte - 0x81)` alongside a per-note
     `instrument_id`, so row 36 = "Snare S3" is TRUE for imported S3 content
     and meaningless for from-scratch content. That is why they were
     documented rather than deleted. Making them real is option (c).
  2. **No cross-project DAC voices.** Samples stay per-project; there is
     still no library entry to drag.
  3. **No per-row default.** Building a kit is still select-notes-then-pick
     per drum, not "row 41 is the kick" once.
  4. **The hardware ceiling is untouched, and correctly so.** The Genesis DAC
     is ONE channel. Two drums at the same tick cannot both sound, and
     `check_voice_overlap` rejects an edit that would create that — so a
     simultaneous kick+hat is refused. This is not a defect introduced here;
     it is the chip, resolved at compile time by last-note-priority. The
     picker's tooltip says so, and the rejection surfaces in the existing
     inline notice rather than failing silently. **Grid-snapped painting does
     NOT trip it** (adjacent cells abut, they do not overlap) — only
     genuinely simultaneous drums do.

  **F26 — THE INVALIDATION ENUMERATION, and why nothing is cached.**
  `handleAudition`'s leading `await ipc.listTracks()` served exactly one
  purpose: read `track.instrumentId`. The obvious fix is to cache that off
  `refresh()` (which already fetches it, and already stores
  `hasInstrument: track?.instrumentId != null` from the same reply). **That
  was considered and rejected on the enumeration, not on instinct.** What
  rewrites a track's instrument binding, derived by grepping the callers of
  every mutation that can touch it rather than by reasoning about which ones
  "can" fire (protocol bar 13):
  - `TrackHeader.handleDrop` → `library.libraryAssignToTrack(track.id, hash)`
    → `assign_library_instrument_to_track`, which sets `track.instrument_id`
    AND clears every per-region and per-note override. **NOT covered** — no
    signal reaches the roll. This is the headline case.
  - `TrackHeader`'s five other `ipc.updateTrack` sites (rename, mute, solo,
    volume, pitch offset). Each passes `track.instrumentId` straight back
    from its own prop, so any of them can WRITE the binding — with whatever
    value that component last rendered. **NOT covered.**
  - `TrackHeader` delete track (`ipc.deleteTrack`). **NOT covered** by any
    notification; the roll only finds out when its own `refresh()` fails to
    find the region.
  - `ArrangementView`'s instrument-add path, which binds a new instrument to
    the lowest empty lane of its kind. **NOT covered.**
  - Undo / redo → `SONG_REVERTED_EVENT` → `refresh()`. **Covered.**
  - Region switch → `refresh()` re-runs on `region.regionId`. **Covered.**
  - The roll's own edits (`set_note_instrument`, paint, erase) →
    `refresh()`. **Covered.**
  Four of seven groups uncovered, and `SONG_REVERTED_EVENT` is the ONLY
  cross-component signal that exists in `src/` (one dispatch site,
  `App.tsx`). A cache would therefore audition the previous voice after a
  header voice change — the region-switch staleness class `e01f6d1` swept,
  reintroduced through a different door. **So: no cache.** New narrow IPC
  `get_track_instrument(track_id) -> Option<String>` (backed by
  `ProjectManager::track_instrument_id`) returns the one field. The finding's
  actual claim — "ships the whole song" — is fully answered; the payload goes
  from the entire track/region/note tree to one string, and freshness is
  exact.
  **Per-caller verdict for the audition path**, since a reachability claim is
  an enumeration claim: `onAudition` has exactly four call sites —
  `PianoRollKeys.handleClick` (keys-column click), and in `PianoRollCanvas`
  the note press, the double-click draw and `paintCellUnderPointer`'s
  new-pitch branch. **All four funnel through the single `handleAudition`**,
  so there is no path that reads the binding by another route and no path
  left on the old one.
  **MEASURED, not asserted:** `PianoRoll.auditionCost.test.tsx` counts mock
  calls — five keys-column auditions and a three-row Draw-Mode paint run add
  **zero** `listTracks` calls beyond the mount's own refresh (the paint run's
  single commit refresh aside). What is NOT measured: wall-clock latency and
  real serialized payload size, which need a running app; the improvement
  claimed here is round-trip *payload*, and the round-trip *count* per
  audition is unchanged at two (narrow read + preview).

  **JUDGEMENT CALLS — listed individually so each can be ratified or
  rejected on its own:**
  1. F25 solved as a picker over the PROJECT bank, not a library `Dac` kind
     (see the `pcm_file` argument above).
  2. The picker is DISABLED with an empty selection rather than hinting, so
     the destructive whole-lane reading is unreachable by construction.
     (`handleVoiceDrop` hints instead; the two differ deliberately, because a
     drop has no other affordance and a disabled control does.)
  3. Picking a sample AUDITIONS it (`preview_dac`). Kits are built by ear;
     this is the one part of the parcel that wants the owner's ears.
  4. "Lane default" is offered as an explicit clear (`instrumentId = null`),
     mirroring `set_note_instrument`'s `None`.
  5. Mixed selections show a disabled "Sample (mixed)" placeholder rather
     than silently showing the first note's voice.
  6. `DAC_SAMPLE_NAMES` KEPT, with its derivation documented at the constant,
     rather than deleted — they are true for imported content.
  7. The DAC-lane drop message rewritten to point at the picker. The old text
     named a thing that cannot exist.
  8. The DAC bank fetch is region-TAGGED and refuses out-of-order replies,
     matching how `loaded` is handled in the same file.
  9. Bank refetched on select FOCUS as well as on mount / region change /
     undo, so a sample imported while the roll is open appears without
     reopening.
  10. F26 solved with a new narrow IPC rather than a cache (the enumeration
      above), accepting a small permanent IPC-surface addition.
  11. `get_track_instrument` returns `Ok(None)` for an unknown track id
      rather than `Err` — "nothing to play" is a valid answer for a track
      deleted under an open roll.
  12. The lookup lives on `ProjectManager` (`track_instrument_id`) so it is
      testable without a Tauri `State`; the command is a two-line wrapper.
  13. Collapsing audition into ONE round trip via a `preview_track_note`
      command was considered and REJECTED: it moves fm/psg/dac dispatch into
      the audio path, which cannot be verified here without ears, for the
      saving of one tiny IPC call.

  **INVARIANTS PINNED BY TEST (16 new tests, EVERY ONE proven red-first;
  the poison and the actual failing assertion are recorded):**
  - `PianoRoll.dacVoice.test.tsx` (11). Poisons and what they broke:
    dropping the `isDac` render gate → *"expected <select …> to be null"*;
    dropping the `isDac` fetch guard → *"expected vi.fn() to not be called at
    all, but actually been called 1 times"*; removing the clear option →
    *"expected [ 'Sample (select notes)', …(2) ] to include 'Lane default'"*;
    sending `""` as an id → *"expected vi.fn() to be called with arguments:
    [ 'track-1', 'region-1', …(2) ]"*; removing the audition → *"expected
    vi.fn() to be called with arguments: [ 'dac-hat' ] — Number of calls:
    0"*; reloading before the backend call → *"expected vi.fn() to be called
    at least once"*; swallowing the rejection → *"expected 'Drums | Bars
    1-42 notesSample (mixed)…' to contain 'voice-overlap'"*; restoring the
    old drop text → *"expected 'Drums | Bars 1-4Only DAC voices can b…' to
    contain 'Sample picker'"*; dropping the out-of-order guard → *"expected
    [ 'Sample (select notes)', …(1) ] to include 'Hat.wav'"*; enabling the
    picker with no selection → *"expected false to be true"*; never deriving
    the selection's voice → *"expected '' to be 'dac-hat'"*; collapsing a
    mixed selection to the first note → *"expected 'Kick.wav' to be 'Sample
    (mixed)'"*; aiming the write at `[]` → *"-[0,1] +[]"*; removing the bank
    options → 4 failures including the bank test.
  - `PianoRoll.auditionCost.test.tsx` (5). Reverting to `listTracks` →
    *"expected vi.fn() to be called 3 times, but got 0 times"* + *"expected 2
    to be 1"* (the listTracks counter) — i.e. the tests fail on the ORIGINAL
    code, which is the point. **Caching the binding once** → 5 failures
    including *"expected last vi.fn() call to have been called with
    [ 'fm-voice-2', 60 ]"*: the freshness pin catches exactly the design that
    was rejected. Removing the unbound-lane guard → *"expected vi.fn() to not
    be called at all, but actually been called 1 times"*; routing PSG through
    the FM preview → *"-psg-voice +fm-voice"*; ignoring the DAC per-note
    override → *"-dac-kick +fm-voice"*.
  - `manager.rs::test_track_instrument_id_agrees_with_list_tracks_through_every_rebind`:
    the narrow read is asserted EQUAL to the `list_tracks` expression it
    replaces, recomputed on every line (bound / rebound / unbound / track
    deleted / never existed) rather than written out, so the two cannot
    drift. Poisoned to `self.tracks.first()` → *"assertion `left == right`
    failed: left: Some(24be7915-…) right: None"*.

  **DEFERRED / NOT DONE (each a separate decision):**
  - Per-pitch DAC sample mapping (option (c), the rest of F7). Needs a model
    concept, persistence, migration and an owner ruling on whether it
    supersedes or coexists with `note.instrument_id`. NOT started.
  - A library `Dac` kind (option (a)). Blocked on PCM asset storage design,
    as above. NOT started.
  - `preview_track_note` (one round trip instead of two). Rejected here as
    unverifiable-without-ears surface, not as a bad idea.
  - The known unidentified vitest flake (1 failure in ~6 full runs, name
    unrecovered) was **not reproduced** in any run this session; it stays
    booked and unidentified.

  **NEEDS THE OWNER (audio, so untried here — standing rule: no emulator
  from a background agent, ever):**
  - Does picking a sample from the header picker SOUND right — the picked
    sample, at the right pitch-independent rate, and immediately?
  - Build a kick/hat/snare kit on one DAC lane from scratch and play it back:
    do the per-note samples come out of `build_snapshot` as authored?
  - Does audition FEEL faster while painting a run across rows? That is the
    half of F26 no test here can report on.

- 2026-08-23: **F27 INVESTIGATIONS DISPATCHED (second attempt) — and the first
  pair's silent death is the transferable part.** The 2026-08-22 session
  dispatched two research agents for F27 and wrote them into
  `docs/lane-status.json` as `inFlight`. The owner then relaunched all six lanes
  from the Dominion console, which cleared that session. **Background subagents do
  not survive a session rotation.** Both branches
  (`research/f27-driver-truth`, `research/f27-exposure-map`) were found at
  `9e7695f` with **zero commits** and **clean worktrees, nothing untracked** — the
  agents died before writing anything. Neither branch nor worktree is recoverable
  work; both were removed and re-created fresh.
  **Two things made this detectable, and one of them was missing.** (1) Branch
  state: `git rev-list --count main..<branch>` = 0. **That result is two-valued**
  (protocol bar 16a) — "never had commits" and "already merged" produce identical
  output — so it was disambiguated with `git log <branch>`, which showed only
  main's own commits, plus `git status --porcelain -uall` in each worktree showing
  nothing written. (2) **A queue Log entry, which did not exist.** The dispatch was
  recorded ONLY in `lane-status.json`, a file whose whole purpose is to describe
  the *live* session — so when the session died, the only record of what it had
  started died with it in every sense that mattered, leaving a status file
  asserting activity that no longer existed. **The status file is the volatile
  record; this Log is the durable one. A dispatch goes in both.**
  Re-dispatched with the same split, both read-only, both delivering a COMMITTED
  report so that a second rotation cannot erase the result:
  - `research/f27-driver-truth` → `docs/research/2026-08-23-f27-driver-truth.md`.
    What aeon's driver actually does with the channel-6 steal, read at aeon
    `origin/master` = `139995f256f5e50c26d2053c229dd09b5e70c84d` (`ls-remote`-verified
    2026-08-23T07:55Z) via `git show <rev>:<path>` — **never** through the sibling
    path, since the aeon lane is live in that tree right now. Plus what the
    established drivers do, from the disassembled Z80 blobs under aeon
    `docs/research/z80_blobs/`.
  - `research/f27-exposure-map` → `docs/research/2026-08-23-f27-exposure-map.md`.
    Every Seraph surface where the clash shows up (preview, `build_snapshot`,
    each export path, `check_voice_overlap` / the uncalled `get_channel_overlaps`,
    the UI affordances F25 shipped), with the three fix options — silent steal /
    visible warning / authoring-time gate — priced individually.
  The briefs deliberately do NOT share a frame: one enumerates over the driver's
  source, the other over Seraph's call graph, and neither is told the other's
  conclusion (protocol bar 19 — two agents given the same brief share it by
  construction, so agreement between them would be echo, not corroboration).
  **The fix choice is NOT delegated.** It is a design call about what the preview
  is allowed to promise, and it goes to the owner with numbers.

- 2026-08-23 (cont.): **BOTH F27 INVESTIGATIONS LANDED — AND F27's PREMISE WAS
  WRONG.** Merged `75fdd1a` (driver truth) + `6852454` (exposure map); findings
  banked in the DAW-feel audit at `32cf763`. Docs-only merges, no code touched, so
  the three verification lanes were not re-run — stated rather than implied.
  **Full detail is in `docs/superpowers/2026-08-21-daw-feel-audit.md`, section
  "F27 — GROUNDED 2026-08-23"; this entry is a pointer, not a second copy.**
  Headline: aeon **does** write `$2B`, at four sites, with a three-mode per-song
  ch6 contract (DEDICATE / FM6-FM / ADAPTIVE). **Hardware CAN sound FM6 and DAC in
  one song** — alternating, in ADAPTIVE — so the F27 row's "real hardware cannot"
  is false as written. That row is deliberately LEFT UNEDITED with the correction
  in its own section, so the change of understanding is visible rather than
  laundered into the original claim.
  **Every load-bearing agent claim was re-verified firsthand by the overseer before
  banking** (the `$2B` write sites and the `SND_FM6_ADAPTIVE` gate in aeon at
  `139995f`; `OPN2_ReadChannels` summing `ch_out[0..6]` blind to `dacen`;
  `vgm.rs`'s `Dac(_) => continue`; `vgm_import.rs`'s `hw_ch == 5 && dac_enabled`;
  `check_voice_overlap`'s `channel_key` narrowing; the NUL byte and its
  tree-wide enumeration). Two claims are marked carried-not-verified in the audit
  rows themselves rather than silently promoted.
  **NEW FINDINGS BOOKED: F28–F31.** F28 is the one with reach beyond F27 —
  `src/components/PianoRoll.tsx` contains a NUL byte (`MIXED_VOICE = "\0mixed"`),
  so grep treats the file as **binary and skips it silently**: `grep -c` exits 1
  with no output where `grep -ac` returns 7. It is the **only source file in the
  tree** with a NUL (18 tracked files have them; 17 are icons — enumerated, not
  assumed), and it is the 907-line note-editing surface. **Every past frontend
  enumeration in this repo that omitted `-a` excluded it and returned a clean
  empty result while doing so** — protocol bar 16(d) living permanently in the
  tree rather than arriving in one command. One-character fix; the value is
  re-running past sweeps.
  F29 VGM export drops every DAC note (`Dac(_) => continue`) — **must land before
  the booked README-7 VGM wiring**, or that fix ships a working button whose first
  output has no percussion. F30 SMPS export emits both an FM and a DAC header for
  index 5. F31 `FlamedriverProfile::channel_layout()` advertises six FM voices
  *including* `FM6/DAC` **plus** a separate DAC channel — seven voices on a
  six-voice chip — and it is the tree's ONLY `DriverProfile`, is S3K's (which has
  no FM6 music voice at all), and there is no aeon/Memra profile. F31 is upstream
  of F27 and wrong however F27 resolves.
  **THE TWO AGENTS DISAGREED ON THE FIX, AND THAT IS THE RESULT.** The exposure
  map recommends a key-off-FM6 steal plus a diagnostic; the driver read shows that
  is right in ADAPTIVE and actively wrong in FM6-FM, where the loss is permanent
  and a restoring preview would sound BETTER than hardware. Both correct in their
  own frame. The reconciliation: **Seraph has no song-level ch6-mode field**, so it
  cannot express which contract a song targets, and no amount of source reading
  decides what a from-scratch Seraph song should default to. **PARKED FOR THE
  OWNER as a model-design call.** Bar-19 note: the two derivations enumerated over
  different parameters (aeon driver source + Z80 blobs vs Seraph's call graph) with
  neither brief carrying the other's conclusion, so their agreement on the VGM
  defect is corroboration and their disagreement on the fix is real, not a frame
  artifact.
  **TAGGED, NOT ATTEMPTED — no emulator from a background agent, ever:** trace
  whether post-sample `$28` writes land on a chip-muted ch6 in FM6-FM mode (the
  driver read calls this inference, not observation); and confirm by ear in Seraph
  that an FM6 sustain and a drum hit audibly coexist.

- 2026-08-23 (cont.): **PUSH AUTHORIZATION — OWNER RULING, RELAYED BY THE HUB, NOT
  WITNESSED BY THIS LANE.** *"A lane may push its own repo's master without asking each
  time."* Chosen by the owner over two narrower options (standing-for-docs-ask-for-code;
  per-push), on a suite-wide question the empyrean hub consolidated from two lanes stopped
  on it separately — sigil asked outright, aeon was holding finished commits for the same
  reason. **Conditions ride with the grant** and are transcribed rather than paraphrased:
  verify `origin` actually MOVED (the push is not the act, the remote moving is); never
  rewrite already-pushed history; never push another lane's repo; publication to the public
  wiki site stays a separate explicit ask.
  **Provenance, stated because this lane's standing rule is that a relayed grant is not a
  witnessed one:** the granting act reached seraph through empyrean-18, not from the owner
  directly. **Anchor verified firsthand here** — empyrean `2bd72a03` is an ancestor of
  empyrean `origin/main` (freshly fetched), and `git show --stat` confirms it is a
  **docs** commit (`docs/OVERSEER.md`, +23) — which is the correct SHA *class*, since what
  it anchors is a ruling record and not a code guarantee. The hub's relayed wording was
  read against the banked text and matches; nothing was added in transit.
  **SCOPE — it authorizes PUSHING, not the work being pushed.** It does not release this
  lane from its boot stop, is not approval to dispatch, and does not touch anything already
  parked with the owner: F27's ch6-mode design call, the five open ear-gates, or the S0
  driver-list question all remain his.
  **What it changes here: nothing operational.** Seraph's landing lane already pushes `main`
  as the last step of every landing and has done so throughout (`origin/main` was
  `ls-remote`-verified at each push, which is condition one already being met). Recorded so
  the *authority* for that practice is banked rather than assumed — the practice was
  correct, its warrant was undocumented. The wiki condition is inert in this repo: seraph
  publishes no `wiki/dist/`.

- 2026-08-24: **THE DECISIONS FORMAT IS IN FORCE, AND THIS LANE'S FOUR BLOCKERS ARE
  NOW STATED RATHER THAN NAMED.** Owner instruction, relayed by the empyrean hub;
  **relayed, not witnessed by this lane** — recorded that way per this repo's standing
  rule, and it costs nothing here because the instruction is a *format* rule and
  carries no authorization. The hub said so itself in the message.
  **Anchor verified firsthand before acting on it:** empyrean `origin/main` =
  `94ea23982df466c04b125e44cf6513a3267741ee`, checked with `git ls-remote` against the
  remote rather than a local ref or the sibling working tree, and `contract/DECISIONS.md`
  + `contract/LANE_LOG.md` read at that revision via `git show <rev>:<path>`. Bonus from
  the same check, worth banking because it retires a doubt rather than raising one: the
  protocol blob at that SHA is `0e55e265…`, **byte-identical to the one this session
  booted on**, so the boot read was already current and no bar moved underneath it.
  **What landed (`9cdbb4a`):** `docs/decisions.jsonl`, four entries, one per existing
  `blockedOnOwner` row, each with the problem stated in his vocabulary, two or more
  options carrying a cost he can picture, and a recommendation. `blockedOnOwner[].id`
  in `docs/lane-status.json` points at them. d-1 the FM6/DAC ch6 model call (F27),
  recommending the song-level mode field on the argument that the two cheaper options
  are each silently wrong for a subset of songs; d-2 the three F25/F26 ear gates;
  d-3 S0's driver list; d-4 the two F6 Draw Mode gates.
  **The shape call, made here and flagged to him rather than hidden:** d-2 and d-4 are
  ear and eye checks, and **a question only his ears can answer has no options for him
  to weigh**, so forcing the literal check into an options list would have invented a
  choice that does not exist. Both are instead written as decisions about *how much of
  his attention to spend* and what this lane does under each answer. The hub proposed
  this shape and explicitly did not rule it; the call is this lane's, and d-2 says so in
  its own text so he can reject it without reading a Log.
  **Contract compliance details that are mechanism, not habit:** every `at` and
  `updatedAt` came from `date -u`, never from this session's sense of the time (rule 7,
  and the reason it exists is that a model has no clock); the barred em and en dashes
  were checked for **programmatically over every prose field before the file was
  written**, not proofread afterwards; and the recommendation key was asserted to name
  one of the entry's own options, since a recommendation pointing at nothing rejects the
  whole entry and looks like guidance while being none.
  **KNOWN GAP, STATED SO IT IS NOT SILENTLY ABSENT: `docs/lane-log.jsonl` is equally
  required by the ratified contract and DOES NOT EXIST in this repo.** The owner's
  relayed text asked for the decisions half only, so writing a retroactive lane log was
  not in the ask and was not done. Not an oversight; an unfilled obligation, and the
  distinction is the whole point of writing it down.
  **`docs/OVERSEER.md` now carries a pointer to both files**, because the rule requiring
  them reaches a lane only through the `/overseer` skill, which lives outside every repo
  and outside version control — a hole `DECISIONS.md`'s own Status section names as open
  and parked with the owner. Without that pointer a cold boot of this repo would read the
  whole boot doc and never learn either file exists. It is a pointer with its citation
  and a "that document governs" line, deliberately not a copy of the rules.

- 2026-08-24 (cont.): **THE LANE-LOG GAP WAS RULED WITHIN THE HOUR, AND THE RULING IS
  NOT WHERE IT SAYS IT IS.** Hub ruling, relayed: **open `docs/lane-log.jsonl` at the
  next landing; do not backfill**, and a lane with nothing landed writes nothing rather
  than writing that it wrote nothing. The entry above is left unedited and corrected
  here, per this repo's habit of not laundering a change of understanding into the
  original claim: what that entry called an unfilled obligation is, by this ruling,
  correct behaviour. **The reasoning is worth keeping** — a log reconstructed after the
  fact out of an overseer doc is a confident guess wearing a record's clothes, and it is
  precisely the failure `LANE_LOG.md` exists to prevent. It also retires the question of
  whether to open the file with a ceremonial empty entry: no.
  **The hub's own measurement, banked because no single lane could have seen it:**
  `decisions.jsonl` present in **6/6** repos within the hour of the relay (this lane
  filed four, the most of any); `lane-log.jsonl` present in **1/6**, empyrean only. The
  oracle lane found the gap against itself and predicted the other five correctly. Two
  lanes reaching it independently is corroboration rather than echo (protocol bar 19):
  oracle enumerated over its own repo's files, this lane over the contract's own
  requirements table while reading it for a different reason.
  **ANCHOR DEFECT, VERIFIED FIRSTHAND AND RAISED WITH THE HUB.** The ruling is banked at
  empyrean `3ca38ac`, which IS reachable from `origin/main` (`ls-remote` + ancestor check
  both run here). But `git show --stat` puts its whole content in the hub's own
  `docs/lane-log.jsonl` and `docs/lane-status.json`; **`contract/LANE_LOG.md` is
  untouched, and grepping it at `origin/main` for backfill, next landing, heartbeat and
  nothing-landed returns no hits.** So a binding cross-lane rule is anchored to a commit
  whose class is one lane's narrative log — the same defect the protocol names for a docs
  SHA standing in for a code guarantee, one level over. It matters concretely rather than
  pedantically: **the next cold lane will read `LANE_LOG.md`, find no backfill rule, and
  either backfill against the ruling or invent its own answer**, and nothing in that path
  passes through the commit that carries it. Raised with the hub with the grep; the fix is
  theirs to land, since the contract is theirs.

- 2026-08-24 (cont.): **RULE 8 LANDED WHERE LANES READ IT; AND THIS LANE SENT A
  MECHANISM IT HAD NOT MEASURED.** The anchor defect flagged above is closed:
  `contract/LANE_LOG.md` now carries **rule 8** (open the log at the next landing, never
  backfill, a lane that has landed nothing writes nothing). Three lanes — aeon, aurora
  and this one — each ran the grep themselves rather than trusting the SHA, and each sent
  it inside the same window; aeon's landed first. **Bar 19 note: that is corroboration
  rather than echo only because the enumeration parameters differed** — oracle found the
  gap by enumerating its own repo's files, this lane by reading the contract's
  requirements table for an unrelated reason, and none of us was working from another's
  conclusion. `docs/OVERSEER.md` re-pointed at rule 8 and the caveat deleted.
  **Citation precision, kept because it is the same defect one notch milder:** the hub
  cited `2c587f2`. That SHA is the branch tip and carries rule 10 in `DECISIONS.md`; it
  does not touch `LANE_LOG.md` at all. **Rule 8's carrying commit is `2e50643`**
  (`-S`-located, `--stat`-confirmed), and that is what this repo cites. Readable-at is
  not carried-by; the protocol's "a SHA has a class, a path has a time" covers exactly
  this, and citing a tip works right up until the tip moves.
  **THE PART THAT IS THIS LANE'S OWN ERROR, banked because it landed inside a message
  about verification discipline.** Reporting that a 120-character `focus` limit is
  unasserted, this lane supplied a mechanism for it: *a lane copying the ratified example
  verbatim can overflow it.* **That mechanism is false — the example is 69 characters**
  (measured here at `origin/main`, and it is in fact this repo's own focus line from an
  earlier session). The overflow was authored, not copied; the 121-character line was one
  this session wrote. **The finding survived and the explanation did not**, and the hub's
  own measurement made the finding much harder than it was sent: **three of six lanes
  exceed the limit — oracle 128, aurora 128, empyrean 227 against a stated 120** — with
  the format's owner worst by nearly double. Booked there as their `FMT-LINT`.
  **The transferable half: a finding and its explanation are separately checkable, and
  the explanation is the cheap one to invent.** This lane measured its own string, then
  supplied a cause for it without measuring the cause, in the same message that corrected
  someone else's inference-dressed-as-measurement. It is protocol bar 17 with the subject
  changed from completeness to causation: **a mechanism offered alongside a verified
  finding inherits the finding's credibility for free.** Nothing marked it as the unverified
  half, including to the sender.

- 2026-08-24: **FIRST OWNER ANSWER THROUGH THE CONSOLE, AND A QUESTION BACK ON THE
  BIGGER ONE. WITNESSED IN THIS SESSION, not relayed.**
  **d-4 SETTLED: the two F6 gates (Draw toggle legibility, paint-drag audibility) are
  ruled "leave both alone until they annoy you in real use"** — this lane's own
  recommendation, accepted. Recorded by appending **d-5** (supersedes d-4) and the
  blocker dropped from `lane-status.json` in the same act, which is the only receipt the
  console ever gets. **Do not re-raise these.**
  **SCOPE CATCH, and it is the reason to read a settled card carefully rather than
  filing it:** the card named TWO gates. The original F6 Log entry above names **THREE**
  — the third being whether a painted run RENDERS correctly, which no test can see
  because jsdom has no 2D context. `OVERSEER.md`'s gate list only ever carried two of the
  three, and the card inherited that omission. **His answer therefore does NOT cover the
  third**, and stretching it to cover a gate he was never shown is precisely the
  laundering `DECISIONS.md` rule 8b bars. Left open and deliberately unfiled: filing a
  near-identical card minutes after he ruled on its siblings spends his attention badly.
  Raised with him in prose instead. Note where the drift came from: the boot doc's own
  snapshot, which that doc warns about at its top, biting the doc that carries the
  warning.
  **FORMAT GAP, raised with the hub rather than worked around:** `DECISIONS.md` has no
  `outcome` field and no shape for a closed card, while the console's answer prompt asks
  the lane to "record the outcome in `docs/decisions.jsonl`". Rule 8 forbids rewriting the
  settled line, and appending an entry with an invented question would be worse. Encoding
  chosen: append a `supersedes` entry carrying the **identical** question, options and
  recommendation, with the outcome in `detail`. Nothing is invented, and because the
  blocker is dropped no card re-renders. Every lane hits this on its first answer.
  **d-1 NOT ANSWERED — HE ASKED A QUESTION, AND HE EXPLICITLY SAID TO KEEP IT LISTED.**
  Per `DECISIONS.md`, a question leaves the blocker standing; it stands. His question:
  *"Maybe we should have the engine determine what it does? Like if sonic 1 always eats a
  channel for DAC, if we're using that engine that's how it should behave right?"*
  **He is right, and the evidence for it was already sitting in this repo unread against
  this question.** `docs/research/2026-08-23-f27-driver-truth.md` §2 read paired `.lst`
  disassemblies for five drivers: S3K Flamedriver has **no FM6 music track at all** (the
  init table's own comment says so); Batman excludes ch6's voice per sub-frame from a
  state flag; Alien Storm, Gunstar and TF4 toggle `$2B` per sample; MDSDRV (second-hand,
  marked) collapses slot 5 to FM6-or-PCM1 so its format cannot express both. §2.3:
  **nobody lets FM6 and the DAC sound simultaneously, and no driver read represents that
  state at all.** So the driver determines it in every established case, and aeon's Memra
  is the *only* driver found that offers a per-song choice.
  Recorded by appending **d-6** (supersedes d-1) with his option added as
  `driver-decides` and `recommend` re-pointed at it, per rule 8b — **added, never mapped
  onto the nearest option already offered**, since mapping would launder his answer into a
  choice he did not make. The status file still names `d-1`; the reader follows
  `supersedes`, which is exactly what that pointer is for, so nothing needed re-pointing.
  **CAVEAT KEPT IN THE CARD: Sonic 1 specifically was NOT among the five blobs read.** His
  instinct holds across all five that were; S1 itself is untested here. Stated rather than
  quietly generalised, because his example is the one thing in his question this repo
  cannot confirm.
  **CONSEQUENCE FOR SEQUENCING: this promotes F31 from cleanup to the first step of the
  fix.** Under `driver-decides` the driver profile is the thing that carries the answer,
  and seraph's only profile is `FlamedriverProfile`, advertising seven voices on a
  six-voice chip, with no aeon/Memra profile at all. It must be right before anything
  reads it. The already-proposed order (F28, then F31) lines up with his thinking by
  accident rather than design, which is worth noting so a future session does not read
  the alignment as evidence.

- 2026-08-24 (cont.): **THE CLOSING ENCODING IS RULED, AND IT IS THIS LANE'S —
  `DECISIONS.md` RULE 8c**, verified firsthand at empyrean `829d3ac` (`git log -S` over
  the file to get the carrier rather than the tip, which it also happens to be).
  `d-5` above is now the worked example of a ruled convention rather than a local
  workaround, and `OVERSEER.md` cites the rule so a future session here does not
  re-derive it.
  **The part worth keeping is why the alternative was worse, because this lane could not
  have worked it out alone.** The hub hit the identical gap an hour earlier and invented
  a top-level `outcome` object without noticing. **A consumer that rebuilds each item
  from a fixed key set — the design that stops an unvalidated field reaching a UI —
  drops an unknown key silently.** So that encoding loses the outcome at the reader while
  looking complete on disk; the producer cannot detect it and the consumer structurally
  cannot report it. Bar 16(d) in a new surface: **an absence with no artifact to
  re-examine.** The `supersedes` form was preferred for costing the consumer nothing,
  not for being tidier.
  **This lane's objection survived into the rule rather than being adopted away:** it is
  a supersession that supersedes nothing, a slightly dishonest use of the pointer. The
  rule states its own limit and books a first-class `answered` field as a coordinated
  change with the console, deliberately not adopted yet, since adding a field the reader
  does not parse trades a clean record for a broken card.
  **Bar 19 check on the convergence, since two lanes agreeing is exactly what this
  document keeps warning about:** the hub and this lane reached the same gap from
  different parameters (their own first answer, an hour apart, neither aware of the
  other) and reached DIFFERENT encodings. That is not corroboration and was never
  claimed as such; it is two independent derivations disagreeing, which is why a central
  ruling was the right resolution rather than either lane's habit spreading by example.

- 2026-08-24 (cont.): **d-1 SETTLED BY THE OWNER — `driver-decides`, closed as `d-7`.**
  Answered through the console 2026-08-24; he chose the option that was his own
  (added under rule 8b as `d-6`) and this lane's recommendation. Closed per rule 8c:
  `d-7` supersedes `d-6` with identical question, options and recommendation, outcome in
  `detail`, blocker dropped from `lane-status.json` in the same act. The audit doc's F27
  section carries the ruling above its grounding, unedited below. **What it rules:** the
  driver profile carries channel-6 behaviour and Seraph follows it; a song carries no
  channel-6 mode. **Not ruled, deliberately:** a Memra-scoped setting (the only driver
  found offering DEDICATE / FM6-FM / ADAPTIVE per song) gets its own card when Memra
  playback exists. **Sequencing:** F31 is step one, F27 implementation behind it.
  Committed 2026-08-26T01:53:44Z by a boot session that found the ruling written 2026-08-24 but
  sitting uncommitted for two days (BANK-D7). No parcel dispatched; the owner named other
  lanes to continue tonight and this lane was not among them.

- 2026-08-26: **S2 HAS LOST ITS INSTRUMENT, AND THAT IS THE REAL NEWS OF THE DAY.**
  Relayed by oracle (their `d-12`), NOT verified here against their tree, and deliberately
  not verified: checking it means calling the very tools that are the hazard below.
  **What they report:** three of the four audio methods this lane would reach for are **not
  served by the new emulator at all** — `emulator_vgm_start/status/stop`, `audio_spectrum`,
  and the channel-state/mask pair (the last needs a synth not compiled into the bus server).
  They are on oracle's unserved list and would **refuse by name** today even from a private
  instance.
  **Why that is load-bearing rather than a catalogue note, verified firsthand HERE in our own
  plan:** `plans/2026-07-03-s2-verification-gate.md:10-16` makes side B of the A/B gate
  *entirely* `emulator_vgm_start/stop` → `vgm2wav`, compared by envelope and spectrum
  correlation. That is not one convenience among several; it is the whole of side B. **So S2
  as banked is not executable against the new Oracle**, and the failure would present late —
  at the runbook step, after the compiler work it gates is already done.
  **NOT a blocker today and must not be written up as one:** S2 sits behind S0 and S1, both
  unopened, so nothing is waiting on it. This is booked so that whoever opens S2 meets it
  first rather than at the runbook, and so the ask can be filed on a real schedule instead of
  as an emergency.
  **The ask was NOT filed as a queue item in oracle.** Their standing invitation is genuine
  (the owner's direction is that instrument asks from other lanes are first-class work there),
  but protocol bar 18 scopes notification to a **live** dependency and this one is two
  unopened packages away. Filing now would put dated work on another lane's board for a
  consumer that does not exist yet. What was sent instead: the three method names, what S2
  uses each for, and the condition that fires the ask — **S1 landing**, which is the point the
  dependency becomes real. Recorded here so a future session can see the ask was shaped and
  timed, not forgotten.
  **Also banked, in `docs/OVERSEER.md` rather than here because it is a boot-time hazard:**
  every suite session started before `oracle-old` `07314aa` runs an MCP shim that dials the
  owner's on-screen player instead of a private emulator, so any `mcp__oracle__*` call from
  this session would pause or write into the game he is playing. `/clear` does not restart
  that process; this session's own shim predates the cutover by **40 minutes**. Corrected the
  boot doc's flatly-wrong sentence on this (`e9e1f5a`, timezone fix `836a7e39`) and gave the
  suite the discriminator that actually works — **does the shim have an `oracle-aether`
  child** — since at a 40-minute margin the clock misclassifies in both directions. The
  four-hour timezone error was made HERE first, in a commit, and caught with `date +%z`.

- 2026-08-26 (cont.): **THE ENTRY ABOVE UNDER-RATED ITS OWN EVIDENCE, AND THE REASONING THAT
  MADE IT DO SO IS THE PART WORTH KEEPING.** It recorded the unserved-audio-methods fact as
  "relayed, NOT verified here, and **deliberately** not verified: checking it means calling the
  very tools that are the hazard". **That sentence is wrong, and comfortably so.** Calling the
  tools is one instrument for the question; **their source is another**, and it was reachable
  the whole time by a read that touches no socket. A hazard on one instrument was allowed to
  read as a hazard on the question. This is protocol bar 9 inverted — not changing the subject
  to suit the instrument, but **abandoning the subject because one instrument was unsafe.**
  **Now VERIFIED FIRSTHAND** at oracle `origin/main` = `903a08fe33776cc96b73a9c8e449efd2fc28cc2e`
  (`git show <rev>:<path>`, never the sibling working tree):
  `crates/oracle-aether/tests/schema_conformance.rs:403` declares
  `SCHEMATIZED_NOT_ADVERTISED`, an 18-entry list of methods that are schematized and **not
  served**. All four this lane cares about are in it: `emulator/vgm_start`, `vgm_status`,
  `vgm_stop`, `audio_spectrum`, plus `get_channel_states` / `set_channel_enabled`. The list is
  **asserted exactly** (`assert_eq!(schema_only_sorted, expected_schema_only)`), and its own
  comment records that methods leave the set *by being served*, forced red on the commit that
  ships each handler. Independently, `crates/oracle-aether/src/engine.rs:1415` has the server
  advertise `"vgm": false` in its handshake `capabilities`, under a comment naming those as the
  groups this build does not implement and telling clients to branch on them.
  **So the S2 conclusion HARDENS: side B has no instrument, and that is machine-enforced in
  oracle's own tree rather than a booked reading.**
  **The distinction that makes this bar 10 rather than a bigger hammer:** oracle hedged, in good
  faith, that the channel pair "needs a synth not compiled into the bus server" was *read, not
  run*. **That hedge is correct and belongs to the REASON, not the VERDICT.** The verdict
  (unserved) is asserted by their test suite; the mechanism (why) remains their unverified
  reading and is NOT claimed here. A gate's verdict and its stated reason are separately
  checkable, and the hedge on one was being carried as a hedge on both — by them offering it and
  by this lane accepting it.
  **Frame check per bar 19, since agreeing with a peer is exactly what this document warns
  about:** their derivation came from a survey agent reading the tree; this one enumerated the
  test's asserted literal and the handshake capability block. Different parameter, same answer,
  so this is corroboration rather than echo. Note also that `audio_spectrum` greps to test files
  only — consistent, but that is an absence and is NOT what the claim rests on; list membership
  is.

- 2026-08-27: **THE OWNER'S ANTI-PIN DIRECTIVE COSTS THIS LANE NOTHING — CHECKED, NOT
  ASSUMED, AND THE NEGATIVE IS THE FINDING.** The hub relayed an owner directive spoken to
  aurora and quoted by that lane (the hub did not hear it firsthand, and neither did this
  lane; **relayed twice over**): stale pinned aeon clones in aurora's scratchpad stood
  between him and running what he had just built, and he wants pins gone from his path.
  Ledgered centrally as Q-28, *a pin created for a parcel dies with the parcel*, banked at
  empyrean `45d1f8d`. The ask to this lane was narrow: does any pinned tree, fixture or
  driver snapshot here sit on his path to testing?
  **It does not, and here is the enumeration rather than the verdict.** (1) No aeon clone
  exists under this repo — a `find` for `.git` returns exactly two entries, neither an aeon
  pin (below). (2) No vendored driver blob, manifest or golden is tracked: `git ls-files`
  filtered for aeon/memra/manifest/`.bin`/blob/golden/corpus returns **seven files, all
  prose docs**. That is expected rather than lucky — S0 has not run, and the manifest
  generator it would create does not exist yet, so there is no artifact to go stale. (3)
  The driver profile is **compiled in**, not loaded from a pinned snapshot
  (`registry.register(Box::new(FlamedriverProfile))`, `src-tauri/src/lib.rs:181`), so there
  is nothing to unpin even in principle. (4) This lane's audio path is its own Rust engine,
  so no ROM or emulator sits on the launch path at all before S3.
  **Why this was worth running rather than answering from the boot doc**, which already
  says all of (4): that is bar 24's shape in the cheap direction — the question had a second
  instrument (this tree, two commands) and answering from a remembered document would have
  been an unchecked assertion about my own repo shipped to a peer with my confidence on it,
  which is exactly bar 20.
  **Two directories the sweep turned up, both dead weight and NEITHER a pin on his path,
  recorded so the next sweep does not re-derive them.** `.claude/worktrees/agent-ad49279747a861166`
  holds `feat/view-state-persistence` — the erroneously-dispatched F15 parcel. It is Q-28's
  own shape (a worktree outliving its stopped parcel) but the *branch* is what the F15
  reversal contingency preserves and the branch survives the worktree, so the directory is
  disposable and the preservation is not. Not deleted: outside the directive's stated scope
  and not in his way. `.worktrees/phase4-sequencer` is **108 MB** and its gitdir pointer
  names `/home/volence/sonic_hacks/megadaw/.git/worktrees/…` — **a repo that no longer
  exists on this machine**, so it is a dangling worktree from a predecessor project, dated
  May. Also not touched, for the same reason, and flagged to the owner rather than cleaned
  on a peer's say-so.
  **ADJACENT FINDING, BOOKED AS F32, NOT IN THE DIRECTIVE'S SCOPE AND NOT ACTED ON.** The
  only absolute paths anywhere in this tree's source are two, both in
  `src-tauri/src/import/mod.rs` (`:1461`, `:1502`), both naming a Batman & Robin ROM under
  this user's home, and both inside `#[test]` bodies that `return` early when the file is
  absent after printing `Batman ROM not found, skipping Zyrinx test`. So on any tree without
  that ROM — a fresh worktree, another machine, CI — `test_import_zyrinx_batman_main_title`
  and `test_import_all_zyrinx_songs` **report green while executing none of their
  assertions**, and the aggregate count this repo treats as monotonic absorbs the loss
  without a mark. That is protocol **bar 25** (a green log and an absent run are the same
  artifact) standing permanently in the tree rather than arriving in one command, and the
  notice it prints is the `skipping …` wording rather than the `skip:` form a grep-based
  bar would catch — sigil's `SKIP-TEXT-HOLE` shape, met here independently, which makes it
  corroboration rather than echo per bar 19. **Size S; the fix is a named fixture path plus
  a hard failure when the harness expects the ROM and cannot find it, so the skip is a
  decision rather than an accident.** It is genuinely NOT on his path to testing: these are
  tests, not the app, and this booking must not be read as agreeing that a pin blocked him
  here. **Nothing was owed back on the directive; a clean negative was sent anyway, because
  the hub is assembling an answer for him across six lanes and "nothing here" is an answer.**

- 2026-08-27 (cont.): **THE WORKTREE SWEEP — 0 REMOVED, 1 KEPT, AND THIS LANE'S OWN
  CLAIM WAS THE THING THAT NEEDED CORRECTING.** Owner directive, this time heard
  firsthand in the hub dock and relayed (his words: *"Can we get everyone to make sure
  theyyy're all merged?"*), after he saw aurora's 20-plus leftover worktrees at 856 MB and
  575 MB. Instruction: test each agent worktree against main with
  `git merge-base --is-ancestor`, remove the merged, report the unmerged by name and size.
  **THE CORRECTION FIRST, because it is the load-bearing part.** An hour earlier this lane
  told the hub the F15 worktree was *"disposable, because the branch is preserved"* — and
  derived that from `git worktree list` **without opening the directory**. The branch half
  was true. The directory half was false: it held a modified `src/App.tsx` **and an
  untracked 271-line `src/App.viewState.test.tsx` that existed nowhere else in the world.**
  The hub then reflected that reasoning back as *"the stopped F15 parcel worktree qualifies
  if its branch is preserved"*, i.e. **this lane's unchecked claim came back as
  authorisation to delete the thing it was wrong about.** Acting on it would have destroyed
  271 lines of tests. Bar 20 exactly: the wrong claim lived only in mail, nothing in this
  tree contradicted it, and no sweep or audit here could ever have met it.
  **This is also the sharpest available case for bar 16's name/presence/behaviour split.**
  `git worktree list` names a directory; it says nothing whatever about that directory's
  contents, and "the branch is preserved" is a claim about a **ref**, not about a
  **working tree**. Two different objects, and the sentence slid between them without
  anything looking wrong.
  **Verdict, tested by HEAD rather than by name:** `feat/view-state-persistence` is NOT an
  ancestor of `main`, 2 unique commits. So it is a report-do-not-delete case under the
  directive's own test, and it was left in place — deliberately not substituting this
  lane's "it is safe now" for the line the owner drew.
  **What was done instead:** the loose state was committed onto the branch and the branch
  **pushed for the first time in its life** (`f6ae8c4`, verified reachable at origin by
  `ls-remote`). It had never been pushed, so an erroneously-dispatched parcel that F15's
  reversal contingency depends on existed **on one filesystem and nowhere citable** — the
  anchor-has-a-location failure sitting unnoticed in this repo's own preservation story.
  Now the disposable claim is actually true rather than merely asserted.
  **Flagged for whoever opens F15:** the uncommitted `App.tsx` change replaced the
  `patchViewState` call with `void region;` — it **disables the write the parcel exists to
  add**. Most likely a red-first poison left in place when the parcel was stopped, but it
  was never reviewed and this lane did not adjudicate it. Treat that file as suspect first.
  **THE SIZE ANSWER IS NOT THE WORKTREE COUNT, and it generalises past this lane.** Of the
  4.1 GB, **3.9 GB is `src-tauri/target` and 152 MB is `node_modules`; the working tree is
  6.6 MB.** 96% is regenerable build cache, not preserved work. A sweep that counts
  worktrees will keep reporting gigabytes that a target-dir clean returns without touching
  a branch, and aurora's 856 MB is probably the same shape. Raised with the hub as its own
  question for the owner rather than folded into the merge sweep.
  **Reported to him, not decided here:** `.worktrees/phase4-sequencer`, 108 MB, dated May,
  whose gitdir names `/home/volence/sonic_hacks/megadaw/.git` — a repo no longer on this
  machine, and invisible to `git worktree list` for that reason.
  **Independent instance, not a save:** the hub's warning to check for running processes
  and to look at uncommitted files arrived **after** this lane had done both. Aurora
  reached it by removing a fixture directory out from under his running Aurora window
  (his next build died `spawn ./build.sh ENOENT`); this lane reached it from a directory it
  was about to remove. Different directions, so bar 19 corroboration rather than echo.
  `/proc` walk confirmed no process holds either directory here.

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

- 2026-08-30: **SIGIL RELINK BROADCAST — COSTS THIS LANE NOTHING, CHECKED RATHER THAN
  ASSUMED.** The sigil lane broadcast that the shared `sigil/target/release/sigil` was
  relinked at 2026-08-30T00:33:36Z to sigil master `85a5726c` (md5 `504b0c0a…` →
  `3fb008c8…`), having been 19 crate-commits stale, with the caveat that anything
  regenerated from a newer assembler should be pinned to its export revision rather than
  assumed reproducible. **Verified firsthand here that seraph consumes no sigil-built
  artifact:** `grep -rani "sigil"` over `src/`, `src-tauri/` and `tools/` (`*.rs`, `*.ts`,
  `*.tsx`, `*.toml`, `*.json`, `*.py`, `*.sh`) returns zero hits, and `git ls-files` shows
  no tracked `.bin`/`.blob`/`.sym`/`.vgm` or manifest. The only aeon coupling in the tree is
  **two source comments** citing driver behaviour at aeon `1ee8f8e6`
  (`sound_fm.emp:1092-1099`), in `src-tauri/src/audio/overlap_audibility.rs` and
  `src-tauri/src/project/manager.rs` — behaviour pinned to a revision, not a figure
  regenerated from an assembler, so a newer sigil does not reach them. Note per the
  protocol's stale-ruling-in-a-comment bar: those two are already anchored at a SHA, which
  is the right shape, but they are the kind of claim to re-ground rather than trust when
  the ch6 work opens.
  **This becomes real only at S3** (driver-in-the-loop), which consumes the blob and symbol
  artifacts sigil's `emit_sound_blob` emits; S3 sits behind S0 and S1, so nothing is owed
  today and nothing should be scheduled. Same shape as the emulator forward-notice already
  in `OVERSEER.md`: recorded so the session that opens S3 re-checks it firsthand instead of
  concluding there is no coupling.
  **METHOD NOTE, and this lane's own near-miss.** The first run of that enumeration used
  unquoted `--include=*.rs` under zsh, so the shell failed the glob and **grep never ran** —
  printing `no matches found` and an exit status that read as a clean empty result. The
  conclusion happened to be right and the command was broken, which is bar 16's shape
  arriving in the instrument rather than the subject: an empty result that is really an
  unexecuted command is indistinguishable from an empty result that is an answer. Quote the
  globs, and treat "no hits" from a glob-bearing grep as unverified until the command is
  known to have run. Passed to the sigil lane in the reply rather than kept here.

- 2026-08-30 (cont.): **F28 LANDED, AND THE LANE LOG IS OPEN.** `src/components/PianoRoll.tsx`
  held the tree's only source NUL (offset 32584, in `MIXED_VOICE`); it is gone. Fix spells the
  sentinel as the escape `"\\0mixed"` rather than a raw byte, so the **runtime string is
  unchanged** and the sentinel still cannot collide with a real voice id: `"\\0mixed" ===
  String.fromCharCode(0)+"mixed"` is `true`, length 6, codepoint 0 (checked in node, not
  reasoned about). `grep -c MIXED_VOICE` now returns 7 and exits 0, the exact inverse of the
  booked symptom. Merged `3207c0b` (fix `6582a94`), pushed, `origin/main` verified moved and
  equal to HEAD. Merged-tree lanes, exit codes read directly and never through a pipe: cargo
  **264 passed / 0 failed** (exit 0), `npm run build` exit 0 with zero warning or error lines,
  vitest **352/352 across 33 files** (exit 0), no `src/bindings.ts` drift.
  **THE POINT OF F28 WAS RE-RUNNING THE SWEEPS IT INVALIDATED, AND BOTH CAME BACK CLEAN —
  recorded because a null result is the one people skip writing down.** (1) F15's evidence
  (`grep` for `localStorage`, the sweep whose `Where` said "grep (no localStorage)"): 11 hits
  with `-a`, **none in `PianoRoll.tsx`**, so F15's zero-view-state-persistence conclusion
  survives and is now verified rather than accidentally-clean. (2) The bar-13 gesture
  reachability sweep, whose precedent is a live drag outliving the document it began on:
  `PianoRoll.tsx` is the 907-line drag surface that sweep would have skipped, and it holds
  both a window-level `keydown` (594) and a `SONG_REVERTED_EVENT` listener (275). Read past
  the cited lines per bar 11: `openRegionIdRef` (219-220, guarding 228 and 267) is a
  **correct guard** on async fetch continuations across a region switch, with the
  out-of-order-reply reasoning stated in its own comment. **No new finding; bar 13's lesson
  is already applied in this file.**
  **CONFIRMED PRESENT AND DELIBERATELY NOT FIXED HERE:** the stale `// Draw Mode (F6):`
  comment at line 118 where the binding is `B` (line 816's tooltip already says `(B)`). It is
  booked under README-7 and folding a second finding into F28's commit would have widened the
  landing quietly. Note it is an instance of the protocol's own worst-place-for-a-perishable-
  claim bar: a stale binding living in a comment, in the one file no grep could see.
  **PROVENANCE OF THE GO, stated because this lane's standing rule is that a relayed grant is
  not a witnessed one.** The owner did not give this lane the go directly; the hub
  (Dominion-launched empyrean session) pushed F28 -> F31 -> F27 under a standing delegation.
  **Anchor verified firsthand here**, not accepted on the hub's word: empyrean `b445116` is
  reachable from and is the tip of a freshly-fetched empyrean `origin/main`, and the RESUME
  BRIEF at `docs/OVERSEER.md:29` carries the owner **verbatim** rather than paraphrased,
  including *"I wanna clear + reboot everything then just have you again continuously push
  things through"*, *"if anything's confused you can make decisions/fable can"* and
  *"(you're the director/overseer)"*. That is a committed verbatim delegation read at a
  committed revision, which is a materially stronger artifact than the unanchored secondhand
  *"I guess"* this lane refused on 2026-08-22 — the distinction is the anchor and the
  verbatim, not the fact of relay. Recorded so the difference is checkable rather than a
  judgement call. Scope held: the pushed item was **this lane's own proposed order,
  unchanged**, the fix is one line and reversible, and nothing irreversible or
  design-changing was taken on relayed authority. **d-9 stays with the owner** (the hub
  agrees and explicitly declined to rule it); its card already carries the changed
  recommendation (`two-drivers`) and its full `because`, verified this session, so no
  rule-8c append was needed.

- 2026-08-30 (cont.): **F31 LANDED — the profile now advertises FIVE FM music voices, and
  the number was DERIVED FROM THE DRIVER, not taken from F31's own booking.** Merged
  `2034560`, pushed, `origin/main` verified moved and equal to HEAD. Merged-tree lanes,
  exit codes read directly: cargo **265 passed / 0 failed** (was 264; +1 is the new guard,
  monotonic), `npm run build` exit 0 with zero warning or error lines, vitest **352/352
  across 33 files**, no `src/bindings.ts` drift.
  **THE DERIVATION, because bar 1 is the whole reason this is trustworthy.** F31's booking
  asserted S3K "has no FM6 music voice at all"; that claim was re-derived from
  `skdisasm/Sound/Z80 Sound Driver.asm` rather than transcribed. Three independent
  witnesses in that file: line **1907** `db 80h, 6 ; FM6 music track (does not exist in
  this driver)`, stated by the driver itself; lines **177-182**, where the track RAM lays
  `zSongFM6_DAC` out **ahead of** `zSongFM1..FM5` as ONE shared slot; and line **2252**,
  where the driver computes its own *"Number of FM tracks"* as
  `(zSongPSG1-zSongFM1)/zTrack.len`, a span that excludes the FM6/DAC slot and therefore
  equals **5**. The booking was right; it is now checkable.
  **AUTHORITY: this rests on a SETTLED OWNER RULING, not on the hub relay that pushed it.**
  d-7 (`driver-decides`, answered by the owner through the console 2026-08-24) rules that
  the driver profile carries the channel-6 behaviour and Seraph follows it, and its own
  `detail` names **F31 as the first work it implies**. So F31 would have been correct to do
  on his word alone; the relay only affected ordering. Recorded because it is the
  distinction that matters if the relay is ever questioned.
  **MIGRATION EXPOSURE CHECKED, NOT ASSUMED — this was the part that could have made a
  correct fix destructive.** Removing a channel from the layout is a change to what the app
  OFFERS, and the question was whether it silently breaks a song that already uses FM6.
  Enumerated the consumers (`manager.rs` 364 roster build, 602 name lookup, 2196 lane
  count; `TrackList.tsx`, `AddTrackDialog.tsx`): `default_lane_name` already returns
  `Option<String>` and its **own doc comment** already documents `None` for a channel absent
  from the layout, and `ChannelAssignment::Fm(n)` is **never validated against the layout
  anywhere**. So an existing `Fm(5)` track is neither deleted nor crashed; it simply loses
  its default name. **Nothing is destroyed, and new projects get the honest roster.**
  **THE TEST SUITE HAD ENCODED THE DEFECT EXACTLY ONCE.** Only `test_channel_layout` broke
  (`assert_eq!(fm_channels.len(), 6)`). The roster tests at `manager.rs` 1663/1684/2196 did
  **not** break, because they derive their expectations from the layout instead of
  hardcoding a count — bar 1 already practised there, and worth noting as the reason this
  fix was cheap.
  **NEW GUARD, POISONED RED-FIRST.** Added `fm_voices_plus_dac_never_exceed_the_chips_six_slots`:
  FM voices + DAC channels <= 6, the invariant F31 violated. Poison = reintroduce the exact
  defect; it fires with **`driver `flamedriver` advertises 6 FM voices plus 1 DAC channels
  = 7, but the chip has only 6 FM slots and the DAC occupies one of them`** — the mismatch
  named, per bar 2, not "something raised". `test_channel_layout` also grew an explicit
  "index 5 must not be a music voice" assertion and fired on the same poison.
  **GUARD SCOPE STATED RATHER THAN OVERCLAIMED, and this is a booked follow-up.** The first
  draft of that test iterated an invented `crate::driver::all_profiles()`, which does not
  exist — caught by reading the real API rather than by the compiler alone. `DriverRegistry`
  offers only `get`/`list`, and `lib.rs:180-181` registers inline with no shared
  constructor, so the guard covers the profiles **it** registers and does not inherit a
  driver added later. Its doc comment says so in those words. **BOOKED: extract a shared
  registry constructor so the guard covers every registered driver** — a `lib.rs` change,
  deliberately out of F31's scope, and it becomes live the moment the absent Memra profile
  is added.
  **NOT TOUCHED, still open:** an existing FM6 track still *exports*, which is F30's
  double-header defect and F27's playback question. F31 was upstream of both and is wrong
  however F27 resolves, which is why it went first.

- 2026-08-30 (cont.): **F29 PART ONE LANDED — the drumless VGM now says it is drumless.
  PART TWO IS PARKED WITH THE OWNER AS d-10, AND THE BOOKED SIZE WAS WRONG.** Merged
  `fd6d21b`, pushed, `origin/main` verified moved and equal to HEAD. Merged-tree lanes, exit
  codes read directly: cargo **268 passed / 0 failed** (was 265; +3 new tests), `npm run
  build` exit 0 with zero warning or error lines, vitest **352/352 across 33 files**, no
  `src/bindings.ts` drift.
  **WHAT LANDED:** `export_vgm_data` returns `VgmExport { data, skipped_dac_tracks }` and
  `export_vgm` appends a warning naming the tracks left out. No bindings change was needed,
  because `export_vgm` already returned a free-text success string. The drop site now carries
  a comment saying it is still a drop and why.
  **WHAT DID NOT LAND, AND WHY THE BOOKED SIZE (M) IS WRONG — this is the finding.**
  Representing DAC in VGM is **not a register write**. It is a `0x67` PCM data block plus a
  stream of `0x8n` write-and-wait commands that must interleave **sample-accurately** with
  every FM and PSG event, which means converting `export_vgm_data`'s **tick-based** event
  loop into a **sample-accurate** one. That is a restructure, not an added match arm.
  **Re-sized M -> L.** Parked as **d-10** with three options and a recommendation.
  **THE FACT THAT DECIDES d-10, VERIFIED FIRSTHAND RATHER THAN TAKEN FROM README-7's
  BOOKING:** the VGM export **UI path is dead**. `exportVgm` exists in `src/bindings.ts`
  (551) and `src/api/ipc.ts` (411) and **no component calls it** — so no VGM can leave the
  app by any route the owner has today, and the drumless gap costs him nothing right now.
  Hence the recommendation: do the DAC work **as part of wiring the button** (README-7), not
  before, so the first reachable VGM is complete on the first try. **README-7's VGM row is
  therefore gated behind d-10, not merely behind F29.**
  **INTERACTION WORTH RECORDING: F31 LANDING FIRST SHRANK THIS PROBLEM.** A faithful DAC
  export must write `$2B`, whose semantics are F27's parked design call. But F31 corrected
  Flamedriver to offer **no FM6 music voice**, so for any song authored after it there is no
  FM6-versus-DAC conflict to rule on. Only a **legacy song carrying an `Fm(5)` track** still
  reaches F27's question. Sequencing F31 ahead of F29 was not merely tidy; it removed a
  design dependency.
  **NEW FINDING BOOKED — F33, SAME SILENT-DROP CLASS, IN SMPS EXPORT.** `export/smps.rs:794`
  does `Path::new(&inst.pcm_file)` and guards on `pcm_src.exists()` **with no `else`**. But
  `pcm_file` is a **bare filename** (`commands.rs:546` writes `format!("{id}.pcm")`) which
  every other consumer resolves as `<project>/instruments/dac/<pcm_file>`
  (`manager.rs:427`, `import/mod.rs:200`, `commands.rs:548`). So the path is resolved against
  the process CWD, essentially never exists, and **SMPS export silently copies no DAC sample
  at all**. Found while sizing F29; not fixed here, because it is a different export path and
  folding it in would have widened F29's landing. Severity looks high: it is the export that
  actually reaches the game.
  **NEW ROW BOOKED — F34, from F31's landing:** extract a shared driver-registry constructor
  so the `fm_voices_plus_dac_never_exceed_the_chips_six_slots` guard covers every registered
  driver instead of only the ones its own test registers. `lib.rs:180-181` registers inline.
  Becomes live when the absent Memra profile is added.
  **METHOD FAILURE, THIRD ZSH QUOTING BITE OF THIS SESSION, AND THIS ONE REACHED A PUSHED
  COMMIT.** F29's commit message contained a backtick-quoted word inside a **double-quoted**
  `-m`, so zsh ran it as command substitution: the shell printed `continue:1: not in while,
  until, select, or repeat loop` and the word was **deleted from the message**, which now
  reads "Reinstating the bare  fires". The commit is pushed and the owner's push grant
  forbids rewriting pushed history, so it is **corrected here rather than amended**. The
  other two bites this session were unquoted `--include=*.rs` globs making `grep` never run
  (twice), one of which produced a clean-looking empty result that was nearly read as an
  answer. **Standing lesson for this lane: in `zsh`, backticks and globs are live inside
  double quotes and inside an UNQUOTED heredoc.** Use `<<'EOF'`, quote every glob, and prefer
  writing prose through `python3` heredocs, which is why the `.jsonl` files were unaffected.
  **Integrity checked, not assumed:** the F28 and F31 Log entries and both `lane-log.jsonl`
  entries were re-read after this was found and are intact.

- 2026-08-30 (cont.): **F33 LANDED — and it was the most consequential defect of the day,
  because it silently destroyed output on the path that actually reaches the game.** Merged
  `c47a938`, pushed, `origin/main` verified moved and equal to HEAD. Merged-tree lanes, exit
  codes read directly: cargo **271 passed / 0 failed** (was 268; +3), `npm run build` exit 0
  with zero warning or error lines, vitest **352/352 across 33 files**, no `src/bindings.ts`
  drift.
  **THE DEFECT:** `DacInstrument::pcm_file` is a **bare filename** (`commands.rs` writes
  `format!("{id}.pcm")`), resolved by every other consumer as
  `<project>/instruments/dac/<pcm_file>`. `export/smps.rs` did `Path::new(&inst.pcm_file)` —
  the process **CWD** — and guarded the copy on a bare `.exists()` **with no `else`**. So an
  SMPS export of a song with drums copied **no sample at all** and reported success.
  **THE FIX:** thread the open project's directory through. `DriverProfile::export_song` and
  `smps::write_export` now take `project_dir: Option<&Path>`, read from
  `ProjectManager::project_path()` at the single call site (`commands.rs:1161`). **No IPC
  signature changed, so `bindings.ts` is untouched** — confirmed by the drift check, not
  assumed. Every silent skip in that block is now a reported failure.
  **THE UPSTREAM CHECK THAT KEPT THIS HONEST (bar 13 — reachability is an enumeration).** Two
  of the four skips are `instrument_id` absent and instrument-not-in-bank. Rather than assume
  they were dead, `validate_for_export` was read: it already errors with *"No instrument
  assigned"* and *"Assigned instrument not found"*, and `write_export` returns early on any
  validation error. So they are genuinely defensive — **kept, converted from silent skips to
  reported errors, and NOT deleted**, since a defensive branch that skips silently is how F33
  hid in the first place.
  **WHY A FULLY GREEN SUITE NEVER SAW IT: no test had ever exercised the DAC copy path.**
  That is the transferable lesson, not the path bug. Threading the new parameter through
  broke **zero** tests, which is itself the finding — a code path with no test is invisible to
  every future refactor as well as to this one. Three tests added, poisoned red-first:
  restoring `Path::new` fails `the_dac_sample_is_copied_into_the_export` with the sample
  *"expected it at `6696a359-...pcm`"* — **a bare filename with no directory, which is the
  CWD lookup made visible in the failure message itself**. Two are controls: a missing sample
  must be **reported, not skipped**, and an unsaved project must say *save first* rather than
  drop the drums.
  **CONSEQUENCE FOR THE OWNER, stated in the lane log rather than buried here:** any song he
  exported to the game before today that had drums was exported **without them**, and needs
  re-exporting.
  **METHOD NOTE — the zsh lesson from the previous entry was applied and worked.** This
  commit message was written with `git commit -F -` over a **quoted** heredoc, and its
  backticked and `Path::new`-bearing lines survived intact (verified by grepping the committed
  message, not by assuming). The F29 message damaged earlier in this session stays damaged,
  because it is pushed.

- 2026-08-30 (cont.): **F32 LANDED — TWO defects, not the one booked, and the second could
  not fail at all.** Merged `b8bc434`, pushed, `origin/main` verified moved and equal to
  HEAD. Merged-tree lanes, exit codes read directly: cargo **271 passed / 0 failed**, `npm
  run build` exit 0 with zero warning or error lines, vitest **352/352 across 33 files**, no
  `src/bindings.ts` drift. (Rust total unchanged at 271 — this parcel repaired two existing
  tests rather than adding any, which is the honest reading of a count that did not move.)
  **DEFECT 1, as booked:** both Zyrinx tests hardcoded `/home/volence/...` and `return`ed
  early when the ROM was absent, so on any other tree they **reported green having checked
  nothing**. Path now from `SERAPH_ZYRINX_ROM` (same default); a missing ROM **panics with
  instructions**; skipping requires `SERAPH_SKIP_ROM_TESTS` set deliberately.
  **DEFECT 2, NOT booked, found while fixing the first, and worse:**
  `test_import_all_zyrinx_songs` looped 19 imports, `eprintln!`d `ERROR` on failure, and
  **asserted nothing**. All 19 could have failed and it passed. **A print harness in a
  test's clothing** — the same "cannot fail, therefore cannot report" shape as the silent
  skip, one level in, and it was live even on the machine that HAS the ROM. It now collects
  failures and zero-note imports and gates on both.
  **GATE VALUES MEASURED, NOT TRANSCRIBED (bar 1).** Ran it first: all 19 slots import with
  **0 errors, 0 warnings**, note counts ranging **5,702..571,753**, 6 tracks each. The flat
  6 is the chip's six channels, not bar 5's suspicious constant; the note spread is the
  varying work. The assertions encode the invariants measurement established — every slot
  imports, an imported song has notes — rather than a brittle snapshot count.
  **POISONED THREE WAYS, because there were two defects and an opt-out:** (1)
  `SERAPH_ZYRINX_ROM=/nonexistent` → **both tests FAIL** (`2 failed`) with the explanation,
  where they used to pass; (2) opt-out set → skips without failing, notice recoverable under
  `--nocapture`; (3) widening the slot range → batch test **fails naming six bad slots**,
  where it previously printed them and passed.
  **NEW FINDING BOOKED — F35, and it is why the gate was NOT widened.** Poison 3's error text
  revealed the driver reports valid song IDs as **0-19**, but the loop runs **1..20**, so
  **slot 0 has never been tested**. It does import — as **"Silence", with 6,868 notes, byte
  for byte slot 1 "Main Title"'s count**. A song named Silence carrying another song's note
  count suggests **slot 0 resolves to slot 1's data**. Extending the gate over it would have
  **ratified unverified behaviour** — bar 9's corollary, an unvalidated instrument recruiting
  the suite into arguing for it — so the range stays `1..20`, the reasoning is a comment at
  the exact spot, and the question is booked as **F35** instead.
  **HONEST LIMIT OF THIS FIX, stated rather than glossed:** `SERAPH_SKIP_ROM_TESTS=1` still
  produces a passing test that checked nothing, and cargo captures its notice unless
  `--nocapture` is passed. That is the same false-green shape, gated behind a deliberate act.
  It is accepted rather than solved: `libtest` has no runtime "skipped" state, so the
  alternatives were an unconditional `#[ignore]` (which would end the real coverage this
  machine has, since the ROM is present here) or no opt-out at all (which breaks the suite
  for any tree without the ROM). **The residual risk is a stale env var in a shell profile
  silently disabling this coverage forever.**

- 2026-08-30 (cont.): **F35 LANDED, AND IT WAS NOT COSMETIC — it had misnamed 23 of the 212
  shipped Batman library instruments.** Merged `7753832`, pushed, `origin/main` verified
  moved and equal to HEAD. Merged-tree lanes, exit codes read directly: cargo **274 passed /
  0 failed** (was 271; +3 guards), `npm run build` exit 0 with zero warning or error lines,
  vitest **352/352 across 33 files**, no `src/bindings.ts` drift.
  **THE MECHANISM, established by reading the table rather than inferring from the symptom.**
  `SONG_INDEX[0]` and `SONG_INDEX[1]` are **both `(0, 0)`** — the table's **only** duplicate,
  and **not a gap**: 20 slots map to 19 distinct songs and every bank's songs are contiguous
  with none unmapped (counted, not eyeballed). Slot 0 is the game's silence/stop-music
  command, pointed at valid data so it parses cleanly. That is why F32's measurement saw
  "Silence" carrying **6,868 notes, byte for byte slot 1 "Main Title"'s count** — they are
  the same song.
  **THE CONSEQUENCE, AND IT IS THE PART A CODE READ WOULD HAVE MISSED.** `library::extract`
  iterated game ids **from 0**, so slot 0 was seen **first** and **won the naming**:
  `inst.name = format!("{song} voice {:02}")` fires only on first insertion, later slots
  merely append to `provenance.songs`. So **23 of 212 tracked library entries shipped as
  `"Silence voice NN"`**, filenames included (`library/batman-robin/fm/silence-voice-90.json`),
  each with provenance `["Silence", "Main Title", ...]`. **Verified by RUNNING the
  extraction** — 23 files named Silence, all 23 also credited to Main Title — not by reading
  the loop.
  **THE DATA HALF WAS PROVEN SAFE BEFORE IT WAS APPLIED, which is the part worth copying.**
  The library is **committed**, so a re-extraction rewrites tracked files. Two checks first:
  (1) **is there curation to clobber?** Measured **0 tagged and 0 hand-renamed** entries
  across 212, so the library is entirely machine-generated. (2) **is this really only a
  rename?** Extracted to a temp dir and diffed against the tracked tree: **hash sets
  identical (212 = 212)**, 23 files out and 23 in, and **the only instrument field that
  differs anywhere is `name`** — **zero sound parameters changed** across all 212. Only then
  applied. Re-running extraction on the applied tree changes nothing further (idempotent, as
  `OVERSEER.md` says).
  **A MEASUREMENT MISTAKE MADE AND CAUGHT HERE, worth recording because the wrong number was
  alarming.** The first curation scan reported **149 of 213 "hand-renamed"**, which would
  have made the data fix unsafe. That was **my regex, not the data**: `voice \d{2}` requires
  exactly two digits, so every three-digit name (`voice 119`) counted as a rename, and
  `_game.json` was not excluded. Corrected to `voice \d+`: **0 renamed, 0 tagged.** A
  measurement that would have blocked the right action, produced by the instrument rather
  than the subject — bar 9's shape, arriving in a throwaway script.
  **GUARDS NEED NO ROM, unlike F32's, because the refusal precedes any ROM access** — so
  these three run on every tree. Poisoned red-first: deleting the guard makes slot 0 fall
  through to **`"ROM too short for bank 1 header"`, the same error a real song gives on an
  empty ROM**, which is precisely why the guard is needed. One is a **control** proving the
  refusal is specific to slot 0 rather than blanket; one asserts the **index table's shape**
  (`SONG_INDEX[0] == SONG_INDEX[1]`, exactly one duplicate) so a future remap **fails loudly
  instead of silently outliving this reasoning** — the perishable-precedent rule applied to
  this lane's own new comment.
  **Extraction stats now read `songs=19`, not 20.** One of those twenty was a phantom.

- 2026-08-30 (cont.): **F34 LANDED — the F31 guard now covers the REGISTRY, not a hand-typed
  list.** Merged `72391ef`, pushed, `origin/main` verified moved and equal to HEAD.
  Merged-tree lanes: cargo **274 passed / 0 failed**, `npm run build` exit 0 with zero
  warning or error lines, vitest **352/352 across 33 files**, no `src/bindings.ts` drift.
  **THE PROBLEM WAS AN ENUMERATION ONE (bar 8).** `DriverRegistry` was constructed inline at
  **three** sites — `lib.rs:180`, `audio/overlap_audibility.rs:61`, `project/manager.rs:1628`
  — each re-typing the same registration. So an invariant asserted over "the registered
  drivers" could only ever cover the list the asserting code itself repeated. The absent
  Memra profile would have been covered by **none** of them **while the guard still looked
  like it covered everything**, which is the failure mode F31's doc comment named and
  deliberately left standing.
  **THE FIX:** `driver::default_registry()` as the single registration site, used at all
  three, plus `DriverRegistry::profiles()` so the guard iterates what the app **actually**
  registers. The guard's doc comment no longer has to disclaim its own scope — the earlier
  disclaimer is deleted rather than left to rot, which is the perishable-comment rule applied
  to a comment this lane wrote four hours ago.
  **POISONED TWO WAYS, because "covers every driver" and "can still catch one" are DIFFERENT
  CLAIMS and only the second is what F31 had proven.** (A) Empty the registry → the guard
  fails on its explicit emptiness control (*"passes by having nothing to check"*) rather than
  passing vacuously over zero drivers. (B) Re-add FM6 to Flamedriver → the invariant still
  fires, now reached **through the registry** rather than a hand-written vec. Neither poison
  alone would have established the pair.
  **Housekeeping, stated so the warning count is readable:** removed four imports this change
  made unused. The remaining `unused variable: track_idx` in `import/smps_mapper.rs` is
  **pre-existing** — confirmed present in the pre-F29 build log — and untouched, since it is
  nobody's parcel today.

- 2026-08-30 (cont.): **README BATCH — 4 OF 6 FIXED, 2 RECLASSIFIED WITH EVIDENCE RATHER
  THAN "FIXED".** Merged `e899992`, pushed. Lanes: cargo **276 passed / 0 failed** (was 274;
  +2), `npm run build` exit 0 zero warnings, vitest **352/352**.
  **ITEM 3 IS THE ONE THAT MATTERED AND IT WAS BIGGER THAN "a UI gap".** `TopBar.tsx` called
  `ipc.exportWav(path, 60)`, so **every WAV export was exactly 60 seconds** — silently
  truncating any longer song and padding any shorter one with silence. Added
  `ProjectManager::song_duration_seconds()`, measured from **both** region extents **and**
  note extents (a note may run past its region's declared duration — checked, not assumed),
  plus a 2s release tail so FM releases are not clipped. `export_wav`'s duration is now
  `Option<f64>`, `None` = whole song; UI passes `None`; a fixed-length excerpt is still
  askable. **IPC shape moved, so `src/bindings.ts` was regenerated by the existing export
  test and committed with the change** — the one parcel today where bindings drift is
  expected rather than a red flag.
  Expectation **derived**: 120bpm/480tpb → 1920 ticks = 4 beats = 2.0s, +2.0s tail = **4.0s**.
  Second test is a **control** proving the duration TRACKS the song rather than being some
  other constant (a region twice as far out must add exactly 2.0s).
  **ITEM 1:** `let out = get("out")` was hoisted above the subcommand match, so `--help` and
  every unknown subcommand died on `missing --out` and `usage()` was unreachable — **the one
  path a confused user takes was the one path that could not print help.** Resolved per-arm.
  Verified both directions: `--help` now prints usage, and `psg-table` with no `--out` still
  correctly reports it (control, so the fix did not simply stop requiring the flag).
  **ITEM 5:** `export_formats()` advertised `"binary"` with nothing implementing it.
  Withdrawn rather than stubbed. **Wider than booked:** the method has **no callers at all**,
  which is exactly why a false claim could sit there without anything ever failing.
  **ITEM 7:** the `// Draw Mode (F6)` comment reworded — `F6` is this repo's audit item
  number, not a key, and it read as a keybinding.
  **ITEM 6 IS NOT A DEFECT, AND "FIXING" IT WOULD HAVE INTRODUCED A FALSEHOOD.** The booking
  read *"`Fm3SpecialMode` declared supported, implemented nowhere"*. Derived from the driver
  instead of accepting that: S3K's driver **does** implement FM3 special mode —
  `Z80 Sound Driver.asm:818` branches on it (`bit 0, (ix+zTrack.PlaybackControl)`), with
  `.special_mode:`, `zGetSpecialFM3DataPointer` and a `zSpecialFreqCommands` table. So
  `supports_feature` is **truthful about the driver**; the gap is that **Seraph offers no way
  to author it**. Feature gap, not false claim. The booking conflated driver-support with
  Seraph-support. **Rebooked as a feature request, code unchanged.**
  **ITEM 8 IS A DESIGN CALL, NOT A DEFECT FIX.** `get_channel_overlaps` is correct, tested
  backend code with no UI caller, and `manager.rs:1501` says overlaps are *"surfaced post-hoc"*
  by it — so deleting it would contradict the design, and **choosing where warnings appear is
  a product decision**. Not invented here. Left booked as a UI gap adjacent to F27.

- 2026-08-30 (cont.): **THE FLAKE IS DEAD, AND IT WAS THE INSTRUMENT, NOT THE SUBJECT.**
  Merged `b826df5`, pushed. **Name finally captured:** `ArrangementView.test.tsx` →
  *"Ctrl+C then Ctrl+V pastes at the bar-snapped seek cursor"*. It surfaced during this
  session's own merged-tree run and **the full log was kept rather than piped through
  `tail`**, which is the only reason the name survived — the exact lesson the queue banked
  when the name was lost the first time.
  **NOT A RACE IN THE COMPONENT.** The case used three `waitFor`s; under a loaded full-suite
  run the first spent its entire **1000ms default budget** and reported `Number of calls: 0`,
  which **reads as the paste never happening rather than as the assertion giving up**. The
  failing run's own duration, **1030ms**, is the tell.
  **DIAGNOSIS PROVEN, AFTER A FIRST ATTEMPT THAT WAS INVALID AND NEARLY COUNTED.** The first
  mechanism test edited the `waitFor` into a **double comma**, so vitest reported
  `Tests no tests` and exit 1 — **a syntax error that would have been read as the timeout
  failing**, i.e. this session's own "empty result that looks like an answer" pattern for the
  third time. Redone correctly: squeezing the OLD form's budget to **1ms reproduces the exact
  production symptom** (same `AssertionError`, same `Number of calls: 0`), while **the same
  old form at its normal budget under identical conditions passes** (control). The file also
  runs **12/12 clean in isolation** — which is why it was never reproducible alone: **alone
  there is no load to blow the budget.**
  **FIX WAS ALREADY WRITTEN DOWN THREE TESTS FURTHER DOWN THE SAME FILE.** The sibling cases
  carry the `await act(async () => {})` drain *and its reasoning* (*"no polling, no timeout
  budget that a loaded CI box can blow through"*). A previous session solved this and left
  this case on `waitFor`. Bar 12's shape inside one file: the rule was written where its
  other readers would not look.
  **HONEST STRENGTH OF EVIDENCE:** full suite **8/8 clean** after — but at a 1-in-6 rate,
  eight clean runs happen by luck **~23%** of the time, so **the mechanism is the
  load-bearing evidence, not the streak.** Stated rather than letting the streak carry it.
  **PROCESS FAILURE TO FIX, MINE:** the README batch's landing chained
  `npm test; git push` so **the push did not depend on the test result** — main was pushed
  while that run was red. It was the pre-existing flake and not caused by that parcel
  (nothing committed today touches `ArrangementView`), and main is green now, but the chain
  is the defect: **a landing command must not be able to push over a red lane.**

- 2026-08-30 (cont.): **SOUND-TRUTH: the copied PSG table is now CHECKED against the driver
  it names, and out-of-range notes are REPORTED instead of silently retuned.** Merged
  `bebea01`, pushed. Lanes: cargo **281 passed / 0 failed** (was 276; +5), `npm run build`
  exit 0 zero warnings, vitest **352/352**, no `src/bindings.ts` drift. Both halves are true
  **for any driver list**, so neither waits on d-9.
  **DEVIATION FROM THE BRIEF, FLAGGED RATHER THAN DONE QUIETLY (bar 7).** The hub's brief
  said read the tables from **Memra's** source. This grounds against **skdisasm** instead.
  Reason: `PSG_PERIOD_TABLE`'s own comment says it *"matches Flamedriver/S3K Z80 driver
  exactly"*, Seraph's only registered profile is Flamedriver, and **there is no Memra profile
  in this tree** (F31 booked its absence). Checking an S3K table against a *different*
  driver's numbers would **manufacture drift rather than detect it** — bar 9's shape, where
  the instrument is pointed at the wrong subject. Memra source **does** exist
  (`aeon/engine/sound/`, `aeon/tools/gen_sound_tables.py`) and is the right target the day a
  Memra profile lands; that is F31's row, not this one.
  **THE DRIFT CHECK IS NOT COPY-VERSUS-COPY, WHICH WOULD PROVE NOTHING.** The driver does not
  store periods at all: it stores **frequencies in Hz** and computes periods at assembly time
  with `zMakePSGFrequency = min(3FFh, round(PSG_Sample_Rate/(frequency*2)))`, where
  `PSG_Sample_Rate = Z80_Clock/16` and `Z80_Clock = Master_Clock/15` (= 3,579,545 → 223,721).
  So the test **parses the Hz list and the clock constants out of the disassembly and applies
  the driver's own formula**. Result today: **84/84 exact** — no live drift, so the value is
  entirely in catching future drift, which is what was asked for.
  **Poisoned three ways:** corrupting one entry names it (*"index 11: driver 0x388 vs table
  0x389"*); an unreachable source **FAILS with instructions** rather than passing (F32's rule
  applied to a new external-input test the same day it was written); the deliberate opt-out
  skips. **Reuses `SERAPH_SKIP_ROM_TESTS` so there is ONE knob, not two** — a second switch
  would be a second thing to leave set by accident.
  **THE RANGE DEFECT WAS WRONG IN BOTH DIRECTIONS, from one channel-agnostic check.**
  `validate_for_export` used `midi_to_smps_note` (MIDI **12-106**, FM's range) for every
  pitched channel. PSG's real range is **36-119**, derived from the driver's own table (z80
  index 0-83, `midi = index + 36`), not chosen here. So: **MIDI 12-35 PASSED validation and
  was then silently retuned** — `smps_note_name_psg` clamps with `.max(0)` and
  `midi_to_psg_period` returns the bottom entry — and **MIDI 107-119 was REJECTED** though
  the table and the exporter both handle it.
  **THE BRIEF'S FRAMING IS CORRECTED ON THE EVIDENCE:** it said such a note "will be silent
  in the game". It is **not silenced, it is RETUNED**, and that is *worse* — silence is
  noticeable, a wrong note in the right rhythm is not, and the app and the export **agree
  with each other** while neither matches what the author wrote, so nothing looks
  inconsistent from any single vantage point. The error message says retuned, not dropped.
  **Four tests, two of them controls** (an above-range note must still be reported; an
  ordinary in-range note must raise **nothing**, or the others would pass with a check that
  fires unconditionally). Poisoned by restoring the channel-agnostic check: MIDI 20 goes back
  to reporting **nothing at all** (`got []`), MIDI 115 back to being refused.
  **Housekeeping:** the one remaining cargo warning (`sum_r`, `audio/engine.rs:1330`) is
  **pre-existing** — present in this session's pre-F29 log, and `engine.rs` is untouched by
  every commit today (checked with `git log --name-only`, not assumed).
- 2026-08-30 (cont.): **F30 LANDED — the export that reaches the game now REFUSES a song that
  uses a voice the driver does not have, and the booked defect was mis-framed in a way that
  mattered.** Merged `02fe728`, pushed, `origin/main` verified moved and equal to HEAD.
  Merged-tree lanes, exit codes read directly: cargo **287 passed / 0 failed** (was 281; +6),
  `npm run build` exit 0 with zero warning or error lines, vitest **352/352 across 33 files**,
  no `src/bindings.ts` drift. Three cargo warnings, all pre-existing and enumerated (`sum_r`
  x2 in `audio/engine.rs`, `track_idx` in `import/smps_mapper.rs`); no new ones.
  **THE BOOKING SAID "emits both an FM and a DAC header for index 5". THE COUNT BYTE AND THE
  ENTRY COUNT ACTUALLY AGREE** — the agent reproduced first and the evidence corrected the
  brief, which had inherited the booking's framing and my own. What is malformed is that an
  entry exists for a voice that does not, and **the driver's damage is POSITIONAL, not
  arithmetic**: `zBGMLoad` fills the FM/DAC slots in order from `zTracksStart`, and there are
  exactly six of them (`zSongFM6_DAC, zSongFM1..zSongFM5`, then `zSongPSG1`), with slot 0
  driven unconditionally as the DAC. So an "FM6" entry lands on whichever slot its position
  reaches — FM1, in the DAC-plus-FM6 case — and a song wanting six FM voices plus DAC needs a
  seventh entry where the driver has six, running off the end into `zSongPSG1`.
  **VERIFIED FIRSTHAND HERE, not taken from the agent's report:** `Z80 Sound Driver.asm`
  176-183 (the six slots and `zSongPSG1` immediately after), 717-719 (slot 0 driven as DAC
  unconditionally), 1836-1839 (`ld b, (iy+2) ; b = number of FM + DAC channels`), 1859-1862
  (PSG count at `(iy+3)`), 1905-1908 (`db 80h, 6 ; FM6 music track (does not exist in this
  driver)`, inside `if fix_sndbugs=0`).
  **SECOND FINDING, CHECKED AND CORRECT, SO DOCUMENTED RATHER THAN CHANGED.** `smpsHeaderChan`'s
  first byte does count DAC together with FM, so `fm_count + dac_count` was right all along.
  The derivation is now a comment at the site with its sources. **Corroborated here by a
  DIFFERENT ENUMERATION PARAMETER (bar 19):** the agent counted files whose declared header
  matched their entry counts; this lane counted every `smpsHeaderChan` declaration across all
  60 files in `skdisasm/Sound/Music/` and got **59 x `$06, $03`, 1 x `$07, $03`**. Same answer,
  different parameter, so this is corroboration rather than echo.
  **THE OUTLIER IS EVIDENCE, NOT NOISE, AND IT IS SHIPPED SEGA CODE.** `Chaos Emerald.asm`
  declares `$07` = 1 DAC + 6 FM — exactly the seventh-entry case above, in a real S3K song.
  It pairs with the driver's `db 80h, 6` sitting behind `if fix_sndbugs=0`: the unfixed driver
  hands out seven init bytes over six slots. So the overflow the fix now refuses is not
  hypothetical; the original game has an instance of it.
  **WHAT LANDED.** `ChannelLayout::channel_name()` and `channel_names()` in `model/driver.rs`
  as the single authority on whether a driver has a channel; `validate_for_export` takes a
  `&dyn DriverProfile` and refuses any track whose channel the layout does not name, reporting
  the track, the channel, the driver and the channels it does have. **Derived from the profile,
  never from the literal 5** — the guard test iterates `driver::default_registry().profiles()`
  (F34's single registration site) and refuses one index past each profile's own list, so it
  never names 5 and covers a driver added later.
  **F27 WAS NOT TOUCHED AND THAT WAS THE POINT.** No steal, merge, priority or drop rule was
  invented. The export refuses and says why; what such a song should SOUND like stays the
  owner's parked call.
  **THE CONTROL THAT PROVES THE FIX IS NOT DESTRUCTIVE, and it is the one worth copying.**
  Every pre-F31 project carries a leftover EMPTY FM6 lane, because lanes were seeded from the
  layout that then offered six FM voices. The check is gated on `has_notes`, matching
  `generate_music_asm`'s own inclusion rule (`!t.muted && ...any(|r| !r.notes.is_empty())`), and
  `an_empty_leftover_fm6_lane_does_not_block_the_export` asserts it. **Without that gate the fix
  would have blocked export of every project saved before this morning.** Verified here that the
  muted half matches too: `validate_for_export` skips muted tracks, as `generate_music_asm` does.
  Six tests, all poisoned red-first, two of them controls.
  **BOOKED, FOUND AND DELIBERATELY NOT FIXED — F36 and F37, both behavioural design calls
  adjacent to F27 rather than defects with an obvious right answer:**
  **F36** — FM header entries are POSITIONAL in the driver but emitted in `song.tracks` order,
  while `zBGMLoad` hands out fixed channel bytes in slot order (`zFMDACInitBytes`), so a song
  using only FM2 and FM5, or ordering FM3 before FM1, exports labels that do not match the
  hardware channel that will play them. Matters for FM3 special mode (README-8's feature row).
  **F37** — a song with FM tracks and NO DAC track puts its first FM track on the DAC, because
  slot 0 is `zSongFM6_DAC` and is driven unconditionally through `zUpdateDACTrack`; the driver
  never asks whether entry 0 is a DAC entry. All 60 shipped S3K songs carry exactly one
  `smpsHeaderDAC` first, so the format effectively requires it. Seraph happily exports a
  DAC-less song today. **This is plausibly larger than F30 itself.**
  **F38 (small)** — `ChannelLayout::channel_name` now states the same `Psg`/`PsgNoise` binding
  convention as `ProjectManager::default_lane_name`; delegating that method to the new helper
  removes the second copy. Exactly F34's class, left alone because it is another file's parcel.
  **ENVIRONMENT FINDING FOR FUTURE WORKTREE PARCELS.** A bare `cargo test` in a worktree under
  `seraph/.claude/worktrees/<agent>/` fails `psg_table_still_matches_the_driver_it_claims_to_match`
  with *"S3K disassembly not found at ../../skdisasm"* — from a worktree, `../../` is
  `seraph/.claude/worktrees/`, not `sonic_hacks/`. Path arithmetic, not a regression: the agent
  used the test's own `SERAPH_SKDISASM_DIR` escape hatch, and the merged-tree run here needed no
  env var at all, which is what confirms it is worktree-only. **Booked as F39:** resolve that
  path via `git rev-parse --show-toplevel` so the test is worktree-portable, since the next
  agent will hit it too.
  **TAGGED, NOT ATTEMPTED (no emulator from a background agent):** what an out-of-order or
  DAC-less header actually sounds like on hardware. Only F36 and F37 would want it, and only
  before ruling on them.
- 2026-08-30 (cont.): **F39 + F38 LANDED, AND THE AGENT WAS RIGHT AGAINST THE BRIEF ON THE
  MECHANISM — the controller's prescribed fix does not work.** Merged `dbab096`, pushed,
  `origin/main` verified moved and equal to HEAD. Merged-tree lanes, exit codes read directly:
  cargo **287 passed / 0 failed** (unchanged — this parcel repaired reachability and removed a
  duplicate rather than adding tests, which is the honest reading of a count that did not move),
  `npm run build` exit 0 with zero warning or error lines, vitest **352/352 across 33 files**,
  no `src/bindings.ts` drift. Three pre-existing cargo warnings, none added.
  **THE BRIEF SAID `git rev-parse --show-toplevel`. THAT IS WRONG INSIDE A LINKED WORKTREE**,
  where it reports the WORKTREE's root and lands in the same wrong directory the defect was
  about. The agent found it, said so, and used `--git-common-dir` instead — the one path every
  worktree shares. **Ratified explicitly (bar 7), and verified firsthand here** rather than
  taken on report:
  `git rev-parse --show-toplevel` in a worktree returns
  `/home/volence/sonic_hacks/seraph/.claude/worktrees/agent-…`, while `--git-common-dir`
  returns `/home/volence/sonic_hacks/seraph/.git` from the worktree and the RELATIVE `.git`
  from the main checkout.
  **A CORRECTION TO THE AGENT'S OWN REASONING, and it is why the tag could not just be
  accepted.** Worktree isolation blocked it from running the test in the main checkout, so it
  simulated the layout and argued the main-checkout case *"consumes the same `--git-common-dir`
  value the worktree already produced, so it is the same computation on the same input"*. It is
  **not the same input**: from `src-tauri/` git answers `../.git`, a RELATIVE path, so the main
  checkout exercises the *other* branch of the resolver (join-against-`CARGO_MANIFEST_DIR`),
  which the worktree case never touches. Checked here directly — that branch resolves to
  `/home/volence/sonic_hacks/skdisasm` and the driver file exists — and then closed properly by
  the merged-tree run, where **`psg_table_still_matches_the_driver_it_claims_to_match` appears
  BY NAME in the run's own log with no env var set** (bar 25's corrective: a green log is not
  evidence the gate ran; its name in the log is).
  **The resolver returns `None` rather than guessing** when git cannot answer, falling back to
  the old literal so the panic still names something, and the unreachable-source FAILURE was
  preserved: a bogus `SERAPH_SKDISASM_DIR` still exits 101 with instructions. An unreachable
  source must never become a pass, which was the property most at risk in a path change.
  **F38: the two implementations were compared case by case BEFORE the dedupe, and agreed
  everywhere** (`Fm`, `Psg`, `PsgNoise`, `Dac`, absent channel, duplicate entries, plus the
  manager's two guards which sit ahead of the match and stay verbatim). One caller,
  `unbind_instrument_from_tracks`, all three of its outcomes preserved; **no test needed
  editing, which was the stated tripwire for having changed behaviour.** Delegation proven live
  rather than dead by sabotaging `channel_name` and watching
  `test_delete_instrument_resets_lane_name_to_channel_default` go red (*left: "Library Lead"
  right: "FM1"*), then restoring `driver.rs` byte-identical to HEAD.
  **NEW FINDING BOOKED — F40, AND IT IS A VACUOUS-COVERAGE ONE.** Inverting the `PsgNoise` arm
  of the now-single convention (`find(|c| c.is_noise)` → `find(|c| !c.is_noise)`), so noise
  resolves to a TONE channel, leaves **all 287 tests green**. FM is covered, noise is not.
  Cheap now that the convention lives in one place: one test against that arm guards both call
  sites. Found by the agent poisoning code it had just written, which is the habit worth having.
  **ALSO BOOKED — F41 (small):** `ZYRINX_ROM_DEFAULT` (`import/mod.rs:1461`) hardcodes one
  user's home directory. No worktree defect, since it is absolute, and it is already guarded by
  `SERAPH_ZYRINX_ROM` plus a loud panic — but the same helper would make it machine-portable.
  Different defect from F39's, so it was correctly left alone.
  **Sweep recorded because a null result is the thing nobody writes down:** the agent swept
  `src/` for `"../`, `/home/`, `~/`, `dirs::`, `env::var`, `include_str!`, `CARGO_MANIFEST_DIR`
  and `PathBuf::from`; exactly two paths leave the repo (the one fixed and `ZYRINX_ROM_DEFAULT`),
  everything else is repo-internal and already worktree-correct.
- 2026-08-30 (cont.): **F37 + F42 LANDED TOGETHER, held and pushed as one because the frontend
  lane was red between them.** Merged `cafd4af` (F37) and the flake fix, pushed at `ee75529`,
  `origin/main` verified moved and equal to HEAD. Merged-tree lanes, exit codes read directly:
  cargo **292 passed / 0 failed** (was 287; +5), `npm run build` exit 0 with zero warning or
  error lines, vitest **352/352 across 33 files on three consecutive runs**, no
  `src/bindings.ts` drift.
  **THE PUSH WAS HELD ON PURPOSE.** F37 was merged locally and green on cargo while vitest was
  red for an unrelated reason. This morning's README batch chained `npm test; git push` and
  pushed over a red lane; the booked lesson was that a landing must not be able to. So F37 sat
  unpushed until the frontend lane was green, and both went out together. Establishing that
  F37 was not the cause took one command: `git diff --name-only` showed it touched
  **`src-tauri/src/export/smps.rs` and nothing else**, so it could not reach a frontend test.
  **F37 — THE ARTIFACT IS BETTER THAN A DERIVATION: THE SHIPPED GAME ALREADY DOES THIS.**
  Verified firsthand rather than taken from the agent: `Chaos Emerald.asm` declares
  `smpsHeaderDAC Snd_Emerald_DAC` at line 7, and that label (74-78) falls straight into a lone
  `smpsStop`. `cfStopTrack` (3443-3444) does `res 7, (ix+zTrack.PlaybackControl)` — exactly the
  bit `zUpdateMusic` tests at 717-719 before calling `zUpdateDACTrack`. The slot is entered
  once, stops, and is never updated again. So the synthesized entry is not an invention; it is
  the shape the original game uses for its own drumless song.
  **AND IT IS THE SAME SONG AS F30's OUTLIER.** `Chaos Emerald` is both the one file of sixty
  declaring `$07` (the seven-entries-over-six-slots case F30 booked) and the one drumless song
  (F37's precedent). One shipped song is the worked example of both defects, found on two
  different days by two different parcels. *(The "only drumless of the 60" count is the agent's,
  carried not verified here; the shape of that one file is what this lane checked, and it is all
  the fix rests on.)*
  Implementation is minimal and gated: `needs_silent_dac = dac_count == 0` folded into the
  header count, the entry emitted FIRST, the body a lone `smpsStop`. **One line of existing code
  changed in the whole parcel** and no existing test expectation was edited. Poisoned both
  directions as the ruling required, plus a control proving the label is DEFINED (`smpsHeaderDAC`
  runs its operand through `CheckedChannelPointer`, so a dangling label is an assembly failure).
  **DIFF CLASS, for the owner's reversal if he wants it:** a song with drums is byte-identical
  and sounds identical; a drumless song's file changes; the SOUND changes only for a drumless
  song that has FM tracks, which is the case that is wrong today.
  **F42 — MY DIAGNOSIS WAS WRONG AND THE AGENT REFUTED IT WITH A PROBE.** I read the failure
  (`App.projectSwitch.test.tsx > New Project clears it`, 1342ms against a 1000ms `waitFor`
  budget) as this morning's budget-blowout flake at a new site, and briefed it that way with a
  two-part proof required. **Step 1 did not reproduce**: squeezing the budget to 1ms left the
  file passing 3/3. The agent then probed and found `handleCreate` calls `ipc.createProject`
  **synchronously, before its first `await`** — so `waitFor`'s first immediate check either sees
  it or it never happens, and **the budget cannot be what decides it**. Verified here at
  `NewProjectDialog.tsx:44-46`: three synchronous guards, and `!driverId` early-returns with
  *"Select a driver"* without ever reaching line 52's call.
  **THE REAL MECHANISM:** the helper clicked Create before the dialog's effect had loaded the
  driver list, so the create was refused and the poll waited for a call that could never come.
  **1342ms was the PRICE of a doomed assertion, not the cause of a slow one** — which is why the
  duration looked like corroboration for the wrong story. Bar 5's shape at the diagnostic level:
  a number that fits the hypothesis is not evidence for it.
  **THE UNFORCED CATCH IS THE EVIDENCE THAT MATTERS.** The agent ran the ORIGINAL file under 48
  spinners on 16 cores: clean five times, then `Open Project clears it` failed at 1029ms citing
  **line 114**, the helper's own `waitFor` — same site, same symptom, same price as production,
  with nothing instrumented. That is a reproduction of the real thing rather than of a theory.
  **NOT A PRODUCT DEFECT, and the agent was told to stop if it were.** A real user cannot fill
  two fields and click Create inside one microtask, and the dialog's refusal to create a
  driverless project is correct behaviour honestly reported. No product code was touched. All
  **4 of 4** `waitFor` sites in that file were converted to wait-for-precondition-then-assert;
  `waitFor` is no longer imported there. Invariant 6 demonstrated by neutering `resetClipboard()`
  in `App.tsx` and watching all three cases go red, then restoring.
  **A BRIEFING DEFECT OF MINE, AND IT IS THE SECOND INSTANCE OF ONE ROOT CAUSE TODAY.** I gave
  the agent a cargo baseline of **292**; its tree produced **287**, and it flagged the mismatch
  rather than assuming. Both numbers were right: **an agent worktree branches from the last
  PUSHED commit, not from my local unpushed merge.** The same root cause produced the earlier
  `d-11` report — I cited a decision record to the F37 agent that I had written but not yet
  committed, so it was structurally invisible in that agent's tree, and the agent correctly
  refused to invent it or to write my ledger for me.
  **STANDING LESSON FOR THIS LANE, both halves:** (1) **commit anything you cite before you
  dispatch**; (2) **do not hand an agent a baseline number at all — tell it to derive the
  baseline from its own tree**, which is bar 1 applied to briefs. A copied number is exactly what
  bar 1 forbids in gates, and I put one in two consecutive briefs.
- 2026-08-30 (cont.): **F40 + F41 + F43 LANDED TOGETHER — and the push was held a SECOND time
  tonight for the same reason, deliberately.** Merged and pushed at `19e6a16`, `origin/main`
  verified moved and equal to HEAD. Merged-tree lanes, exit codes read directly: cargo **293
  passed / 0 failed** (was 292; +1), `npm run build` exit 0 with zero warning or error lines,
  vitest **352/352 across 33 files on four runs executed as TWO CONCURRENT PAIRS**, no
  `src/bindings.ts` drift.
  **THE LANES WERE RUN UNDER CONTENTION ON PURPOSE.** F43's defect is invisible on an idle box —
  the agent measured its own three pre-change runs as clean — so idle green would have been the
  weakest possible evidence. Two full suites were run simultaneously, twice. This is the
  instrument matching the subject rather than the subject being made convenient for the
  instrument.
  **F40 — the noise arm is now covered, and the red-first run RE-CONFIRMED the gap.** Inverting
  `find(|c| c.is_noise)` produced `292 passed / 1 failed` with the new test as the **only**
  failure, which independently reproduces the finding that the rest of the suite is blind to it.
  The expectation is derived from each registered profile's own layout and asserted as a
  RELATIONSHIP in both directions (the resolved name must be an entry the layout flags noise,
  and must not be one it flags tone), so an inverted arm cannot satisfy it regardless of naming.
  Anti-vacuity guard fails loudly if no registered profile advertises a noise channel. Production
  code byte-identical to before (`git diff` shows zero deleted lines).
  **F41 — the hardcoded home directory is gone and the loud-when-absent behaviour was verified
  HERE, not taken on report.** `ZYRINX_ROM_DEFAULT` is replaced by a relative leaf joined to a
  resolved root; `test_support::sibling_root()` is now the single place holding the
  `--git-common-dir` reasoning, consumed by both `audio::frequency` and `import` — factored
  rather than copied, which is F38's lesson applied one day later. Re-run firsthand on the merged
  tree: `SERAPH_ZYRINX_ROM=/nonexistent/... cargo test zyrinx` exits **101** with the full
  instructions, no skip. `git grep "/home/volence" -- src-tauri/src src` now returns **nothing**.
  *(Cross-lane note: the hub counted this pattern at every tip — sigil 142, aurora 138, aeon 28,
  empyrean 15, seraph 1, oracle 0 — and asked whether seraph's one was a row. Answered no, with
  the discriminator stated: a hardcoded path that PANICS is a portability nuisance; one that
  `return`s early is a test reporting green having checked nothing.)*
  **F43 — THE MECHANISM IS NOT WHAT ANYONE GUESSED, INCLUDING ME, AND THE STARTING POINTS WERE
  PARTLY REFUTED.** The failing assertion was a guard an earlier parcel added on purpose, and it
  fired correctly. My brief offered "the `tracks` state is not populated" and "or a cross-file
  interaction". **Both wrong.** `tracks` WAS populated in the committed render, and cross-file
  leakage was refuted by checking the resolved config rather than assuming it (`pool=forks`,
  `isolate=true`, so module state is not shared; the reproduction also survives
  `resetClipboardForTest()` immediately before each mount).
  **WHAT WAS ACTUALLY ABSENT WAS THE LISTENER, NOT THE DATA.** Ctrl+C is served by a `window`
  listener installed in a `useEffect` closing over `tracks`. Painting the header and
  re-registering that listener are two different steps — commit in a microtask, passive-effect
  flush on React's scheduler via a MessageChannel macrotask — and `findByText` resolves off the
  first. RTL's `asyncWrapper` bridges the gap with a `setTimeout(0)` **bet** that it will lose to
  the scheduler; the agent measured that bet winning 20/20 idle. Under CPU contention the
  scheduler's 5ms host-yield budget lets it be preempted and repost AFTER the bet, leaving the
  MOUNT-TIME listener on `window` with `tracks == []`; the region lookup misses and
  `copyRegions([])` is a documented no-op. Instrumented proof over 600 mounts, every stale
  iteration reading `keydownAdds=1 keydownRemoves=0 headerInDom=true`.
  **THE A/B IS THE EVIDENCE, NOT THE STREAK, AND THE AGENT SAID SO ITSELF:** load-matched
  interleaved, **4 stale in 300 undrained mounts, 0 in 300 drained**. The fix removes the timing
  bet rather than lengthening it — inside `act`, React queues passive effects on the act queue and
  flushes them before act resolves. Four cases shared the exposure; the fourth is the interesting
  one, because it would have **passed for the wrong reason** (a stale handler copies nothing, so
  "nothing pastes" holds even if the behaviour under test were broken). Cases reading only props
  were correctly left alone.
  **A NON-FIX RULED OUT AND STATED:** setting `IS_REACT_ACT_ENVIRONMENT` would NOT have fixed
  this, because `asyncWrapper` forces it false for the duration of every `waitFor`/`findBy`.
  **NEW ROW BOOKED — F44, and it is a lost diagnostic across the whole frontend suite.**
  `IS_REACT_ACT_ENVIRONMENT` is never set anywhere, and `vitest.config.ts` does not enable
  `globals`, so RTL's auto-configuration never runs and **React's "update not wrapped in act"
  warnings are silenced across all 33 test files.** Verified firsthand here (`git grep` finds no
  assignment; the config has no `globals` key). Turning it on will likely surface warnings
  repo-wide, so it is its own parcel rather than a rider.
  **PROCESS, REPEATED ON PURPOSE:** F40/F41 were merged and green on cargo while vitest was
  intermittently red for an unrelated reason. Held unpushed until F43 explained it, then all
  three went out together. `git diff --name-only` established in one command that the held work
  could not be the cause (only Rust and docs had changed). Second time tonight; "it is only
  intermittent" is precisely how a lane talks itself into pushing over red.
- 2026-08-30 (cont.): **F44 LANDED — the act() warning is on again, and the parcel uncovered a
  MEASUREMENT hazard bigger than the item it was dispatched for.** Merged and pushed at
  `05ca871`, `origin/main` verified moved and equal to HEAD. Merged-tree lanes, exit codes read
  directly: cargo **293 passed / 0 failed**, `npm run build` exit 0 with zero warning or error
  lines, `npx tsc --noEmit` exit 0, vitest **352/352 across 33 files**, plus a **contended pair**
  (two suites at once) both 352/352 with **zero act warnings**. No `src/bindings.ts` drift.
  **THE HAZARD, AND IT INVALIDATES A CLASS OF PAST CLAIMS IN THIS REPO.** vitest 4 selects its
  reporter as `isAgent ? "agent" : "default"`, and std-env's `isAgent` is true whenever
  `CLAUDECODE` or `AI_AGENT` is set — which is every session like this one. The agent reporter
  runs `silent: "passed-only"` and **drops console output from PASSING tests entirely**; a
  config-level `silent: false` does not override it. So a warning printed for a human and printed
  nothing for an agent, and the agent's own first measurement of this parcel read **0 warnings**,
  which was false.
  **VERIFIED FIRSTHAND HERE with a control pair**, because it is a claim about this lane's own
  instrument: a scratch test emitting both, run twice on this machine with `CLAUDECODE=1` and
  `AI_AGENT` set, vitest 4.1.10 — `console.log` from a passing test appears **0** times under the
  default reporter and **1** time under `--reporter=verbose`, while `process.stderr.write`
  survives **both**. Probe removed after measuring.
  **CONSEQUENCE, stated plainly rather than softened: any "no warnings" claim about VITEST output
  made from an agent session in this repo was measuring a muted channel.** Pass/fail counts are
  unaffected, because failures print under every reporter — so tonight's landings, which reported
  totals and failing names rather than warning counts, stand. `npm run build`'s zero-warning
  claims are also unaffected: that is vite and tsc, a different process, not vitest's reporter.
  **This is bar 16(d)'s absence surface wearing a new costume:** the command succeeded, the output
  was real, and the emptiness meant "your channel is muted", not "there is nothing there".
  **WHAT LANDED FOR F44 ITSELF.** `IS_REACT_ACT_ENVIRONMENT` is set in `src/test/setup.ts`.
  `globals: true` was **rejected with a reason**: it would also inject `describe/it/expect` into
  every file and make RTL register a SECOND `afterEach(cleanup)` on top of the existing one — a
  large blast radius to buy one boolean.
  **THE ENABLEMENT WAS PROVEN TO FIRE, WITH A CONTROL** (an enablement that cannot warn is
  indistinguishable from none): a scratch component updated outside `act()` produced *"An update
  to Counter inside a test was not wrapped in act(...)"* with the flag on and **nothing** with it
  off, same test, same file.
  **BLAST RADIUS AND TRIAGE: 6 warnings, 1 file, 3 tests — all (a), all the F43 family, zero
  benign, zero unexplained.** `NewProjectDialog` loads its driver list in a mount effect setting
  two states; three synchronous location tests rendered, asserted and returned with both still in
  flight (2 updates x 3 tests = exactly 6). Fixed the F43 way — wait for the real precondition
  (`await screen.findByText("Flamedriver")`) — not by wrapping the symptom, and assertions were
  left unchanged. **The two tests in that same file that already awaited a precondition never
  warned**, which is the same fix arrived at independently and is the corroboration.
  **ZERO RESIDUAL, WITH A GUARD SO IT STAYS ZERO.** An act warning is re-emitted to
  `process.stderr` (which vitest does not intercept) **and fails the causing test**, since
  failures print under every reporter. That design is a direct consequence of the hazard above:
  a diagnostic that only prints is invisible to exactly the sessions that run this suite most.
  **A DEFECT THE AGENT FOUND IN ITS OWN GUARD, worth copying:** as two separate hooks, vitest's
  reverse `afterEach` ordering ran the check BEFORE `cleanup`, and the throw then skipped the
  unmount, leaking a mounted tree into later tests. Now one hook with `cleanup()` in `try` and the
  report in `finally`.
  **POISONED FIRSTHAND HERE, not accepted on report:** reverting `NewProjectDialog.test.tsx` to
  its pre-fix state on the merged tree produces exactly **6 stderr warnings and `3 failed | 2
  passed`** — the guard fires, names the component, and is specific rather than blanket. Restored
  after.
  **BOOKED — F45:** the reporter suppression is **repo-wide, not act-specific**. Any console-based
  diagnostic (React key warnings, deprecation notices, anything a library prints from a passing
  test) is invisible to every agent session running `npm test` here. F44 routed ONE diagnostic
  around it; the general question — make agent runs use a reporter that does not drop console
  output, or route other diagnostics to stderr as well — is unresolved and is its own parcel.
  **Branch-name note:** the dispatched branch name did not pre-exist, so the agent renamed its
  auto-named worktree branch to the briefed `parcel/f44-act-warnings` rather than inventing a
  different target, and flagged it for confirmation. Confirmed correct; that is the intended name.
- 2026-08-30 (cont.): **F45 RESOLVED branch-side — the reporter is pinned, and the real noise
  turned out to be somewhere else entirely.** Branch `parcel/f45-agent-reporter-output`, not
  merged, not pushed. One-line fix in `vitest.config.ts`: `reporters: ["default"]`.
  **WHAT THE "AGENT REPORTER" ACTUALLY IS, and it is the finding that decided the trade.** It is
  not a compact machine-readable format that we would be giving up. In
  `node_modules/vitest/dist/chunks/index.UpGiHP7g.js` the `ReportersMap` reads
  `"agent": MinimalReporter, "minimal": MinimalReporter` — `"agent"` is a plain **alias for
  `MinimalReporter`**, whose constructor is
  `super({ silent: "passed-only", ...options, summary: false })`. Against `default` in a
  non-TTY session it differs in exactly two ways: it drops passing-test console output, and it
  skips the per-file `✓` line. `summary: false` costs nothing, because `DefaultReporter` already
  does `if (!this.isTTY) this.options.summary = false` — that flag is the live TTY progress
  display, not the `Test Files` / `Tests` block. **So pinning it away gives up compactness and
  nothing else.**
  **WHY `silent: false` IN CONFIG NEVER WORKED, precisely.** `BaseReporter` does
  `this.silent = options.silent` then `this.silent ??= this.ctx.config.silent`. The reporter
  supplies its own `silent`, so the `??=` fallback to config never fires. Reporter *options* do
  override it — `reporters: [["agent", { silent: false }]]` was tested here and works — so the
  narrow option was real, not hypothetical.
  **THE NUMBERS, because the brief asked for the trade quantified and not asserted.** Full suite,
  same tree, logs captured whole (never through `tail`):

  | reporter | total lines | jsdom canvas noise | real lines |
  |---|---|---|---|
  | `agent` (what every session got) | 1250 | 1237 | **13** |
  | `[["agent", {silent:false}]]` | 1326 | 1237 | 89 |
  | `default` (pinned) | 1363 | 1237 | 126 |

  Making console output visible costs **~+69 real lines**; the per-file `✓` listing costs
  **~+36** on top. Total ~+105 on a run that was already 1250 lines — about **8%**. The trade is
  not close, and it is not close for a reason nobody had measured.
  **CHOSE `default` OVER THE NARROWER `[["agent", {silent:false}]]`**, paying 36 lines for it:
  agents and humans then see byte-identical structure, and the per-file listing is what lets a
  fully-skipped file be told apart from a passing one. That second reason is aurora's, arrived at
  independently — see the corroboration note below.
  **PROVEN IN BOTH DIRECTIONS, under the agent condition, with a control.** Scratch test with a
  passing `console.log`/`console.warn`, `CLAUDECODE=1` and `AI_AGENT` set throughout. Before:
  **0** occurrences of either marker. After: **both** printed, under `stdout |` and `stderr |`
  headers naming the test. With `CLAUDECODE` and `AI_AGENT` **unset** via `env -u`: identical
  output, differing only in ANSI color (vitest's separate `isAgent → disableDefaultColors()`,
  left alone). Scratch test removed.
  **THE SUMMARY LINES ARE UNCHANGED IN SHAPE**, which is the constraint several sessions' landing
  checks depend on. Before: ` Test Files  33 passed (33)` / `      Tests  352 passed (352)`.
  After: ` Test Files  34 passed (34)` / `      Tests  355 passed (355)`. Same padding, same
  wording, same position; the deltas are this parcel's own guard test (+1 file, +3 tests).
  Nothing for a landing procedure to update.
  **A GUARD THAT WAS PROVEN TO FIRE, both ways.** `src/test/reporterPin.test.ts` reads
  `vitest.config.ts` as text and fails if the pin is gone or names `agent`/`minimal`. Poisoned
  firsthand: pin deleted → `AssertionError` naming the mechanism and `1 failed | 2 passed`; pin
  set to `["agent"]` → the second assertion fires. Restored after. It also emits a **canary**
  `console.warn` from a passing test, so its absence from a log is itself the signal. Two dead
  ends worth recording: importing the config pulls `@vitejs/plugin-react` → esbuild, which throws
  `TextEncoder ... instanceof Uint8Array is incorrectly false` under jsdom; and `node:fs` fails
  `tsc` because this project has **no `@types/node`**. Vite's `?raw` import solves both.
  **BOOKED — F46, and it is the actual token cost of reading a test log here.** **1237 of 1250
  baseline lines** are one repeated string: `Not implemented: HTMLCanvasElement's getContext()
  method: without installing the canvas npm package`. It is identical under every reporter
  (counted at 1237 in all three runs above) because it comes from **jsdom's own virtual console**,
  not vitest's — vitest's jsdom env defaults `console = false` and passes no `virtualConsole`, so
  jsdom forwards straight past the reporter. It was never hidden and this parcel did not create
  it. But **99% of every test log this repo has ever handed an agent is that one sentence**, which
  is a larger context cost than the entire reporter question. Not fixed here (out of parcel);
  likely fixes are a `HTMLCanvasElement.prototype.getContext` stub in `src/test/setup.ts` or
  installing `canvas`.
  **WHAT SURFACED once passing-test output became visible** — reported, not silenced, per the
  brief. Three things, and **none is a new defect**: (1) `close-confirm unavailable: TypeError:
  Cannot read properties of undefined (reading 'metadata')` at `src/App.tsx:186`, **13
  occurrences** with stack traces (~65 lines — the bulk of the +105). It is the deliberate
  `catch` at `App.tsx` whose own comment reads *"Non-Tauri environment (tests) or missing
  permission"*, so it is expected — but it does mean the `onCloseRequested` dirty-guard branch is
  **never exercised by any test**, which is worth knowing and was previously unknowable.
  (2) and (3) `Set DAC sample failed: voice-overlap...` and `Set note voice failed:
  voice-overlap...` — both from tests *named* for surfacing a backend rejection, i.e. working as
  designed. **No React key warnings, no act() warnings, no deprecation notices appeared**, which
  is the first time that claim has been made here from an unmuted channel.
  **CORROBORATION, not echo — worth stating because the two lanes enumerated different things.**
  Aurora reached the same one-line fix from the opposite direction: they already had a pinned
  `reporters` array for an adjacent reason (*"a skip that cannot be told from a pass is a silent
  zero"*), and their enumeration parameter was **config-line-presence on vitest 4.1.4**; this
  lane's was **reporter selection on 4.1.10**. Same fix, two independent parameters, two versions
  — so the defect is not version-scoped and the mechanism is confirmed twice. Their reason also
  supplied the tiebreak between `default` and the narrower option above. **Relayed through the
  hub, not witnessed by this lane**; the 4.1.10 half was verified firsthand here.
  **`docs/OVERSEER.md` updated in the same commit**, because its verification-lanes section
  carried the F44-era standing instruction *"measure warnings with `--reporter=verbose` or not at
  all"*. That advice is now wrong, and leaving it would have kept every future session on the
  workaround while the fix sat in the config.
  **LANES, exit codes read from the runner, never through a pipe.** `npm test` **355 passed /
  34 files**, exit 0; **contended run** (four concurrent full suites) all **355/355**, exit 0;
  `npm run build` exit 0 with **zero** warning or error lines; `npx tsc --noEmit` exit 0;
  `cargo test` **293 passed / 0 failed**, exit 0 (untouched, run to prove it); `git status` shows
  only `vitest.config.ts`, `src/test/reporterPin.test.ts`, `docs/OVERSEER.md` and this file — no
  `src/bindings.ts` drift.
- 2026-08-30 (cont.): **F45 LANDED — the runner no longer hides output from agent sessions, and
  the "agent reporter" turned out to be an ALIAS, which is what made the trade one-sided.**
  Merged and pushed at `ce0a57a`, `origin/main` verified moved and equal to HEAD. Merged-tree
  lanes, exit codes read directly: vitest **355 passed / 34 files** (was 352/33; +3 and +1 are
  this parcel's own guard), `npm run build` exit 0 zero warning or error lines, `npx tsc
  --noEmit` exit 0, cargo **293 passed / 0 failed** untouched, no `src/bindings.ts` drift.
  **THE FINDING THAT DECIDED IT, verified firsthand here rather than taken from the report:**
  vitest's reporter map has **`"agent": MinimalReporter`** — a plain alias — and the selection is
  `resolved.reporters.push([isAgent ? "agent" : "default", {}])`
  (`node_modules/vitest/dist/chunks/index.UpGiHP7g.js:4241` and `coverage.DM_a_rWm.js:455`). So
  the agent reporter is **not** a compact machine-readable format bought at the price of console
  output; against `default` in a non-TTY session it differs in exactly two ways — it drops
  passing-test console output and skips the per-file line. **Pinning it away gives up
  compactness and nothing else.**
  **WHY THE CONFIG KNOB NEVER WORKED, precisely:** `BaseReporter` does `this.silent =
  options.silent` then `this.silent ??= this.ctx.config.silent`, so a reporter supplying its own
  value means the config fallback never fires. Reporter *options* do override it
  (`[["agent", { silent: false }]]` works) — so the earlier workaround-only reading was correct
  about the symptom and wrong about the ceiling.
  **NOISE QUANTIFIED BEFORE RECOMMENDING, which is what the brief asked for and what makes the
  choice defensible:** full-suite line counts — `agent` **1250**, `[["agent",{silent:false}]]`
  **1326**, `default` **1363**. Visibility costs about **+69 lines**, the per-file listing about
  **+36** more, roughly 8% of a run. The trade was not close, so the general fix was taken and
  **F44's targeted stderr-and-fail routing was NOT needed as a fallback** — it stays anyway,
  belt and braces, as the one diagnostic that fails a test rather than merely printing.
  **PROVEN BOTH DIRECTIONS with the environment held live:** a passing scratch test's
  `console.log`/`console.warn` appeared **0** times before and **both** after, with `CLAUDECODE=1`
  and `AI_AGENT` set throughout; with those unset via `env -u` the output is identical but for
  ANSI colour. Scratch removed.
  **THE CONSTRAINT HELD: the summary lines are unchanged in shape** — ` Test Files  N passed (N)`
  / `      Tests  N passed (N)`, same padding, wording and position — so nothing in this lane's
  landing procedure needed updating. It was called out explicitly rather than left for someone to
  discover, which was the point of naming it in the brief.
  **NEW GUARD, POISONED FIRSTHAND HERE:** `src/test/reporterPin.test.ts` reads `vitest.config.ts`
  as text and fails if the pin is missing or names `agent`/`minimal`. Setting the pin back to
  `["agent"]` on the merged tree fails it with *"expected '\"agent\"' not to match
  /[\"']agent[\"']/"*, restored after. It also emits a canary `console.warn` from a passing
  test, so the guard proves the visibility it guards.
  **WHAT SURFACED ONCE THE CHANNEL WAS UNMUTED — reported, not silenced, and none is new.**
  (1) `close-confirm unavailable: TypeError ... 'metadata'` (`src/App.tsx:186`) **13 times** with
  stacks — the deliberate non-Tauri `catch`, but it means **the `onCloseRequested` dirty-guard
  branch is exercised by no test at all**; (2)+(3) two `voice-overlap` rejection logs from tests
  named for surfacing them. **No React key warnings, no act() warnings, no deprecation notices —
  and that is the first time this repo can say so from an unmuted channel.**
  **NEW ROW BOOKED — F46, AND IT IS A BIGGER CONTEXT COST THAN THE WHOLE REPORTER QUESTION.**
  **1237 of 1364 log lines — 91% of every test log this repo hands a session — are one repeated
  sentence**, jsdom's `Not implemented: HTMLCanvasElement's getContext()`. Measured firsthand on
  the merged tree, not carried from the report. It is identical under every reporter because it
  comes from jsdom's own virtual console and bypasses vitest entirely, so no reporter choice
  touches it; it was never hidden and this parcel did not create it. Booked rather than folded in.
  **DOC CORRECTION IN THE SAME COMMIT, and this is the perishable-guidance rule catching its own
  entry from four hours earlier:** `docs/OVERSEER.md`'s verification-lanes section had just been
  updated to instruct every session to *"measure warnings with `--reporter=verbose` or not at
  all"*. That became **wrong the moment this landed**, and would have kept future sessions on a
  workaround for a fixed defect. Updated with the parcel rather than left to rot.
  **THE AURORA DATUM: CORROBORATION, NOT ECHO, and both halves are now anchored.** The hub
  relayed that aurora had the same fix, proven by plant on **4.1.4**; it was passed to the agent
  mid-flight **marked unverified and not to be adopted on relay**, and it reproduced independently
  here at **4.1.10**. Different versions, different repos, different motivating defects — aurora
  arrived from *"a skip that cannot be told from a pass is a silent zero"*, this lane from a
  warning silent for the suite's lifetime — and neither brief carried the other's conclusion, so
  the enumeration parameters genuinely differ (bar 19). **Aurora's half stays marked relayed, not
  witnessed; the 4.1.10 half is firsthand.** Their reason also supplied the tiebreak toward
  `default` over the narrower `[["agent",{silent:false}]]`: a fully skipped file must stay
  distinguishable from a passing one.
- 2026-08-30 (cont.): **F46 IS jsdom-SPECIFIC — aurora measured itself IMMUNE, and the negative
  result is worth as much as the finding.** Relayed from the aurora lane, their measurement, not
  reproduced here: `jsdom` is **not installed** in that repo at all, `vitest.config.ts` declares
  no `environment`, and no file carries an `@vitest-environment` pragma, so that suite is
  node-only by construction. Their full-suite profile for comparison with this lane's 1237/1364:
  **1007 lines, no dominant repeated message** (top repeat is 80 blank lines, then 16 identical
  fixture handshake lines). **So the 90% figure is real and specific to a jsdom environment, and a
  lane without one should not go looking** — recorded because a null result is the thing nobody
  writes down, and because F46 travelling as a suite-wide hazard would have cost other lanes a
  hunt for something they cannot have.
  **THE AUDIT AURORA'S REPLY PROMPTED, RUN FIRSTHAND HERE, because an open channel is not the same
  claim as nothing being ignored in it.** That distinction is theirs; this lane had reported the
  channel open without separately auditing what passes through it while tests still go green.
  Grepping the merged-tree full-suite log for warning-shaped lines outside the canvas noise
  reproduces the F45 agent's triage **exactly and independently**: **13** x `close-confirm
  unavailable: TypeError ... 'metadata'`, **1** `Set note voice failed: voice-overlap`, **1** `Set
  DAC sample failed: voice-overlap`, and **nothing else** — no deprecations, no unhandled
  rejections, no experimental-warning lines. The two voice-overlap lines come from tests named for
  surfacing them. **So: open channel, nothing silently ignored — with the one honest exception
  already booked as F47**, the close-confirm branch that prints 13 stack traces and is exercised by
  no test.
  **ATTRIBUTION, settled with aurora and recorded so a later reader does not flatten it:** the
  4.1.10 measurement and the alias-cost analysis (*"pinning it away gives up compactness and
  nothing else"*) are **this lane's, firsthand**; the 4.1.4 measurement and the canvas negative are
  **aurora's, firsthand theirs, relayed here**. Aurora states they had no basis for the cost claim
  and had measured only *that* they were immune, not *what* immunity costs.
  **Bar 19, the strongest instance this suite has produced:** aurora's pin was added so a SKIPPED
  file stays distinguishable from a passing one (*"a skip that cannot be told from a pass is a
  silent zero"*); this lane's was reached from console visibility for a warning silent for the
  suite's lifetime. **Two lanes, two unrelated motivations, one remedy** — and this lane cited
  their reason as its tiebreak before either knew it was the same fix, which is the argument
  travelling ahead of the agreement rather than the agreement being assumed from it.
- 2026-08-30 (cont.): **A DISCRIMINATOR WORTH HAVING GENERALLY, from the aurora lane, and it is
  NOT the one either lane reached for first: a warning-shaped log line is triaged by WHETHER A
  TEST IS NAMED FOR THE CONDITION, never by whether it prints a stack trace.**
  How it arrived: this lane's F47 (13 `close-confirm` traces from a branch no test covers) sent
  aurora back to re-check their own audit, where they had told their owner *"6 stderr blocks, all
  from tests deliberately exercising error paths"*. **They had counted the six; they had not
  verified "deliberately"** — the precise assumption F47 broke. Re-checked, their claim held:
  every one is attributed to a test whose NAME is about the condition being reported. **Three of
  their six print a stack trace each, so on surface shape they are indistinguishable from this
  lane's thirteen.** Same artifact, opposite meaning, and **only the test inventory tells them
  apart**.
  **Why it matters operationally:** *"the log is noisy"* invites suppression; *"the log is noisy
  from an uncovered branch"* invites a test. This lane had both kinds in one log tonight and
  triaged them correctly by luck of already knowing which was which, not by a stated rule.
  **The symmetry is the part worth keeping.** This lane blurred *"the channel is open"* into
  *"nothing is being ignored in it"*; aurora blurred *counted* into *deliberate*. **Both sentences
  sounded finished, which is the mechanism** — neither lane would have re-run its own check, and
  each only did because the other said it would not have. Bar 17's shape (an assertion of
  completeness is cheaper than the check that earns it) arriving twice in one exchange, in
  opposite directions.
  **Self-discount, carried because this document is about not trusting clean answers:** n=2, one
  evening, two lanes that were already corresponding — one lane's finding with a second's
  endorsement, not two independent derivations. Proposed to the hub for the shared protocol rather
  than banked as a private bar, per the rule that cross-tool bars change in empyrean.
  **ALSO RECORDED, aurora's local note, because it constrains how F46's outcome may be
  generalised:** aurora's reporter pin protects a **second** property this lane's does not — a
  fully SKIPPED file staying distinguishable from a passing one. So if this lane's guard shape
  (text assertion + a canary that proves the behaviour) is adopted there, **it must guard both
  properties, or a future reader who knows only the console-visibility reason will weaken it
  correctly by their own lights.** That is the perishable-precedent rule pointed at a guard rather
  than at prose: a guard carries its reason or loses it.
- 2026-08-30 (cont.): **F46 LANDED — and the noise was CONCEALING a finding rather than merely
  costing context: NO CANVAS DRAWING CODE IN THIS REPO HAS EVER EXECUTED UNDER TEST.** Merged and
  pushed, `origin/main` verified moved and equal to HEAD. Merged-tree lanes, exit codes read
  directly: vitest **361 passed / 35 files** (was 355/34; +6 is this parcel's guard), `npm run
  build` exit 0 zero warning or error lines, `npx tsc --noEmit` exit 0, cargo **293 passed / 0
  failed** untouched, `reporterPin.test.ts` still passing and unedited, no `src/bindings.ts` drift.
  **THE MEASUREMENT: 1365 log lines to 128, canvas line 1237 to 0.** Verified firsthand on the
  merged tree, not carried from the report.
  **THE FINDING, and it is the reason this was worth doing beyond tidiness.** Every one of the
  **12** `getContext("2d")` call sites in `src/` is followed by `if (!ctx) return;` — checked here
  with a grep over all twelve, not sampled. Under jsdom `getContext` returns null, so **all 1237
  messages marked a draw that aborted on its second line.** The drawing code was not merely
  untested; it had never run at all. The agent proved the change with a throwaway probe (since
  deleted): a `Knob` render that previously issued zero canvas operations now issues twenty. **A
  crash in any of that code is reachable by the suite for the first time. Nothing crashed.**
  **THE FIX MAKES jsdom's STATEMENT FALSE RATHER THAN UNSAYABLE**, which was the parcel's binding
  constraint: a recording 2D stub (`src/test/canvasStub.ts`) installed from the existing setup.
  Nothing is filtered, muted or reporter-configured, so F44 and F45 stay intact. **Suppression
  would have undone the two parcels before it in the same breath**, and the brief said so.
  **REJECTED WITH REASONS, recorded so nobody re-litigates:** the `canvas` npm package (a native
  Cairo build on every machine and CI, for a Tauri app that needs none at runtime);
  `vitest-canvas-mock` (equally fake, third-party, less control over how honestly it documents
  itself); and narrowing which tests mount canvas components (**deleting coverage to quiet a
  log** — the same defect class in a new costume, and `PianoRollRuler.scale.test.tsx` exists
  precisely to run the real component).
  **TWO DESIGN CALLS WORTH KEEPING.** The surface is an **explicit allowlist, not a Proxy**, so
  `ctx.filRect(...)` throws a TypeError as a browser would rather than being swallowed — verified
  here in the file's own comments. And **only `"2d"` is stubbed**; every other context type still
  delegates to jsdom and is still reported unimplemented, **because it still is**.
  **WHAT THE STUB DOES NOT ESTABLISH, stated in the file header and repeated here so it cannot
  drift into a stronger claim:** it rasterises nothing, `measureText` returns a nominal estimate
  rather than a measurement, and `clip`/transforms/alpha are recorded but not applied. **It is not
  evidence that anything looks right on screen.**
  **THE F6 RENDERING GATE IS NOW APPROACHABLE BUT IS NOT CLOSED, AND IT REMAINS THE OWNER'S.**
  The queue has carried a third F6 gate — whether a painted run of notes RENDERS correctly —
  as unclosable precisely because jsdom had no 2D context. `canvasOps(canvas)` now makes drawing
  commands observable in order, which **opens a path to asserting a command was ISSUED** at given
  coordinates with a given fill. That is strictly more than existed. **It is not the gate.** The
  agent deliberately wrote no test whose name implies rendering was verified, and named its guard
  for the environment supplying a context instead — the right call, and the one most likely to
  have been fudged.
  **NOTHING NEW BECAME VISIBLE, checked by diffing message classes rather than eyeballed:** 13
  `close-confirm` (F47, as booked) and 3 `voice-overlap` occurrences, byte-identical counts before
  and after; zero act warnings. Confirmed independently here (16 matching lines on the merged
  tree). **F47's 13 traces are now ~10% of a 128-line log rather than 1% of a 1365-line one**, so
  the booked item is now conspicuous — which is what an unmuted channel is for.
  **A VACUOUS ASSERTION THE AGENT CAUGHT IN ITS OWN GUARD:** one of the six new tests passed as
  `null === null` and was given a non-null assertion so it cannot. Guard proven red-first without
  the stub (`expected null not to be null`), and `tsc` caught a real typing bug in the test before
  either lane ran.
- 2026-08-30 (cont.): **AUTHORITY NOTE FOR ANY FUTURE SERAPH SESSION: this lane's licence to work
  without a boot stop is the 00:31:51Z RESUME BRIEF, NOT the 06:38:01Z "finish line" quote.**
  Both are real, both are the owner's own words, both are committed in empyrean's `docs/OVERSEER.md`
  — and **only the first one reaches this lane.**
  Verified firsthand: empyrean **`f4f3753`** is reachable on freshly-fetched `origin/main` and is a
  docs commit (+6 lines), which is the correct class for a ruling record. Its banked text quotes
  him at **2026-08-30T06:38:01Z** — *"ok, let's just remember to push and make it continue through
  the parallax/raster project if yyou don't mind..."* — and glosses it as: a lane rebooted
  mid-project does not stop at its boot stop, **"its pick is its own `next` row under the effects
  plan"**.
  **THE SCOPE IS IN THE GLOSS ITSELF: "under the effects plan".** EFFECTS-W1 is aeon, aurora and
  sigil. **Seraph is outside it, as the hub stated in the same message that relayed the quote.**
  So the 06:38:01Z quote is about a project this lane is not in, and reading it as authority here
  would be extending his words past what they say.
  **This changes nothing operationally and that is exactly why it is worth writing down.** The
  earlier RESUME BRIEF (00:31:51Z, verified at boot this session) is addressed to every lane and
  says in his voice *"Do not boot into a stop and wait for a pick: his pick is this paragraph"*.
  That already authorises continuous owner-free work here, so the behaviour is identical either
  way — **but the authority is not, and a future session citing the newer quote would be laundering
  an inference about another project into a ruling about this one.** The relay was in good faith
  and its practical instruction was right; the provenance is what needed pinning.
  **Minor citation correction, recorded because this suite's whole anchor discipline rests on
  timestamps matching:** the relay gave the quote's time as *"06:5xZ"*; the banked text and the
  commit both say **06:38:01Z**. A future reader matching on the relayed time would not find it.
- 2026-08-30 (cont.): **F47 LANDED — the guard that stops the owner losing unsaved work on window
  close is covered by tests for the first time, and `App.tsx` was not touched to do it.** Merged
  and pushed, `origin/main` verified moved and equal to HEAD. Merged-tree lanes, exit codes read
  directly: vitest **369 passed / 36 files** (was 361/35; +8 is this parcel), `npm run build` exit
  0 zero warning or error lines, `npx tsc --noEmit` exit 0, cargo **293 passed / 0 failed**
  untouched, and both prior guards (`reporterPin`, `canvasStub`) still passing **unedited**. Log
  **128 to 85 lines**; `close-confirm unavailable` **13 to 1**, and the survivor is now
  **asserted** by a test rather than accidental. `git diff` on `src/App.tsx` across the whole
  parcel is **empty** — verified here, not taken on report.
  **EVERY CASE PROVEN RED-FIRST BY BREAKING THE GUARD, and the CONTROL is the one that matters.**
  Independently reproduced here on the merged tree: deleting `if (!dirtyRef.current) return;` from
  the handler fails **exactly** `lets a close through untouched when there are no unsaved changes`
  (`1 failed | 7 passed`), with `App.tsx` restored byte-identical afterwards. **Under that break
  every DIRTY test still passes**, so a guard that intercepted *every* close would have looked
  like a fix. That is why the control carries the file.
  **MOCKING CHOICE, and the rejected option is the interesting half.** Per-file
  `vi.mock("@tauri-apps/api/window")` in the three suites that mount App — already this repo's
  convention, which is why `App.test.tsx` never printed a `close-confirm` line while the other
  three did. **Rejected: a global mock in `src/test/setup.ts`**, which would have removed all 13
  lines in one line of code but would make ~30 files that never intended to touch Tauri run the
  Tauri-PRESENT branch, leaving **no test anywhere on the browser-only branch a plain `npm run
  dev` takes**. Trading a real code path for a quieter log, on a data-loss guard.
  **FINDING BOOKED — F48, and my check NARROWED it from the agent's reading.** The `try` covers
  only **registration** (the import, `getCurrentWindow()`, the `await` on `onCloseRequested`); the
  handler body runs later, outside it. So if `ask()` or `win.destroy()` rejects at close time, the
  user clicks quit, nothing happens, **there is no console line at all**, and because
  `preventDefault()` already fired **the window is left unclosable**.
  **CORRECTION TO THE AGENT'S FRAMING, checked firsthand:** it raised this via a missing
  `core:window:allow-destroy` permission. That permission **IS granted** —
  `src-tauri/capabilities/default.json:10` lists it. So the missing-permission route is **not
  live**, and F48 is a **latent** robustness gap reachable by other rejection causes, not a
  configuration defect waiting to fire. Booked at that severity rather than the higher one.
  **FINDING BOOKED — F49: `confirmDiscard`'s dirty branch is covered by NOTHING**, and it can
  disagree with the close guard **from the same cause**. The only `dirty: true` anywhere in the
  suite is a dirty-indicator test that never opens or creates a project, so only the
  `if (!dirtyRef.current) return true` early-out ever runs; `ask` is mocked in two files and
  asserted in none. With an unavailable dialog, `confirmDiscard` rejects and the button **silently
  does nothing**, while the close guard has already called `preventDefault()` and leaves the window
  **unclosable**. Same failure, opposite user-visible outcome, neither says anything.
  **A FACTUAL CORRECTION TO THE AGENT, AND AN OMISSION OF MINE IT EXPOSED.** The agent omitted the
  harness's `Claude-Session:` commit trailer, reasoning that *"every commit in this repo's history
  lacks any Claude-identifying trailer"* and citing this lane's standing memory. **That memory is
  about `Co-Authored-By: Claude`, which is a different trailer, and the premise is false:** counted
  here, **3 of the last 20 commits carry `Claude-Session:`** and older history carries more.
  **What the count actually exposed is mine:** most of tonight's commits are missing it because I
  wrote every message through a heredoc without it. The agent's two commits were left as they are
  (unpushed rewriting would buy nothing and they match tonight's majority); **the trailer resumes
  from this commit forward.** Recorded because a wrong premise that happens to reach a defensible
  action is still a wrong premise, and because the lapse it revealed was the controller's.
