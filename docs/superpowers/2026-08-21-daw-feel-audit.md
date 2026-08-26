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

> ### ⚠ RE-GROUNDING PASS 2 — 2026-08-22 (branch `docs/reground-feel-audit`, base `3d72793`)
>
> Everything below the "Findings table" heading was re-checked row by row against
> `main` at `3d72793`. **Read the `Status (2026-08-22)` column before funding
> anything off this document** — six rows changed verdict and three had a false
> premise. The Top-10 was re-ranked; the play-test script was rewritten (it was
> instructing the owner to confirm behaviour that four merged parcels had already
> fixed, which would have burned a gate that cannot be re-run cheaply).
>
> **The Scenario A–G prose, the keyboard-centricity pass and the shortcut
> inventory below still carry their ORIGINAL `file:line` citations from
> 2026-08-21.** Those coordinates were never re-grounded and most have drifted;
> treat them as historical provenance, not as addresses. What HAS been re-checked
> in the prose is its *claims* — see **"Prose drift (Scenarios A–G + keyboard
> pass)"** immediately before the findings table for the enumerated list of prose
> statements that are now false, each with a symbol-level correction.

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

## Prose drift (Scenarios A–G + keyboard pass) — enumerated 2026-08-22

The prose above was written on 2026-08-21 and was explicitly excluded from the
first citation re-grounding. Its `file:line` coordinates are stale as
coordinates; the list below is every **claim** in it that is now false or
narrowed. Claims not listed here were re-checked and still hold, except where
marked *(carried forward — not re-verified)*.

**Scenario A**

- *"the Location field is `readOnly` — typing a path is not allowed"* — **FALSE**.
  `NewProjectDialog`'s location `<input>` has a plain `onChange` and is prefilled
  from `mostRecentLocation()`, with a recent-locations suggestion listbox and a
  `defaultPath`-seeded `handleBrowse` (`049721b`, `87bc14f`). See F16.
- *"No Enter-to-create, no Esc-to-cancel (no keydown handlers in the dialog)"* —
  **still true**, verified with a control: zero `onKeyDown`/`keydown` in
  `NewProjectDialog.tsx`, `AddTrackDialog.tsx`, `ImportDialog.tsx`,
  `LibraryRootsDialog.tsx`, while the same grep shape finds `onClick` in all four.
- *"the full seeded lane roster — every lane instrument-less"* — **still true**
  (`ProjectManager::default_tracks_for_layout` builds every lane with
  `instrument_id: None`), **but no longer silent about it**: unbound lanes now
  render a `no voice` badge and a de-emphasised name (F2).
- *"first pitched sound of your own ≈ action 8"* — **narrowed**. Draw Mode (`B`,
  or the header `Draw` button) makes a single left-click place a note, so the
  grid double-click is no longer the only entry gesture (F6, `8359f75`).

**Scenario B**

- *"`PianoRoll.tsx` contains zero `reloadSequence` calls"* — **FALSE**, and this
  was the audit's own #1. `PianoRoll.tsx` now calls `ipc.reloadSequence()` on
  **every** mutating commit; all thirteen mutation paths were enumerated (F1).
- *"right-click is suppressed and does nothing — no erase"* — **FALSE**.
  `PianoRollCanvas.handleMouseDown`'s `e.button === 2` branch erases the note
  under the cursor in both modes. The container's `handleContextMenu` is still
  `preventDefault`-only, but that is now suppression of the browser menu, not
  absence of a verb (F6).
- *"A 16-step drum pattern = 16 double-clicks"* — **FALSE**. One Draw-Mode drag
  paints the run as one gesture, one undo step, one reload.
- *"no UI or IPC can author [per-note samples] (`add_note` has no instrument
  param)"* — **FALSE**. `add_note` takes an optional `instrument_id`,
  `set_note_instrument` exists as IPC, `build_snapshot` resolves
  `note.instrument_id` through `resolve_instrument_data_by_id`, and the piano
  roll has a drop gesture for it (`abb22a9`). That gesture could not reach a DAC
  lane — the only lane F7 was about — which was **F25**; `ea1adcf` closed it
  with a per-note Sample picker, so per-note DAC samples are now authorable.
- *"drag auditions only the initial press"* — **narrowed**: a Draw-Mode paint run
  auditions on each new pitch (`paintCellUnderPointer`). Move-drag across
  pitches is still silent (F9).

**Scenario C**

- *"Deselecting regions needs a target … no Esc"* — **still true** for regions
  (`TimelineCanvas.handleClick` acts only on a hit; the marquee's `handleMouseUp`
  calls `onSelectRegions` only when `hits.length > 0`), but see the F19/F20
  correction: Esc is not absent from the app, only from these surfaces.
- *"Paste that overflows the region end drops notes with only a `console.warn`"* —
  **still true for the overflow skip**; a paste that the backend *rejects* now
  surfaces in-app through `showVoiceHint` (F24).

**Scenario D**

- *"WAV export renders a fixed user-supplied duration (default 60 s)"* — **FALSE
  in the user's favour of the wrong direction**: `TopBar.handleExportWav` calls
  `ipc.exportWav(path, 60)` with a hardcoded literal. There is no duration input
  anywhere in the app. The finding is slightly *worse* than written (F12).
- *"Arrangement Ctrl+wheel zoom is not cursor-anchored"* — **still true**
  (`useArrangementZoom`'s `handleWheel` rescales `ticksPerPixel` and never
  touches `scrollLeft`), but the sibling `zoomAtBy` (ruler vertical-drag zoom)
  *is* anchored via `zoomAroundPixel`. The seam exists; the wheel path just
  doesn't use it (F11).

**Scenario E**

- *"knobs write `updateFmInstrument` only — no reload"* — **FALSE**. Both
  `FmEditor` and `PsgEditor` call `scheduleReloadSequence` (`7f59c18`). The audit
  had only inspected `FmEditor`; `PsgEditor` had the same hole and was fixed in
  the same parcel.
- *"`InstrumentBrowser` … is mounted nowhere (only `Sidebar.tsx` imports it;
  nothing imports `Sidebar`)"* — **still true**, re-verified with an unfiltered
  grep: `Sidebar` is referenced only inside `Sidebar.tsx` itself. G7 still open.

**Scenario F**

- *"per-track slider fires `updateTrack` + `reloadSequence` per input event …
  machine-guns the audio"* — **FALSE**. `TrackHeader.handleVolume` still commits
  per event (deliberate) but reloads through the `scheduleReloadSequence`
  coalescer, and `reload_snapshot` no longer calls `silence_all` (F13, `7f59c18`).
- Metering claims (no master meter, meters zeroed when stopped, master volume
  resets per launch) — **all still true**, verified (F14).

**Scenario G**

- *"there is no client-side persistence at all (no localStorage/sessionStorage
  anywhere — verified by grep)"* — **FALSE**. `src/utils/recentLocations.ts` is
  the app's first `localStorage` user (key `seraph.recentProjectLocations.v1`).
  It stores project *locations* only, so every view-state item F15 lists is still
  lost on reopen — but the "no persistence seam exists" premise is dead, and the
  queue has already designated this module as the seam F15 should extend.
