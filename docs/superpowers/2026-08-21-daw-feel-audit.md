# Seraph DAW-Feel Audit — 2026-08-21

**Branch:** `audit/daw-feel`. **Method:** code-grounded walkthrough of seven composing
scenarios (no GUI launched, no emulator). Every behavioral claim cites the file/line that
implements it; anything not decidable from code is marked **UNVERIFIED — needs live check**.
This audit deliberately does NOT re-list the 41 mechanical gaps (G1–G41,
`docs/superpowers/2026-07-03-seraph-banking-queue.md`, Log 2026-08-21); G-numbers are
cross-referenced where a finding overlaps. In-flight work (piano-roll ruler + ruler-drag
zoom + loop handles, loop-wrap follow-scroll, live marquee highlight, quiet-voice gain,
per-track voice-switching research) is excluded.

Classification: **missing** (capability absent) / **clunky** (possible but laborious) /
**discoverability** (possible but nothing reveals it) / **feedback** (works but the user
can't tell). Severity = centrality to a composing session: **critical / high / med / low**.

---

## Scenario A — Cold start to first sound

Traced path (new user, empty library aside):

1. Launch → welcome screen, `New Project` / `Open Project` buttons only
   (`src/components/MainArea.tsx:42-53`).
2. Click **New Project** → modal requires Name, a **Browse** trip through the OS
   directory picker (the Location field is `readOnly` — typing a path is not allowed,
   `src/components/NewProjectDialog.tsx:70-82`), driver/tempo default. **Create**
   (`NewProjectDialog.tsx:35-52`). No Enter-to-create, no Esc-to-cancel (no keydown
   handlers in the dialog; only backdrop click closes, line 55).
3. Project opens with the full seeded lane roster — every lane **instrument-less**
   (create seeds tracks with `instrumentId: null`; banking-queue log 113-138).
4. First *possible* sound: hold-click a Library entry name = audition at C4
   (`src/components/LibraryPanel.tsx:298-303`). ~5 user actions in. But nothing on
   screen says "hold to audition" beyond a hover tooltip.
5. First *sequenced* sound requires knowing three unadvertised gestures:
   (a) give a lane a voice — drag a library entry onto a track header
   (`src/components/TrackHeader.tsx:100-127`) or click `+ FM Patch`, which silently
   binds to the lowest empty FM lane (`src-tauri/src/project/manager.rs:419-423,462`);
   (b) double-click an empty lane to create a one-bar region
   (`src/components/TimelineCanvas.tsx:508-513`); (c) double-click the piano-roll grid
   to draw a note (`src/components/PianoRollCanvas.tsx:332-352`) — the draw auditions
   the pitch, so **first pitched sound of your own ≈ action 8**; press Space for the
   sequenced version (≈ action 9).

**The trap:** if the user skips step 5a (nothing prompts it), every note they place is
silently dropped from playback — `build_snapshot` emits a `NoteOn` only when the track
resolves an instrument (`src-tauri/src/project/manager.rs:825`, `resolve_instrument_data`
returns `None` for `instrument_id: None`, `manager.rs:1030-1036`), and the piano-roll
audition also silently no-ops (`src/components/PianoRoll.tsx:353-354`). The note draws
fine, the region draws fine, playback runs — and nothing sounds, with zero indication of
why. `TrackHeader` renders no "no voice" state (`TrackHeader.tsx:186-272`). → **F2**.

Also: the seeded-roster + `+ FM Patch` auto-bind behavior is invisible ("+ FM Patch"
reads as "add a patch to a list", not "give FM1 a voice") → **F17**.

## Scenario B — Sketch an 8-bar loop while looping

Loop setup: drag the ruler's upper half (bar-snapped bracket) or `L` for one bar at the
cursor (`src/components/TimelineRuler.tsx:153-194`, `src/App.tsx:99-117`). Fine.

- **Can you edit while looping?** Structurally yes: `reload_snapshot` preserves
  playing/tick/loop (`src-tauri/src/sequencer/mod.rs:65-77`), and region-level ops do
  reload (`src/components/ArrangementView.tsx:177,200,271,319,326,347` — G30).
  **But note-level edits never reload**: `PianoRoll.tsx` contains *zero*
  `reloadSequence` calls (verified by grep across the tree — only App/TopBar/
  TrackHeader/ArrangementView call it). Playback consumes the snapshot built at
  play-time (`src-tauri/src/ipc/commands.rs:960-971`), so a note you place, move,
  resize, delete, transpose, nudge, paste, or re-velocity **does not sound until you
  stop/restart or trigger a region-level op**. The core sketch-into-the-loop workflow
  is broken at the exact center of the feel. → **F1** (critical). UNVERIFIED live, but
  the code path is unambiguous.
- **Audition at pitch while placing:** yes on note press and draw start
  (`PianoRollCanvas.tsx:297,350`) — but the preview plays on hardware **channel 0**
  (FM1 / PSG1) unconditionally (`commands.rs:386-401` FM; `do_preview_psg` channel 0,
  `commands.rs:487-501`), stealing that channel from the running loop
  (recovery via cache invalidation, `src-tauri/src/audio/engine.rs:248-259`). While
  sketching the FM1 part itself that may pass unnoticed; auditioning against any other
  playing channel audibly fights FM1's part. → **F4**. UNVERIFIED audibly.
- **Hear a note when you click it:** yes (press = audition). Keyboard transpose and
  drag-across-pitches give no sound (`PianoRoll.tsx:202-224` has no audition call;
  drag auditions only the initial press). → **F9**.
- **Step-record:** none. No QWERTY note input at all — every key handler in the tree
  is transport/edit-verbs only; the `PianoKeys` widget is mouse-only
  (`src/widgets/PianoKeys.tsx:18-26`). G36 booked (owner call open). → **F5**.
- **Note-placement rhythm:** double-click per note, optional keep-holding to drag
  length (`PianoRollCanvas.tsx:332-352,432-469`). No pencil/paint mode; right-click is
  suppressed and does nothing (`PianoRollCanvas.tsx:590-592`) — no erase, no context
  menu. A 16-step drum pattern = 16 double-clicks. G13/G14 booked. → **F6**.
- **Drums on DAC:** the DAC roll shows 29 named S3 drum rows
  (`PianoRoll.tsx:42-51`) — but for from-scratch content **every row plays the same
  single sample**: the sequencer ignores DAC pitch and plays the resolved instrument
  (`src-tauri/src/sequencer/mod.rs:243-247`); per-note samples exist in the model
  (`src-tauri/src/model/song.rs:77` `Note.instrument_id`) and in playback resolution
  (`manager.rs:819-824`) but no UI or IPC can author them (`add_note` has no
  instrument param, `src/api/ipc.ts:256-264`). A kick+snare+hat kit requires one
  track per sample stacked on the DAC channel (channel merge:
  `manager.rs:769-785`) — workable, laborious, and totally undiscoverable, while the
  labeled rows actively mislead. → **F7**.

## Scenario C — Editing feel

- **Lengths:** edge-drag with grid snap, Ctrl = 1-tick fine (`PianoRollCanvas.tsx:471-501`). Good.
- **Micro-timing:** Ctrl+Arrow = 1-tick nudge, plain Arrow = grid step, block-whole-move
  (`PianoRoll.tsx:241-264`, `src/utils/pianoRollEdit.ts:61-78`). Good — but a nudge
  cannot push past the region end (G21 auto-extend booked).
- **Velocity:** single click on the lane sets one note's velocity from click height
  (`src/components/VelocityLane.tsx:56-73`). No drag-paint (G17 booked), no
  multi-note edit, targets the *first* note matched by x regardless of pitch or
  selection — chords are un-editable per-voice; no numeric entry; no audition of the
  result. → **F21**.
- **Transpose a phrase:** marquee + Arrow/Ctrl+Arrow — solid (`PianoRoll.tsx:202-224`),
  silent (F9).
- **Duplicate 4 bars and vary:** region Ctrl+D / Ctrl+C/V at the bar-snapped cursor
  (`ArrangementView.tsx:181-276`), note Ctrl+D appends after selection end
  (`PianoRoll.tsx:318-343`). Works. Paste that overflows the region end drops notes
  with only a `console.warn` (`PianoRoll.tsx:289-293`) — invisible in-app. → **F24**.
- **Two channels at once:** impossible. The bottom panel opens only the *last*
  selected region (`src/App.tsx:405`); the roll renders only that region's notes
  (`PianoRoll.tsx:109-120`) — no ghost notes, no multi-region editing, no
  second roll. Bass-against-lead work = open/close cycling, losing note selection each
  swap. → **F18**. (No G-number visible for this in the log.)
- Deselecting regions needs a target: empty-canvas click doesn't clear
  (`TimelineCanvas.tsx:491-495`; empty marquee keeps selection, line 434), and the
  roll closes only via its tiny `x` (`PianoRoll.tsx:405`) — no Esc. → **F19**.

## Scenario D — Song assembly

- **Sections:** `Region` has no name and no color (`src-tauri/src/model/song.rs:60-67`);
  the ruler shows bar numbers only (`TimelineRuler.tsx:84-106`). No markers, no
  sections, no region labels — verse/chorus/bridge live in the composer's memory of
  bar numbers. Rearranging works mechanically (multi-select marquee + group drag,
  `TimelineCanvas.tsx:416-446`; copy/paste at cursor) but you're moving anonymous
  colored boxes. → **F10**.
- **Navigating 3+ min:** at default zoom one bar ≈ 75 px → a 90-bar song ≈ 6,800 px
  with **no horizontal scrollbar** (`ArrangementView.module.css:26-27` overflow-x
  hidden; scroll is state-only, `src/hooks/useArrangementZoom.ts`), no minimap, no
  zoom-to-fit, no keyboard paging. Arrangement Ctrl+wheel zoom is not cursor-anchored
  (tpp changes with scrollLeft untouched, `useArrangementZoom.ts:40-44`) so zooming
  drifts the view — the piano roll got this right (`PianoRoll.tsx:89-100`). G31/G32
  booked for zoom polish. → **F11**.
- **Song end:** none — `advance` has no end-of-song condition
  (`src-tauri/src/sequencer/mod.rs:137-155`): play runs forever past the last note,
  UI stays "playing". WAV export renders a fixed user-supplied duration (default 60 s,
  `src/components/TopBar.tsx:93-109`). Known open design Q (booked in Wave-3 list) —
  cliff noted, not redesigned. → **F12**.

## Scenario E — Sound-design loop

- **Audition library voices in context:** possible only by ear-splicing — audition
  plays on ch0 over/into the mix (F4). Assign-by-drag *is* audible-current while
  looping (drop calls `reloadSequence`, `TrackHeader.tsx:121`) — the actually good
  path is drag-swap-listen, but nothing reveals it (drag target cue appears only
  mid-drag, `TrackHeader.tsx:100-110`).
- **Tweak FM params and hear it:** knobs write `updateFmInstrument` only —
  **no reload** (`src/components/FmEditor.tsx:30-38`); the running snapshot embeds
  patch bytes per NoteOn (`manager.rs:833-842`), so the loop keeps playing the old
  patch until stop/start. The editor's own preview keys do reflect edits — on ch0,
  fighting the loop (F4). "Tweak while looping" is the sound-design heartbeat; today
  it's tweak-stop-play-listen. → **F3**. UNVERIFIED live.
- **A/B two candidates:** no mechanism — re-drag each voice (each swap is an undoable
  track edit, `manager.rs:632`), no slot/compare/hold-to-toggle. Clunky but possible.
  (No G-ref visible.)
- Instrument rename/duplicate/delete UI exists only in `InstrumentBrowser`, which is
  mounted nowhere (only `Sidebar.tsx` imports it; nothing imports `Sidebar` —
  verified by grep). G7 booked (dead Sidebar).

## Scenario F — Mix pass

- **Balance while playing:** per-track slider fires `updateTrack` + `reloadSequence`
  **per input event** during the drag (`TrackHeader.tsx:162-171`), and every reload
  runs `silence_all` + reprogram (`sequencer/mod.rs:65-77`) — a volume ride during
  playback likely machine-guns the audio with silence/reprogram cycles. Undo-grouped
  (nice, `TrackHeader.tsx:153-160`) but the audible feel is suspect.
  → **F13**. UNVERIFIED audibly — top live-check candidate.
- **Mute/solo:** one click each, reloads immediately (`TrackHeader.tsx:129-147`) —
  correct semantics (`build_snapshot` honors solo-any, `manager.rs:767-776`). Good.
  No keyboard access (F20).
- **Metering:** per-track level bar polled every 60 ms while playing
  (`ArrangementView.tsx:145-155`, `TrackHeader.tsx:184,262-270`), color thresholds
  (`TrackHeader.tsx:44-48`), zeroed when stopped; plus the always-on spectrum strip
  (`src/App.tsx:338`). No master meter next to the master slider, no peak hold, no
  clip latch. Master volume resets to 100% every launch (component state,
  `TopBar.tsx:46`). → **F14**.

## Scenario G — Session hygiene

Save/dirty/confirm-on-close all exist and are honest (dirty polling 1 s,
`App.tsx:66-71`; confirm dialogs `App.tsx:160-198`). But **reopen loses the entire
workspace**: the project file persists song data only (`manager.rs:308-341`; model has
zero view fields, `song.rs`), and there is no client-side persistence at all (no
localStorage/sessionStorage anywhere — verified by grep). Gone on reopen: open region /
piano roll, arrangement zoom + scroll, piano-roll zoom + grid size, seek position, loop
range + enabled state, snap mode, bottom-panel height/collapse, collapsed channel
groups, library filters/selection. Additionally there is no recent-projects list —
every Open is a full OS directory-picker expedition (`App.tsx:260-276`), and every
New requires Browse (A). → **F15**, **F16**.

## Keyboard-centricity pass

Complete shortcut inventory (every `keydown`/`onKeyDown` in `src/`):

| Where | Keys |
|---|---|
| App (`App.tsx:200-253`) | Ctrl+S save; Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y; Space play-pause (+stop-double-tap); `l` loop; Home |
| Arrangement (`ArrangementView.tsx:157-280`) | Delete/Backspace, Ctrl+D/C/V on *selected regions* |
| Piano roll (`PianoRoll.tsx:195-347`) | Arrows transpose/nudge (+Ctrl), Delete/Backspace, Ctrl+C/X/V/A/D on *selected notes* |
| Inline edit fields | Enter/Esc commit/cancel (tempo `TopBar.tsx:83-86`, rename `TrackHeader.tsx:83-89`, Knob `Knob.tsx:126`, tag edit `LibraryPanel.tsx:329`) |

Everything else is mouse-only. A fluent composer would expect keys for: seek by
bar/beat (arrows with nothing selected do nothing), zoom in/out/fit, grid/snap cycling,
open/close the piano roll (Esc at minimum — F19), track focus + M/S/volume on the
focused track, tempo nudge, loop-range set (only re-arm `l` exists), New/Open (G40
booked), playing notes/step entry (G36 booked, F5), and dialog Enter/Esc — **no dialog
in the app handles Escape or Enter** (zero keydown handlers in NewProjectDialog,
AddTrackDialog, ImportDialog, LibraryRootsDialog — verified by grep). Drags and
marquees have no Esc-cancel either. Minor: the loop toggle matches `e.key === "l"`
only (`App.tsx:244`) — dead under CapsLock/Shift. No keymap/help panel (G39 booked).
→ **F20**.

---

## Findings table

| ID | Finding | Class | Severity | Where | G-ref |
|----|---------|-------|----------|-------|-------|
| F1 | Note-level edits inaudible while transport runs (PianoRoll never reloads sequence) | missing | **critical** | PianoRoll.tsx (no reloadSequence); commands.rs:960-971 | G30 fixed regions only — notes missed |
| F2 | Silent-track trap: instrument-less lanes drop notes with zero feedback; audition no-ops | feedback + discoverability | **critical** | manager.rs:825,1030-1036; PianoRoll.tsx:353-354; TrackHeader.tsx | adjacent to booked "stale lane name" wart |
| F3 | FM/PSG param edits inaudible during playback (snapshot embeds patches; editors don't reload) | missing | **high** | FmEditor.tsx:30-38; manager.rs:833-842 | — |
| F4 | All previews/auditions hardcode ch0 → auditioning fights the playing mix | clunky | **high** | commands.rs:386-401,487-501; engine.rs:248-259 | — |
| F5 | No QWERTY note input / step record anywhere | missing | **high** | grep; PianoKeys.tsx:18-26 | G36 (booked, owner call open) |
| F6 | Double-click-per-note placement; no pencil/paint; right-click inert (no erase/menu) | clunky | **high** | PianoRollCanvas.tsx:332-352,590-592 | G13/G14 (booked) |
| F7 | From-scratch DAC kits impossible on one track; drum-name rows all play one sample | missing + feedback | **high** | sequencer/mod.rs:243-247; song.rs:77; ipc.ts:256-264; PianoRoll.tsx:42-51 | — |
| F8 | Piano-roll PSG audition has no stop path (looped envelope may ring); FM audition fixed 500 ms | feedback | med | PianoRoll.tsx:349-365; engine.rs:236-259 | — |
| F9 | No audible pitch feedback on transpose or drag-across-pitches | feedback | med | PianoRoll.tsx:202-224; PianoRollCanvas.tsx:297 | — |
| F10 | No section markers / region names / colors — song assembly by bar-number memory | missing | **high** | song.rs:60-67; TimelineRuler.tsx:84-106 | — |
| F11 | No h-scrollbar/minimap/zoom-to-fit; arrangement zoom not cursor-anchored | clunky | med | ArrangementView.module.css:26-27; useArrangementZoom.ts:40-44 | G31/G32 (booked) |
| F12 | No song end: playback runs forever past last note; export needs manual duration | missing (design Q) | med | sequencer/mod.rs:137-155; TopBar.tsx:93-109 | booked design Q — noted only |
| F13 | Volume-slider ride = reload (silence_all+reprogram) per input event during playback | feedback | **high** | TrackHeader.tsx:162-171; sequencer/mod.rs:65-77 | — |
| F14 | No master meter/clip latch; meters dead when stopped; master vol resets per launch | missing | med | TrackHeader.tsx:44-48,184; TopBar.tsx:46 | — |
| F15 | Zero view-state persistence: reopen loses roll/zoom/scroll/loop/snap/panel/filters | missing | **critical** | manager.rs:308-341; song.rs; grep (no localStorage) | — |
| F16 | Cold-start ceremony: forced Browse, no default location/scratch project, no recents | clunky | med | NewProjectDialog.tsx:70-82; App.tsx:260-276 | — |
| F17 | Core gestures invisible: lane dbl-click, grid dbl-click, ruler halves, header drag-drop, auto-bind | discoverability | **high** | TimelineCanvas.tsx:508-513; TimelineRuler.tsx:146-151; manager.rs:419-423 | G39 (keymap panel, booked) |
| F18 | One region at a time: no ghost notes, no multi-region editing across channels | missing | **high** | App.tsx:405; PianoRoll.tsx:109-120 | — |
| F19 | Can't click-empty to deselect regions; roll closes only via tiny x; no Esc | clunky | low | TimelineCanvas.tsx:434,491-495; PianoRoll.tsx:405 | — |
| F20 | Mouse-only surface: no seek/zoom/track-focus/M-S/dialog keys; no Esc anywhere; `l` case-sensitive | missing | **high** | inventory above | G39/G40 (booked) |
| F21 | Velocity lane: click-only, first-hit-by-x (chords un-editable), no paint/numeric/audition | clunky | med | VelocityLane.tsx:56-73 | G17 (booked) |
| F22 | Single-note readout is header text only; no inspector for exact tick/len/vel | missing | low | PianoRoll.tsx:377-387 | S4 NoteInspector (planned) |
| F23 | 1 s polling for dirty state + track list — laggy dirty dot / cross-view refresh | feedback | low | App.tsx:66-71; ArrangementView.tsx:133-136 | — |
| F24 | Failures console-only: paste overflow skips, library assign errors, save-to-library errors | feedback | med | PianoRoll.tsx:289-293; TrackHeader.tsx:125-126; FmEditor.tsx:113 | — |

## Top-10 biggest feel wins (ranked)

1. **Hear what you just placed** — reload the sequence on note-edit commits (or make the
   sequencer read live data). Fixes the sketch-into-the-loop core. (F1)
2. **Kill the silent-track trap** — a loud "no voice" state on instrument-less lanes
   plus a default voice (or one-click assign) at region creation. (F2)
3. **Session restore** — persist open region, zoom/scroll, loop range, snap/grid, panel
   layout; add a recent-projects list. Reopening should feel like sitting back down. (F15)
4. **Live patch tweaking** — instrument edits audible on the next note while looping. (F3)
5. **One-gesture note entry + right-click erase** — pencil-style draw and erase; ends the
   double-click tax. (F6, with G13/G14)
6. **QWERTY audition + step entry** — hands-on-keys composing. (F5/G36)
7. **Audition on a free channel** — route previews to an unused/overlay channel so
   in-context audition doesn't fight the mix. (F4)
8. **Region names/colors or ruler markers** — make verse/chorus visible for assembly. (F10)
9. **DAC kit authoring** — per-note sample UI (the model already supports it) or per-pitch
   sample mapping; make the drum-name rows true. (F7)
10. **Arrangement navigation** — cursor-anchored zoom, h-scrollbar or minimap,
    zoom-to-fit. (F11, with G31/G32)

## 15-minute owner play-test script

Confirms/refutes the highest-severity UNVERIFIED findings, in order:

1. **(F1, 3 min)** New project, voice on FM1, one-bar region with a note, arm loop, play.
   While looping: draw a second note; drag the first to a new pitch; delete one.
   *Expected per code: none of it sounds until you stop/restart (but dragging the
   region itself, or muting/unmuting any track, makes everything current).*
2. **(F2, 2 min)** Same project: on a lane with NO voice, create a region, place notes,
   play. *Expected: total silence, no cue anywhere. Also click the piano-roll keys
   column — expected: nothing.*
3. **(F3, 2 min)** While the FM1 note loops, open its instrument, drag TL/algorithm
   hard. *Expected: loop timbre unchanged until stop/play; the editor's preview keys
   DO reflect the change.*
4. **(F4, 2 min)** While looping with FM1 sounding, hold-audition a library entry, then
   click piano-roll keys. *Expected: FM1's part drops/glitches during audition and
   recovers on its next note-on.*
5. **(F13, 2 min)** Add notes on two FM lanes, play, then ride one track's volume
   slider slowly. *Listen for stutter/zipper — every slider event silences and
   reprograms all channels.*
6. **(F8, 1 min)** Give a PSG lane a looping envelope voice (loop point set), click a
   note in its piano roll once. *Expected per code: it rings until something else
   stops previews.*
7. **(F15, 2 min)** Mid-work: zoomed in, roll open, loop armed, snap=Beat. Save, close,
   reopen. *Expected: all of it gone — default zoom, no roll, no loop, snap=Bar.*
8. **(F12+F24, 1 min)** Play past the last region — does it ever stop? Then copy 2 bars
   of notes and paste 1 bar before the region end — *expected: half the notes silently
   missing (warning only in devtools console).*

## BLOCKED / UNVERIFIED for controller follow-up

- **Nothing blocked.** All scenarios were traceable in code.
- UNVERIFIED (audible/live confirmation needed, covered by the script above):
  F1 (note edits inaudible mid-loop — code-certain, feel-severity needs ears),
  F3 (patch edits inaudible mid-loop), F4 (ch0 steal audibility), F8 (endless PSG
  audition), F13 (volume-ride stutter — the reload storm may or may not be audible
  at 60 ms poll + per-event reloads), and the exact audible cost of `silence_all`
  on every region-op reload during playback (brief note-cut per commit,
  `sequencer/mod.rs:70`).
- The full G1–G41 enumeration is not present verbatim in the queue doc (only ~25
  G-numbers are named in the Log); cross-refs here cover the named ones. If the full
  list lives elsewhere, a few "no G-ref" rows above (F7, F10, F18) may overlap it.
