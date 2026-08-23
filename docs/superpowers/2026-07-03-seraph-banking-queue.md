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