- *"there is no recent-projects list"* — **still true**. `App.handleOpenProject`
  seeds the OS picker's `defaultPath` from the MRU; it does not offer a list of
  recent *projects* to open, and `MainArea`'s welcome screen still shows two bare
  buttons.

**Keyboard-centricity pass**

- *"no Esc anywhere"* / *"Drags and marquees have no Esc-cancel either"* — the
  second clause is true, the first is **FALSE** and was false when written. Three
  `key === "Escape"` handlers exist: `TrackHeader.handleRenameKeyDown`,
  `TopBar.handleMetaKeyDown`, and `LibraryPanel`'s inline tag-edit `onKeyDown` —
  all three of which this document's OWN inventory table already lists under
  "Inline edit fields". The load-bearing claim (*"no dialog in the app handles
  Escape or Enter"*) is **still true and re-verified with a control**.
- The inventory is **missing one shortcut**: `B` (plain, no modifiers) toggles
  Draw Mode, handled in `PianoRoll`'s `handleKeyDown` (`8359f75`).
- *"the loop toggle matches `e.key === "l"` only — dead under CapsLock/Shift"* —
  **still true** (`App.tsx` `handleKeyDown`).
- Completeness note: the inventory was re-derived from a full enumeration of
  every `onKeyDown`/`"keydown"` site in `src/` (11 non-test sites, 6 distinct
  handlers). No QWERTY pitch map exists in any of them — that absence is now
  established by enumeration rather than by an empty grep (F5).

## Findings table

> **Citations re-grounded 2026-08-22 on `fix/booked-defect-sweep` (base
> `ecb2fcd`)** (still-open rows only; the
> FIXED rows keep their historical coordinates). The `Where` column names
> **symbols, not line numbers**, on purpose: a repaired line number goes stale on
> the same clock as the one it replaced — "a correction that carries a line
> number inherits the defect it was correcting" (empyrean
> `docs/OVERSEER-PROTOCOL.md`). Each symbol below was verified to exist at that
> revision. Note the earlier prose sections still cite `file:line` and were NOT
> re-grounded; and `src-tauri/src/commands.rs` never existed — the module is
> `src-tauri/src/ipc/commands.rs`.
>
> **Second pass 2026-08-22, `docs/reground-feel-audit` (base `3d72793`):** every
> row below re-checked against the code, not against this document. The symbol
> discipline is preserved — **no line numbers were added, and every symbol named
> in a still-open row was confirmed to exist at `3d72793`** rather than inherited
> from the first pass. `Status` says what is true today; `Sev` carries the
> proposed severity with the old one struck where it moved. Every row is marked
> **[V]** verified firsthand this pass or **[C]** carried forward unverified —
> there are no unmarked rows.

| ID | Finding | Class | Sev | Status (2026-08-22 pass 2) | Where | G-ref |
|----|---------|-------|-----|---------------------------|-------|-------|
| F1 | Note-level edits inaudible while transport runs (PianoRoll never reloads sequence) | missing | ~~critical~~ — | **[V] FIXED** — `54c6082` (`bac49a6`). `PianoRoll.tsx` now reloads on every mutating commit; all 13 mutation paths enumerated (add, paint, erase, resize/move via `handleGestureEnd`'s `gestureMutatedRef`, velocity, transpose, nudge, delete, cut, paste, duplicate, voice-drop) | historical | G30 |
| F2 | Silent-track trap: instrument-less lanes drop notes with zero feedback; audition no-ops | feedback + discoverability | ~~critical~~ **med** | **[V] PARTLY FIXED** — `54c6082` (`be1d0c9`, `6d1382f`). The *feedback* half landed: `TrackHeader`'s `noVoice` badge + `nameUnbound` name style, `PianoRoll`'s `silentNotice` header badge, both with tooltips naming the two binding paths. **Surviving half:** no default voice and no one-click assign at region creation — you must still drag from the Library or find `+ FM Patch`; and `PianoRoll.handleAudition` still returns silently at the moment of the click | src/components/PianoRoll.tsx `handleAudition` (early return on `!track?.instrumentId`); src/components/ArrangementView.tsx `addFm` / `addPsg` | adjacent to booked "stale lane name" wart |
| F3 | FM/PSG param edits inaudible during playback (snapshot embeds patches; editors don't reload) | missing | ~~high~~ — | **[V] FIXED** — `7f59c18`. Both `FmEditor` and `PsgEditor` call `scheduleReloadSequence`; `PsgEditor` had the same hole the audit missed | historical | — |
| F4 | All previews/auditions hardcode ch0 → auditioning fights the playing mix | clunky | **high** | **[V] STILL OPEN, and now hit more often** — `do_preview_psg` still opens with `let channel: u8 = 0`, `stop_fm_preview` still sends `FmKeyOff { channel: 0 }`, `AudioEngine` still initialises `psg_preview_channel: 0`. F6's Draw Mode auditions on **each new pitch of a paint run**, so one drag now steals ch0 from the running mix repeatedly | src-tauri/src/ipc/commands.rs `preview_fm_instrument`, `do_preview_fm`, `do_preview_psg`, `stop_fm_preview` (all pin ch 0); src-tauri/src/audio/engine.rs `AudioEngine::process_command` (`StopPreview` arm's `invalidate_fm_cache(0)` recovery), `psg_preview_channel` field | — |
| F5 | No QWERTY note input / step record anywhere | missing | **high** | **[V] STILL OPEN** — established by full enumeration of every `onKeyDown`/`"keydown"` site in `src/` (11 non-test sites), not by an empty grep. None maps a key to a pitch | src/widgets/PianoKeys.tsx `PianoKeys` / `handleKey` (takes `React.MouseEvent` — mouse-only); src/App.tsx `handleKeyDown`, src/components/PianoRoll.tsx `handleKeyDown`, src/components/ArrangementView.tsx `handleKeyDown` (the only global handlers; transport/edit verbs only) | G36 (booked, owner call open) |
| F6 | Double-click-per-note placement; no pencil/paint; right-click inert (no erase/menu) | clunky | ~~high~~ — | **[V] FIXED** — `8359f75` (`9f2dc66`). `drawMode` state + `B` key + header `Draw` button; `PianoRollCanvas.paintCellUnderPointer` paints a run (Shift = pitch lock) committed as one gesture/undo step/reload; `handleMouseDown`'s `e.button === 2` branch erases in both modes. `handleContextMenu` is still `preventDefault`-only — that is now browser-menu suppression, not an absent verb | historical | G13/G14 |
| F7 | From-scratch DAC kits impossible on one track; drum-name rows all play one sample | missing + feedback | ~~high~~ ~~med~~ **low** | **[V] HEADLINE FIXED (`ea1adcf`), LABEL HALF OPEN.** The first half — from-scratch DAC kits — is done: `abb22a9` shipped per-note voices end to end (optional `instrumentId` on `addNote`, `set_note_instrument` IPC, `build_snapshot` resolving `note.instrument_id` through `resolve_instrument_data_by_id`'s DAC arm, per-voice colors), and F25's Sample picker made that reachable on a DAC lane, so a kick/snare/hat kit is now authorable on ONE track. **Surviving, and it is only the labels:** DAC pitch still selects nothing (`process_event`'s `ChannelType::Dac` arm plays the resolved instrument and ignores pitch), so the 29 `DAC_SAMPLE_NAMES` rows are a convention, not a mapping. They are TRUE for imported S3 content — `smps_mapper` writes `pitch = 36 + (sample_byte - 0x81)` next to a per-note `instrument_id` — and arbitrary for from-scratch content, which is why they were not deleted. `ea1adcf` documents that at the constant; making the rows real would be per-pitch mapping (a new model concept), deliberately not built. **Note the hardware ceiling either way: the DAC is ONE channel, so two drums cannot sound at the same tick, and the voice-overlap gate rejects such an edit — correctly** | src-tauri/src/sequencer/mod.rs `Sequencer::process_event` (`ChannelType::Dac` arm — pitch unused); src/components/PianoRoll.tsx `DAC_SAMPLE_NAMES` (now carries the derivation) | — |
| F8 | Piano-roll PSG audition has no stop path (looped envelope may ring); FM audition fixed 500 ms | feedback | med | **[V] STILL OPEN, verbatim** — `handleAudition`'s FM branch arms a 500 ms `stopFmPreview` timer; its PSG branch calls `previewPsgInstrument` and nothing stops it. `stopPreview` exists but is called from nowhere in `src/`; the engine re-seeds `psg_preview_index` from `psg_preview_loop` forever when a loop point is set | src/components/PianoRoll.tsx `handleAudition`; src-tauri/src/audio/engine.rs `AudioEngine::process_command` (`PsgEnvelopePreview` arm) + `AudioEngine::render` (preview-envelope stepping, `psg_preview_loop` re-seed) | — |
| F9 | No audible pitch feedback on transpose or drag-across-pitches | feedback | med | **[V] STILL OPEN, narrowed** — Draw-Mode paint *does* audition on each new pitch (`paintCellUnderPointer`), so the gesture added since the audit got this right. Keyboard transpose and move-drag are still silent | src/components/PianoRoll.tsx `handleKeyDown` (transpose/nudge branches — no `handleAudition` call); src/components/PianoRollCanvas.tsx `moveDrag` effect's `handleMouseMove` | — |
| F10 | No section markers / region names / colors — song assembly by bar-number memory | missing | **high** | **[V] STILL OPEN, verbatim** — `Region` gained `instrument_id` since the audit but still has no name and no color | src-tauri/src/model/song.rs `Region`; src/components/TimelineRuler.tsx `TimelineRuler`'s `draw` | — |
| F11 | No h-scrollbar/minimap/zoom-to-fit; arrangement zoom not cursor-anchored | clunky | med | **[V] STILL OPEN, narrowed** — `.body` is still `overflow-x: hidden` and `handleWheel` still rescales `ticksPerPixel` without touching `scrollLeft`. But the anchoring seam now exists and its sibling uses it: `zoomAtBy` (ruler vertical-drag) calls `zoomAroundPixel`. The wheel path is a small delta from correct | src/components/ArrangementView.module.css `.body` (`overflow-x: hidden`); src/hooks/useArrangementZoom.ts `useArrangementZoom`'s `handleWheel` (vs. its own `zoomAtBy`, which does anchor) | G31/G32 (booked) |
| F12 | No song end: playback runs forever past last note; **export duration is hardcoded, not user-supplied** | missing (design Q) | med | **[V] STILL OPEN; the audit's own text was wrong** — `Sequencer::advance` still has only loop bounds and no end condition. But WAV export takes **no** duration input: `handleExportWav` calls `ipc.exportWav(path, 60)` with a literal. "Needs manual duration" overstated the user's control; there is none | src-tauri/src/sequencer/mod.rs `Sequencer::advance` (loop bounds only); src/components/TopBar.tsx `handleExportWav` (literal `60`) | booked design Q — noted only |
| F13 | Volume-slider ride = reload (silence_all+reprogram) per input event during playback | feedback | ~~high~~ — | **[V] FIXED** — `7f59c18`. `handleVolume` still commits per event (deliberate) but reloads through `scheduleReloadSequence`; `reload_snapshot` diffs instead of calling `silence_all` | historical | — |
| F14 | No master meter/clip latch; meters dead when stopped; master vol resets per launch | missing | med | **[V] STILL OPEN, verbatim** — `levelColor` has three thresholds and no peak/clip latch, the polling effect returns early and clears when `!playing`, `masterVol` is `useState(100)` component state | src/components/TrackHeader.tsx `levelColor`; src/components/ArrangementView.tsx channel-level polling effect (early-returns and clears when `!playing`); src/components/TopBar.tsx `masterVol` | — |
| F15 | Zero view-state persistence: reopen loses roll/zoom/scroll/loop/snap/panel/filters | missing | **critical** (owner-DEPRIORITIZED) | **[V] STILL OPEN; `Where` was wrong** — `ProjectFile` is still `{ metadata, tracks }` with no view fields, so every listed item is still lost. But **"no localStorage" is false**: `src/utils/recentLocations.ts` (key `seraph.recentProjectLocations.v1`) is the app's first and only `localStorage` user, and the queue has designated it as the seam F15 should extend rather than a counter-example. Owner ruled this deprioritized (queue 2026-08-22: "current behavior matches how they work") — severity is unchanged, fundability is not | src-tauri/src/project/manager.rs `ProjectManager::open` / `ProjectManager::save`; src-tauri/src/model/song.rs `ProjectFile`; src/utils/recentLocations.ts (the designated seam — **not** an absence) | — |
| F16 | Cold-start ceremony: no default location/scratch project, no recent-**projects** list | clunky | ~~med~~ **low** | **[V] PARTLY FIXED** — `049721b` (`87bc14f`). Dead: "forced Browse" (the location input is editable and prefilled from `mostRecentLocation()`) and "no recents" (a suggestion listbox, plus `defaultPath` on both `handleBrowse` and `handleOpenProject`). **Surviving:** no scratch/default project, no recent-*projects* list to open (only a recent-*locations* prefill), welcome screen still two bare buttons, and still no Enter-to-create / Esc-to-cancel | src/components/MainArea.tsx welcome branch; src/App.tsx `handleOpenProject` (`defaultPath` only, no project list) | — |
| F17 | Core gestures invisible: lane dbl-click, grid dbl-click, header drag-drop, auto-bind | discoverability | ~~high~~ **med** | **[V] PARTLY FIXED** — the "invisible ruler halves" sub-claim is dead: `TimelineRuler` now sets a per-zone cursor from `HOVER_CURSOR[hoverZone]` (`fba0e3b`). The F2 badges' tooltips now name both voice-binding paths on the lane where it matters, and the `Draw` button's tooltip advertises `B`/paint/right-click-erase. **Surviving:** lane double-click-to-create-region, grid double-click, header drag-drop and `+ FM Patch`'s auto-bind are still unadvertised until you already know | src/components/TimelineCanvas.tsx `handleDoubleClick`; src-tauri/src/project/manager.rs `ProjectManager::bind_to_empty_lane`, `ProjectManager::assign_library_instrument_to_track` | G39 (keymap panel, booked) |
| F18 | One region at a time: no ghost notes, no multi-region editing across channels | missing | **high** | **[V] STILL OPEN, verbatim** — `App` passes `selectedRegions[selectedRegions.length - 1]` into `BottomPanel`; `PianoRoll` renders only `loaded.notes` for the one open region. Note selection is now *deliberately* cleared on region switch (`e01f6d1`), which makes the open/close cycling the audit describes strictly more costly | src/App.tsx `selectedRegions` (last-wins into `BottomPanel`'s `selectedRegion`); src/components/PianoRoll.tsx `loaded` / `notes` (one region) | — |
| F19 | Can't click-empty to deselect regions; roll closes only via tiny x; no Esc **on these surfaces** | clunky | low | **[V] STILL OPEN; phrasing corrected** — `handleClick` acts only on a hit and `handleMouseUp` calls `onSelectRegions` only when `hits.length > 0`, so neither an empty click nor an empty marquee clears. `PianoRoll.tsx` genuinely contains no Escape handling. What is wrong is the *global* reading of "no Esc" — see F20 | src/components/TimelineCanvas.tsx `handleMouseUp` (marquee), `handleClick` (hit-only); src/components/PianoRoll.tsx `closeBtn` button (no Escape handling in this file) | — |
| F20 | Mouse-only surface: no seek/zoom/track-focus/M-S/dialog keys; **no Esc outside inline edit fields**; `l` case-sensitive | missing | **high** | **[V] STILL OPEN; premise corrected** — "no Esc anywhere" is **false** and was false when written: `TrackHeader.handleRenameKeyDown`, `TopBar.handleMetaKeyDown` and `LibraryPanel`'s tag-edit `onKeyDown` all handle `"Escape"`, and this document's own inventory already listed them. The load-bearing claim survives and was **re-verified with a control**: zero `onKeyDown`/`keydown` in all four dialogs, while the same grep shape finds `onClick` in all four. `l` is still `e.key === "l"` (dead under CapsLock/Shift). Inventory gained one key since the audit: `B` = Draw Mode | src/App.tsx `handleKeyDown` (`e.key === "l"`); src/components/NewProjectDialog.tsx, AddTrackDialog.tsx, ImportDialog.tsx, LibraryRootsDialog.tsx (no key handler in any); inventory above | G39/G40 (booked) |
| F21 | Velocity lane: click-only, first-hit-by-x (chords un-editable), no paint/numeric/audition | clunky | med | **[V] STILL OPEN, verbatim** — `handleMouseDown` loops notes in array order and `return`s on the first x-overlap, ignoring pitch and selection; one `onVelocityChange` per click, no drag, no audition | src/components/VelocityLane.tsx `VelocityLane`'s `handleMouseDown` | G17 (booked) |
| F22 | Single-note readout is header text only; no inspector for exact tick/len/vel | missing | low | **[V] STILL OPEN** — `selInfo` still renders a header string (it gained detune/mod flags but no tick and no editability) | src/components/PianoRoll.tsx `selInfo` | S4 NoteInspector (planned) |
| F23 | 1 s polling for dirty state + track list — laggy dirty dot / cross-view refresh | feedback | low | **[V] STILL OPEN, verbatim** — both 1 s `setInterval`s present (App's undo-state poll, ArrangementView's `refresh`) | src/App.tsx `refreshUndoState` poll effect; src/components/ArrangementView.tsx `refresh` + its 1 s interval effect | — |
| F24 | Failures console-only: paste overflow skips, library assign errors, save-to-library errors | feedback | med | **[V] STILL OPEN, narrowed** — all three cited sites are unchanged (`console.warn` on paste overflow, `console.error` in `TrackHeader.handleDrop` and the FmEditor save-to-library `onClick`). But the premise "the app has no in-app notice channel" is now false: `PianoRoll`'s `showVoiceHint` is a working non-modal notice used for voice-drop hints and backend *rejections* — the fix is to route the remaining sites into it, not to invent a system | src/components/PianoRoll.tsx `handleKeyDown` (paste `console.warn`) vs. its own `showVoiceHint`; src/components/TrackHeader.tsx `handleDrop`; src/components/FmEditor.tsx save-to-library button `onClick` | — |
| **F25** | **NEW — per-note voice assignment is unreachable for DAC, the one chip it was needed for** | missing | ~~med~~ — | **[V] NEW THIS PASS, not previously booked.** `PianoRoll.handleVoiceDrop` rejects any drop whose payload `kind !== region.channelType`, and the library can never produce a `"dac"` entry: `LibraryInstrument` has exactly two variants (`Fm`, `Psg`) and `grep -rn "Dac" src-tauri/src/library/` returns nothing (exit 1, checked in isolation). So on a DAC region the only per-note-voice gesture in the app always fails with "Only DAC voices can be dropped on this lane". The IPC (`set_note_instrument`) and the resolution path (`resolve_instrument_data_by_id`'s `ChannelAssignment::Dac` arm) both support it; nothing in the UI can call them for DAC. Imported songs can still carry per-note DAC ids, so the read path is live — only authoring is unreachable. **This is what still blocks F7's headline scenario.** **[V] FIXED `ea1adcf`** — via a per-note **Sample picker** in the piano-roll header (DAC lanes only), fed by the PROJECT's DAC bank (`list_dac_instruments`) and applying to the note selection through the existing `set_note_instrument`. ZERO backend change. The library was deliberately NOT given a `Dac` variant: `DacInstrument` is a pointer to a `pcm_file`, not a self-contained parameter struct like `FmInstrument`/`PsgInstrument`, so a library DAC kind means designing PCM asset storage, hashing and extraction — a much larger parcel, and not what F25 was about. The drop message on a DAC lane now points at the picker rather than at a library kind that cannot exist | src/components/PianoRoll.tsx `handleVoiceDrop` (kind gate); src/api/library.ts `LIBRARY_DRAG_TYPE` (kind-suffixed types); src-tauri/src/library/entry.rs `LibraryInstrument` (`Fm`/`Psg` only — unchanged); src-tauri/src/library/store.rs `kind` mapping | — |
| **F26** | **NEW — every audition costs a full `listTracks` round-trip, on the interactive path** | feedback | ~~low~~ — | **[V] NEW THIS PASS, not previously booked.** `PianoRoll.handleAudition` starts with `await ipc.listTracks()` — the whole track/region/note tree serialized across IPC — before it can send a preview. It runs on note press, on grid double-click, on every keys-column click, and (since F6) **once per new pitch in a Draw-Mode paint run**, so a fast painted run issues one full song fetch per row it crosses. The pitch feedback is delayed by that round-trip by construction. Not measured; code-certain. **[V] FIXED `b7eb13f`** — new narrow IPC `get_track_instrument(track_id) -> Option<String>` (backed by `ProjectManager::track_instrument_id`) replaces the leading `listTracks`. Deliberately NOT a frontend cache: the binding is rewritten by `TrackHeader`'s library drop / unbind / track delete and by `library_assign_to_track`, none of which notify the roll (`SONG_REVERTED_EVENT` is the only cross-component signal, and undo/redo alone dispatches it), so any cache would audition the previous voice. MEASURED by mock call count in `PianoRoll.auditionCost.test.tsx`: five keys-column auditions and a three-row paint run now add ZERO `listTracks` calls. The second round trip (the preview itself) remains — collapsing both into one `preview_track_note` was considered and rejected as audio-path surface that cannot be verified without ears | src/components/PianoRoll.tsx `handleAudition`; called from `PianoRollCanvas` `handleMouseDown`, `handleDoubleClick`, `paintCellUnderPointer` and `PianoRollKeys` `onAudition` — all four funnel through the one function | — |
| **F27** | **NEW — the preview lets an FM6 track and a DAC track sound together; real hardware cannot** | fidelity (preview-vs-driver divergence) | **high** | **[V] NEW, found during the README accuracy pass (`f5eb86f`), verified firsthand by the overseer WITH A CONTROL.** On a Mega Drive the DAC steals FM channel 6 — writing `$2B` bit 7 swaps FM6's output for the 8-bit DAC stream, so the two are mutually exclusive by construction. Seraph's preview models neither half: `AudioEngine` keeps `dac_samples`/`dac_position` as an **independent** stream summed into the mix alongside `fm_l`/`fm_r`, and register `$2B` is **never written** anywhere in `audio/`, `sequencer/` or `dac/`. Evidence: `grep -rniE "0x2b\|\$2b"` over all three trees exits 1, with a control grep for `0x28` returning 9 hits in `engine.rs` — so the empty result is evidence, not a broken invocation. **Consequence:** a song that sounds correct in Seraph can be silently wrong on hardware and in any driver export, because FM6 and DAC content that overlaps in time is unreproducible. This is the SAME CLASS as the overlap fidelity bug already fixed (`ee11da5`, last-note-priority): the preview must not promise what the driver cannot deliver. **Not yet sized.** The honest fix is presumably to mute FM6 while DAC content is sounding and reflect it in the UI, but that is a design call (silent steal? a visible warning? an authoring-time gate like `check_voice_overlap`?) and the driver's exact behaviour should be confirmed against aeon rather than assumed. **Related but distinct from F7/F25**, which are about *which* sample a DAC note plays; this is about DAC and FM6 coexisting at all | src-tauri/src/audio/engine.rs `AudioEngine` (`dac_samples`/`dac_position` fields; the mix summation in `AudioEngine::render`); absence of any `$2B` write across `src-tauri/src/{audio,sequencer,dac}` | — |
| **F28** | **NEW — `PianoRoll.tsx` holds a NUL byte, so every grep silently skips the note-editing surface** | methodology hazard | **high** | **[V] NEW 2026-08-23, verified firsthand by the overseer.** `src/components/PianoRoll.tsx` contains one NUL byte, in `const MIXED_VOICE = "\0mixed"`. GNU grep therefore classifies the file as **binary**: `grep -c MIXED_VOICE src/components/PianoRoll.tsx` exits **1 with no usable output**, while `grep -ac` on the same file returns **7**. Enumerated across the whole tree: 18 tracked files contain NULs and **17 are icons/PNGs — this is the ONLY source file**, and it is 907 lines and is the entire note-editing surface. **Consequence: every frontend search in this repo's history that omitted `-a` excluded the most-edited file in the app, and reported a clean empty result while doing so.** This is protocol bar 16(d) — "a failing command and an empty world produce the same output" — sitting permanently in the tree rather than arriving in one command. Fix is one character (`\u0000` escape or a non-NUL sentinel); the audit value is re-running any past enumeration that touched `src/` | src/components/PianoRoll.tsx `MIXED_VOICE` | — |
| **F29** | **NEW — VGM export silently discards every DAC note** | fidelity (export) | **high** | **[V] NEW 2026-08-23, verified firsthand by the overseer.** `src-tauri/src/export/vgm.rs` has `ChannelAssignment::Dac(_) => continue` — there is no DAC in the VGM exporter at all, so every drum in the song is dropped with no error, no warning and no log line. **Sequencing matters: this must land BEFORE the booked README-7 fix that wires up the dead `export_vgm` UI path**, or that fix ships a working button whose first output is missing all percussion. Note for any follow-up grep: `out[0x2B]` in this file is a **VGM header offset, not a YM register** — it is not evidence of `$2B` handling | src-tauri/src/export/vgm.rs (the `ChannelAssignment::Dac(_)` arm) | — |
| **F30** | **NEW — SMPS export emits both an FM and a DAC header for channel 6, with no cross-channel validation** | fidelity (export) | med | **[C] NEW 2026-08-23, from the exposure map; the `Dac(_)` arms were confirmed present by the overseer, the both-headers-for-index-5 behaviour is carried from the agent's read and NOT independently re-derived.** `smps.rs` emits `smpsHeaderDAC` and `smpsHeaderFM` for index 5 and never checks whether the two conflict, so an FM6+DAC song exports a header pair the hardware cannot honour. Distinct from F29: SMPS over-promises where VGM under-delivers | src-tauri/src/export/smps.rs (`ChannelAssignment::Dac(_)` arms; header emission) | — |
| **F31** | **NEW — the only driver profile advertises seven voices on a six-voice chip** | fidelity (capability model) | **high** | **[V] NEW 2026-08-23, verified firsthand by the overseer.** `FlamedriverProfile::channel_layout()` returns six `FmChannelInfo` entries — including `{ index: 5, name: "FM6/DAC" }` — **and** a separate `DacChannelInfo { index: 0, name: "DAC" }`. The name is honest about the sharing; the **structure** is not, since the two are offered as independent lanes, which is where every downstream surface gets its channel roster. Worse: this profile is **S3K's Flamedriver, which has no FM6 music voice at all** (aeon's init table reads `db 80h, 6 ; FM6 music track (does not exist in this driver)` at aeon `origin/master` `139995f`), and **there is no aeon/Memra profile in the tree** — `FlamedriverProfile` is the only `DriverProfile`. **This is upstream of F27**: the roster over-promises before any note is authored, and it is wrong however F27 resolves | src-tauri/src/driver/flamedriver.rs `FlamedriverProfile::channel_layout`; src-tauri/src/driver/mod.rs (sole profile export) | — |

### F27 — GROUNDED 2026-08-23, AND THE ROW ABOVE STATES THE PREMISE WRONG

**RULED BY THE OWNER 2026-08-24 — THE DRIVER DECIDES. Design closed; the row and the
grounding below are both left unedited.** Decision `d-7` in `docs/decisions.jsonl`
(chain `d-1` → `d-6` → `d-7`). **The driver profile carries the channel-6 behaviour and
Seraph follows it. There is no per-song channel-6 mode**, and a song cannot claim a
behaviour its driver does not have.

The option he chose was **his own**, proposed as a question and added under
`DECISIONS.md` rule 8b rather than mapped onto the nearest one already offered. His
reasoning: if a given driver always eats a voice for drums, a song on that driver should
behave that way. The driver read in §2 of `docs/research/2026-08-23-f27-driver-truth.md`
supports it across **all five drivers whose bytes were read** — S3K has no FM6 music
track at all; Batman excludes the voice per sub-frame; Alien Storm, Gunstar and TF4
toggle `$2B` per sample; §2.3 records that **nobody lets FM6 and the DAC sound
simultaneously and no driver read represents that state**. *Caveat kept: Sonic 1, his own
example, was NOT among the five.*

**What stays open, and is deliberately not being re-asked now:** aeon's Memra is the only
driver found that genuinely offers a per-song choice (DEDICATE / FM6-FM / ADAPTIVE), so
one Memra-scoped setting is still wanted. It gets its own card when Memra playback is
actually built, per his stated preference for deciding things he can hear.

**Sequencing consequence: F31 is promoted from cleanup to step one.** Under this ruling
the profile is the thing that carries the answer, so it must be right before any surface
reads it. F27's remaining work is implementation, unblocked, and sits behind F31.


Two investigations landed (`75fdd1a`, `6852454`; reports at
`docs/research/2026-08-23-f27-driver-truth.md` and
`docs/research/2026-08-23-f27-exposure-map.md`). **The F27 row's central claim —
"real hardware cannot" — is false as written, and the row is left unedited above so
the correction is visible rather than laundered.**

**What the driver actually does** (read at aeon `origin/master` =
`139995f256f5e50c26d2053c229dd09b5e70c84d`, every read via `git show <rev>:<path>`
and never through the sibling path, since the aeon lane is live in that tree;
re-verified firsthand by the overseer):

aeon **does** write `$2B` — at four sites — and implements a deliberate **three-mode
per-song contract** for chip channel 6, selected by `SH_FLAGS`:

- **DEDICATE** — ch6 is the DAC. No FM6 music voice.
- **FM6-FM** (`SH_F_FM6_FM`) — ch6 is a sixth FM voice, DAC off.
- **ADAPTIVE** (`SH_F_FM6_ADAPTIVE`, requires `SH_F_FM6_FM`) — genuine time-share:
  key-off before each sample, EG-edge re-key when the sample drains.

So **hardware CAN sound FM6 and DAC in one song** — in ADAPTIVE, alternating, if the
song declares it. What it cannot do is sound them *simultaneously*, and what the
format does not do is stop you declaring the wrong mode.

**The trap that makes the obvious fix wrong.** `.stop`'s DAC-off-and-restore is gated
on `SND_FM6_ADAPTIVE`. In **FM6-FM** mode there is no restore, so the first drum takes
FM6 away **permanently** — and `Fm_NoteOn`'s suppression stops firing once
`SND_STAT_DAC_ACTIVE` clears, so the driver then keys FM6 on into a channel the chip
has muted. **A preview that ducks FM6 and restores it would therefore sound BETTER
than hardware**, which is precisely the failure F27 exists to prevent. Silent steal is
not a whole fix; it is a fix for one of three modes.

**And `$2B` is the wrong lever on the Seraph side anyway.** Nuked-OPN2 substitutes the
DAC in its time-multiplexed output stage, but `AudioEngine` reads through
`OPN2_ReadChannels`, which sums `ch_out[0..6]` and is **blind to `dacen`** (verified
firsthand in `src-tauri/vendor/nuked-opn2/ym3438.c`). Any estimate premised on "just
write `$2B`" is wrong: the fix is a new accessor in the vendored C, or a driver-level
key-off of FM6.

**The semantics already exist in this repo, on the import side only.**
`ImportState::process_key_on` in `src-tauri/src/import/vgm_import.rs` has
`if hw_ch == 5 && self.dac_enabled { return; }` — the one line in the tree coupling FM
channel 5 to DAC state. VGM-imported projects are self-consistent and **cannot exhibit
the bug**; only hand-authored (and possibly SMPS-imported) ones can. The model was
built and never reached playback or export.

**`check_voice_overlap` cannot host the gate.** It narrows to a single
`channel_key(&target.channel)` on its third line and iterates only tracks matching that
key (verified firsthand), so a cross-channel constraint is not a case inside it. Gating
this properly means gating 7–8 entry points — `update_note`, `move_region`,
`update_track` among them — none of which has ever had a gate.

**THE OPEN DESIGN CALL, and why it is the owner's.** The two investigations
**disagree**, and the disagreement is the finding: the exposure map recommends a
key-off-FM6 steal plus a diagnostic, while the driver read shows that is correct only
in ADAPTIVE and actively wrong in FM6-FM. Both are right within their own frame. The
reconciliation is that **Seraph has no song-level ch6-mode field at all**, so it cannot
currently express which of the three contracts a song is written against — and no
amount of source reading decides which mode a from-scratch Seraph song should default
to. That is a model-design question, PARKED for the owner with numbers.

*(Independence check, protocol bar 19: these two derivations enumerated over different
parameters — one over aeon's driver source and disassembled Z80 blobs, one over
Seraph's own call graph — and neither brief carried the other's conclusion. Their
agreement on the VGM-export defect is therefore corroboration rather than echo. Their
disagreement on the fix is real, not a frame artifact.)*

**TAGGED for foreground follow-up — neither agent could run the emulator, and did not
try.** (1) Pack a song with `flags=SH_F_FM6_FM`, an FM6 melody and one `$E2` drum, and
trace whether post-sample `$28` writes land on a chip-muted ch6 — the driver read calls
this inference from four code paths plus a source comment, **not** an observation.
(2) By ear in Seraph: play an FM6 sustain under a drum hit and confirm they audibly
coexist — a 30-second check that upgrades the central claim from read-verified to
heard-verified.

**Could not be established, recorded so it is not mistaken for closed:** whether real
SMPS songs actually produce FM6+DAC through `smps_mapper`'s sequential FM indexing; and
MDSDRV's single FM6/PCM1 slot — `aeon/docs/research/external/` is **empty** at that
SHA, so every `mdsdrv.68k:` citation in aeon's in-repo docs is currently unreproducible
and must be treated as second-hand.

## Top-10 biggest feel wins — ORIGINAL ranking (2026-08-21, superseded)

Kept verbatim so the re-rank below can be read as a diff, and so a cold session can
see which wins the last four parcels bought.

1. ~~**Hear what you just placed**~~ — **SHIPPED** `54c6082`. (F1)
2. ~~**Kill the silent-track trap**~~ — **half shipped** `54c6082` (cues yes, default
   voice / one-click assign no). (F2)
3. **Session restore** — (F15) *owner-deprioritized 2026-08-22, still open.*
4. ~~**Live patch tweaking**~~ — **SHIPPED** `7f59c18`. (F3)
5. ~~**One-gesture note entry + right-click erase**~~ — **SHIPPED** `8359f75`. (F6)
6. **QWERTY audition + step entry** — (F5/G36) *still open, owner call open.*
7. **Audition on a free channel** — (F4) *still open.*
8. **Region names/colors or ruler markers** — (F10) *still open.*
9. ~~**DAC kit authoring**~~ — **SHIPPED** `abb22a9` + `ea1adcf` (F7/F25).
   Only the row LABELS remain a convention rather than a mapping.
10. **Arrangement navigation** — (F11) *still open, narrowed.*

## Top-10 biggest feel wins — RE-RANKED 2026-08-22 (pass 2)

Four of the original ten are shipped and two are half-shipped, so the ranking is
re-derived rather than re-numbered. The ordering principle is unchanged
(centrality to a composing session), with one addition: **a finding whose blast
radius GREW because a neighbouring parcel landed outranks one that merely stayed
put.** Proposed, not ratified — the owner's deprioritization of F15 is honoured
in the fundability note rather than by moving its severity.

1. **Audition on a free channel** (F4) — *promoted from #7, and the recommended next
   parcel.* Two things changed under it. F1/F3 made "edit and tweak while the loop
   runs" the normal workflow this audit was aiming for, so an audition that
   corrupts the running mix is now hit constantly rather than occasionally. And
   F6's paint run auditions **per new pitch**, so a single drag across five rows
   now steals ch0 five times. It is also the last audible-severity finding never
   measured, and the harness that measured F3/F13 (`rendered_rms`,
   `live_edit_audibility`) can measure it without ears.
2. **Region names/colors or ruler markers** (F10) — unchanged at high, now the
   largest untouched capability gap. Song assembly is still bar-number memory, and
   nothing merged since the audit has touched it.
3. **QWERTY audition + step entry** (F5/G36) — unchanged. Still gated on an owner
   call; ranked below F10 only because F10 needs no ruling.
4. **Two regions at once / ghost notes** (F18) — held at high and arguably
   *worsened*: the region-switch fix (`e01f6d1`) now correctly clears note selection
   on every switch, so the open/close cycling the audit describes loses selection by
   design rather than by accident.
5. **Finish the silent-track trap** (F2) — the cue landed; the cure did not. A
   default voice, or one-click assign at region creation, is a small parcel now that
   the "no voice" state is already computed in two components.
6. ~~**Make DAC per-note voices reachable**~~ (F25, unblocking F7) — **SHIPPED
   `ea1adcf`.** Landed as a header Sample picker over the project's DAC bank
   rather than as a library `Dac` kind: `DacInstrument` points at a `pcm_file`
   instead of carrying its parameters, so a library kind means designing PCM
   asset storage — a different, much larger parcel. What remains of F7 is the
   row labels, not the capability.
7. **Route the remaining failures into the existing notice channel** (F24) — was
   "build a way to show errors"; is now "call `showVoiceHint`'s equivalent from three
   more sites". Cheap, and it removes a whole class of silent failure.
8. **Arrangement navigation** (F11) — narrowed to two concrete deltas: use the
   existing `zoomAroundPixel` from `handleWheel`, and give `.body` a horizontal
   scrollbar. The anchoring seam already exists and is already used by `zoomAtBy`.
9. **Session restore** (F15) — severity is still critical; **fundability is not.**
   Owner ruled it deprioritized ("current behavior matches how they work"). Left
   here so a cold session sees the severity and the ruling together. When it is
   funded, extend `src/utils/recentLocations.ts` rather than starting fresh.
10. **Velocity lane + note inspector** (F21/F22) — unchanged, and the natural
    companions to any S4 NoteInspector work.

Dropped off the list because they shipped: F1, F3, F6, F13, F25, F26.

## 15-minute owner play-test script — REWRITTEN 2026-08-22 (pass 2)

> The 2026-08-21 script told the owner to expect behaviour that four merged
> parcels had since fixed (steps 1, 2 and 5 in particular would have "confirmed"
> defects that no longer exist, and step 3's *Was:* framing had already been
> patched once). It is replaced, not amended. Two steps are now **regression
> checks on shipped fixes** rather than confirmations of open findings — those are
> the ones worth the owner's ears, because nothing else can confirm them.

Every step's expectation below is derived from the code at `3d72793`.

1. **(F1 regression, 2 min)** New project, drag a Library FM voice onto FM1,
   double-click the FM1 lane for a one-bar region, place a note, `L` to arm the
   loop, Space. While it loops: place a second note; drag the first to a new
   pitch; delete one. **Expected now: every one of those is audible on the very
   next pass, with no stop/start.** If anything needs a restart to be heard, F1
   has regressed — say so, it is the audit's #1 and it is supposed to be closed.
2. **(F6 regression + F4, 3 min)** Still looping. Press `B` (or click **Draw**),
   then drag across five or six rows to paint a run. **Expected: one gesture
   paints the whole run, one undo (Ctrl+Z) removes all of it, and it is audible
   on the next pass.** Now the part that matters: **listen to FM1's own part
   while you paint.** *Expected per code (F4, needs ears): every new pitch in the
   run fires a preview on hardware channel 0, so the loop's FM1 voice should drop
   out or glitch repeatedly during the drag and recover on its next note-on.* This
   is the single most valuable ear-minute in the script — F4 is the recommended
   next parcel and its audible cost has never been measured.
3. **(F6 owner ruling, 1 min)** Still in Draw Mode: left-click an **existing**
   note. **Current behaviour: it selects/moves the note. Ableton would delete
   it.** The implementer deviated deliberately and the deviation was ratified
   *provisionally* pending your ear. **Your call — one-line change either way.**
4. **(F2, 1 min)** On a lane with **no** voice: it should already read `no voice`
   next to a dimmed lane name, and opening its region should show a
   `silent — no voice assigned` badge in the roll header. **Expected: those cues
   are present** (that half shipped). Then place notes and play. *Expected: still
   total silence, and clicking the keys column still does nothing —* the cue
   explains the silence but nothing offers to fix it in one click. That gap is
   what F2 still funds.
5. **(F3 + F13 regression, 3 min)** Notes on two FM lanes plus one PSG lane
   sustaining an envelope voice. Play. (a) Open an FM instrument and drag
   TL/algorithm hard and fast. **Expected: the timbre moves under your hand inside
   the sustaining note — no re-attack, no gap, no stutter.** (b) Ride one track's
   volume slider slowly, then fast. **Expected: that lane's level follows the
   slider inside its note; the OTHER FM lane is completely undisturbed; and the
   PSG lane does not re-attack.** These were measured as rendered audio, never
   heard — a rendered rms of 0 was the original defect, so "it sounds fine" is the
   confirmation being sought.
6. **(F8, 1 min)** Give a PSG lane an envelope voice **with a loop point set**,
   then click one note in its piano roll, once, and let go. *Expected per code: it
   rings indefinitely — the FM audition self-stops after 500 ms, the PSG one has
   no stop path at all.*
7. **(F16, 1 min)** File → New Project. **Expected: the Location field is
   pre-filled and typeable, and focusing it drops a list of recent locations** —
   that much shipped. *Still expected to be missing: any way to press Enter to
   create or Esc to cancel, and any list of recent **projects** to reopen.*
8. **(F12 + F24, 1 min)** Play past the last region — *expected: it never stops,
   the UI stays "playing".* Then Ctrl+C two bars of notes and Ctrl+V one bar
   before the region end — *expected: the overflowing notes are silently missing,
   with the warning only in the devtools console. Note the contrast: a paste the
   backend **rejects** does show an in-app notice in the roll header — the channel
   exists, this path just doesn't use it.*
9. **(F15, 2 min — optional, owner-deprioritized)** Mid-work: zoomed in, roll
   open, loop armed, snap=Beat. Save, close, reopen. *Expected: all of it gone.*
   Skip unless you want to re-examine the deprioritization ruling.

## BLOCKED / UNVERIFIED for controller follow-up

- **Nothing blocked.** All scenarios were traceable in code.
- UNVERIFIED (audible/live confirmation needed, covered by the script above):
  ~~F1~~ (fixed `54c6082`; regression check is script step 1),
  ~~F3~~, **F4 (ch0 steal audibility — still unmeasured, and now the top-ranked
  parcel; script step 2)**, **F8 (endless PSG audition — still unmeasured;
  script step 6)**, ~~F13~~, and
  ~~the exact audible cost of `silence_all` on every reload~~.
- **TAGGED for the controller's foreground follow-up (never attempted from a
  background lane):** F4 and F8 both need either ears or a rendered-audio run.
  Neither was attempted this pass. F4 is measurable *without* ears using the
  existing harness (`src-tauri/src/audio/rendered_rms.rs`,
  `src-tauri/src/audio/live_edit_audibility.rs`) — render a two-lane snapshot,
  fire a preview mid-note, and assert the non-previewed lane's rms is undisturbed.
  That is the same shape that corrected F13's severity, and it should precede the
  F4 parcel rather than follow it.
- **F6's Draw-Mode click-on-existing-note deviation** (selects/moves where Ableton
  deletes) remains provisionally ratified pending the owner's ear — script step 3.
  Not a re-grounding finding; carried forward from the queue so it is not lost.

## Re-grounding pass 2 — scope, method, and what was NOT checked

Recorded so the next pass does not have to re-derive it, and so the completeness
claim can be audited rather than taken on trust.

**Enumeration actually run.** The row set is F1–F24 as written plus the two rows
added this pass (F25, F26) = **26 rows, every one marked [V]** — for each, a
symbol or a behaviour was read in the tree at `3d72793` this pass. Zero rows are
**[C]**. That is a strong claim, so here is exactly what it does and does not
cover: **[V] means the cited symbol was read and the stated behaviour follows
from the code as read.** It does **not** mean the behaviour was observed running
— nothing was executed, no test suite was run, and no audio was rendered. The
four already-FIXED rows (F1, F3, F6, F13) keep their pre-fix `Where` coordinates
deliberately; their **[V]** attaches to the fix being present, which was checked
in the fixing files, not to those historical coordinates. The Top-10 was
re-derived from this set rather than renumbered.

**Prior-pass warning, honoured rather than repeated.** The first re-grounding
pass on this document asserted completeness three times and was wrong each time;
its third miss (F15's `Where`) was found afterwards by someone else. So: this
pass makes no claim about findings that are *absent* from the table. Two new ones
(F25, F26) surfaced incidentally while checking F7 and F6 — which is itself
evidence that the table is a record of what has been looked at, not of what is
wrong with the app.

**Absences confirmed with controls, not with empty greps** (the F19/F20 defect was
an uncontrolled absence claim, so every absence below has a paired positive):

- *No key handler in any dialog* — `grep onKeyDown|keydown` over the four dialog
  files returns nothing; the same shape over the same four files returns `onClick`
  in all four.
- *No `Dac` anywhere under `src-tauri/src/library/`* — grep exit status checked in
  isolation (1, no match), with `LibraryInstrument`'s two variants read directly
  from `entry.rs` as the positive control.
- *No QWERTY pitch map* — established by enumerating all 11 `onKeyDown`/`keydown`
  sites in `src/` and reading each, not by searching for a name it might not use.
- *Nothing imports `Sidebar`* — re-run unfiltered after an earlier filtered grep
  could have hidden the answer inside its own `-v` pattern.

**Deliberately NOT re-grounded, and why:**

- The `file:line` coordinates inside Scenarios A–G and the keyboard pass. Their
  *claims* were re-checked and the false ones are enumerated above; converting
  ~60 historical coordinates to symbols would rewrite the audit's narrative record
  for no funding benefit. If a future parcel needs one of those addresses, take it
  from the findings table's `Where`, which is symbol-grounded, never from the prose.
- The G1–G41 cross-references. The full enumeration still is not present verbatim
  in the queue doc (only ~25 G-numbers are named in its Log), so the "no G-ref"
  rows (F7, F10, F18, F25, F26) may still overlap unnamed G-numbers. Unchanged
  from the 2026-08-21 note; not resolvable from this repo's docs alone.
- Anything requiring runtime or audio. No emulator or audio tool was invoked; see
  the TAGGED items above.

### F3 / F13 / `silence_all` — VERIFIED then FIXED (2026-08-22)

Reproduced as rendered audio (`src-tauri/src/audio/live_edit_audibility.rs`,
harness `rendered_rms::render_snapshot_with_edits`), NOT by ear and NOT by
register inspection. The measurement corrected the audit on two points:

- **All three findings were one defect**, `Sequencer::reload_snapshot` calling
  `silence_all`.
- **It was worse than "stutter" or "inaudible".** Every mid-note
  `ReloadSequence` rendered **rms 0.00000** for the remainder of the note —
  `silence_all` key-offs every sounding channel *and* blanket-writes
  attenuation `0x0F` to all four PSG channels. So a volume ride did not zipper,
  it muted the mix for the length of the drag; a knob turn (once wired) would
  have done the same. F13's severity was understated, not overstated.

Fixed by making `reload_snapshot` diff instead of silence: sounding notes are
re-identified in the new snapshot (by `ChannelType`, not index) and carried
across with a live reprogram; only genuine orphans get a targeted key-off.

Two corrections to the audit's text while here:

- **PsgEditor had the same hole as FmEditor** (the audit only inspected
  `FmEditor.tsx`) — `update` called `updatePsgInstrument` and nothing else.
  Both now reload.
- **F13's "undo-grouped (nice)" still holds**, and the per-event `updateTrack`
  is deliberately kept — every input event must commit its value. Only the
  *reload* coalesces now (`src/utils/liveReload.ts`).

Deliberately still out of scope, recorded so nobody re-derives them:

- A **retuned** note (pitch changed while sounding) is not re-articulated — it
  is keyed off and waits for the next note-on, as before. Re-articulating on
  every reload would re-attack during a drag.
- **PSG noise-mode** edits apply from the next note-on only: re-writing the
  noise register resets the SN76489 LFSR, which is an audible re-attack.
- **SSG-EG-only** edits do not trigger a live reprogram — `last_fm_patch`
  caches the 25 packed bytes, which exclude SSG-EG. Pre-existing, and no UI
  currently exposes SSG-EG.
- **DAC**: a streaming sample has no per-note register state to refresh.
- The full G1–G41 enumeration is not present verbatim in the queue doc (only ~25
  G-numbers are named in the Log); cross-refs here cover the named ones. If the full
  list lives elsewhere, a few "no G-ref" rows above (F7, F10, F18) may overlap it.
