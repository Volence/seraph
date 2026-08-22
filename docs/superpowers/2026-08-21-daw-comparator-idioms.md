# DAW Comparator Idioms — Furnace / Deflemask / FL Studio / Ableton vs Seraph

**Date:** 2026-08-21 · **Branch:** `audit/comparator-idioms` · **Type:** comparative UX research (no code changes)

**Question:** Seraph's likely users bring muscle memory from Furnace and Deflemask (the
dominant YM2612/PSG trackers) and from FL Studio / Ableton Live (piano-roll DAWs; seraph
already adopted Ableton's mouse grammar by owner ruling). Seraph is a piano-roll/timeline
DAW, not a tracker — which idioms from each world must it honor, and which should it
consciously replace?

**Sources actually consulted** (marked ✅ when a claim is grounded in them; unmarked
tool-behavior claims are general knowledge and flagged inline):

- ✅ **Furnace manual** (tildearrow.org/furnace/doc/latest/): `3-pattern/index.html`
  (note entry), `2-interface/keyboard.html` (default shortcuts),
  `2-interface/play-edit-controls.html`, `2-interface/order-list.html`,
  `4-instrument/index.html` (macros).
- ✅ **DefleMask Tracker Manual v2.0.0** (deflemask.com/manual.pdf, full text extracted):
  Pattern Matrix, Instrument Editor Window, Default Controls tables, Genesis chapter.
- ✅ **FL Studio online manual** (image-line.com): `pianoroll.htm`, `playlist.htm`; plus
  web-search summaries for typing-keyboard-to-piano (Ctrl+T) — secondary sources, marked.
- ✅ **Ableton Live 12 Reference Manual** (ableton.com/en/manual/): `editing-midi/`
  chapter; `arrangement-view` loop behavior via search-result summaries of the manual.
- **Seraph state:** read firsthand on this worktree (components, `src/utils/keyboard.ts`,
  `src-tauri/src/sequencer/mod.rs`, and the 2026-08-21 Log entries in
  `docs/superpowers/2026-07-03-seraph-banking-queue.md`).

**Caveats:** Deflemask claims are from the v2.0.0 manual (the current paid rewrite);
legacy 1.x differs in places. FL claims come from Image-Line's manual as summarized by a
fetch model plus secondary tutorials for Ctrl+T — exact FL key names for typing-piano
layout are secondary-sourced. Ableton Arrangement-loop keys (Ctrl+L) are from search
summaries of the official manual, not a direct chapter fetch. Nothing here was verified by
running any of the four applications.

---

## 1. Note entry rhythm

**Furnace** ✅ — QWERTY-as-piano: pressing a note key "insert[s] a note at the cursor's
location, then advance[s] to the next row (or otherwise according to the Edit Step)".
Volume/effect columns take raw hex; "the cursor will move by the Edit Step when a suitable
value is entered." `Space` toggles edit mode; octave on `Keypad *`/`Keypad /`; edit step
on `Ctrl-Keypad *`/`/`. Mouse is for cursor placement and drag-selection only.

**Deflemask** ✅ — same grammar: `Space` = "Recording mode", step size
`Ctrl+Add`/`Ctrl+Subtract`, octave on numpad `Divide`/`Multiply`, `Tab` = Note-Off,
hex values `0-F`, on-screen piano toggled with `Shift+P`. MIDI-in with optional poly
input (`Ctrl+P`) and MIDI velocity → note volume.

**FL Studio** ✅ — mouse-first with two entry tools: **Draw (P)** — one click adds one
note, drag repositions before release, right-click deletes (no tool switch to erase);
**Paint (B)** — click-and-drag paints a run of grid-snapped notes in one gesture; a drum
variant (N) paints and click-mutes. Plus **typing keyboard to piano** (`Ctrl+T` toggle;
Z–M lower octave, Q–P upper — secondary-sourced) feeding recording/step entry, and a
chord-stamp tool. Alt bypasses snap.

**Ableton** ✅ — Draw Mode on the `B` key: "click and drag inside the MIDI Note Editor to
add notes"; clicking existing notes deletes them; Pitch Lock constrains drawing to one
row, `Alt` enables freehand melodic drawing. Outside Draw Mode, double-click draws.

**What makes fast entry fast** — in all four, rhythm arithmetic is delegated to a
persistent step/snap setting so each note costs one atomic gesture: tracker = keypress +
auto-advance (hands never leave home row; the edit step *is* the rhythm); FL = paint-drag
amortizes N notes into one gesture and right-click-delete removes the most common
correction's tool switch; Ableton = draw-drag. The tracker version is fastest for
monophonic chip lines because pitch selection and confirmation are the same keystroke.

**Seraph today:** double-click draws (Ableton ruling, banked); no paint/brush drag; no
QWERTY step entry (G36, owner call open); no right-click delete yet (G13/G14 in backlog).

---

## 2. Audition feedback

**Furnace** ✅ — note keys always preview through the current instrument (edit mode off =
pure jamming); play/edit controls include a polyphony toggle ("simultaneous note playback
[vs] single-note-only") and a metronome. An on-screen piano/input pad exists
(`8-advanced/piano.html`).

**Deflemask** ✅ — on-screen piano for touch/mouse preview; the instrument editor is a
floating subwindow "very useful while you are editing a song and you want to check the
instrument at the same time" — i.e. audition-while-the-song-plays is a designed-for flow.
Wavetable editor has an explicit preview selector.

**FL Studio** ✅ — the **Play selected (Y)** tool clicks-to-play notes and drag-scrubs;
(general knowledge, not confirmed in the fetched excerpt: FL also previews notes on
place/drag by default via a "click sounds" hint setting).

**Ableton** ✅ — the **Preview** (headphone) switch: "hear notes as you add them or select
and move existing notes"; it is global "to all MIDI tracks in the Live Set" and, with the
track armed, doubles as the step-recording gate.

**Seraph today:** has audition on key-column click, note click, and note draw
(`handleAudition` in `src/components/PianoRoll.tsx` wired to `PianoRollKeys` and
`PianoRollCanvas`; FM preview auto-stops after 500 ms). Lacks: audition during
drag-move/arrow-transpose (the moment pitch feedback matters most), a global preview
toggle, and any audition-over-running-loop guarantee (preview IPC vs sequencer
interaction unaudited here).

---

## 3. Loop-centric composing

**Furnace** ✅ — no arbitrary loop range: the loop unit is the **pattern**. A dedicated
control "repeat[s the] current pattern from its beginning" instead of following the order
list. The deeper idiom: trackers edit **live during playback** — edit mode stays on while
the pattern cycles and changes are heard next pass; playback never stops for an edit.

**Deflemask** ✅ — `Alt+Return`/`F6` plays the pattern, `Shift+Return`/`F7` plays from
position, `Space` toggles recording while playing = loop-recording into the cycling
pattern. Hold the record button to arm **Clone Pattern On Write** (`Ctrl+D`): "any new
input on the pattern will automatically create a clone of the current pattern" — jam over
a loop without corrupting the shared pattern other orders reference.

**FL Studio** ✅ — PAT/SONG modes make "loop the current pattern" a one-key mental state;
in the Playlist a `Ctrl+click`-drag timeline selection "will play in loop mode"; (general
knowledge: loop-recording overdubs, accumulating notes each pass).

**Ableton** ✅ — `Ctrl+L` = Loop Selection (sets the Arrangement loop brace to the time
selection and toggles it); brace edges resize, brace body drags whole; the manual confirms
"it is possible to adjust the looping region during playback." Clip loop brace likewise.

**Seraph today:** has ruler-upper-half drag loop (bar-snapped bracket), zero-move click =
one-bar loop, `l` re-arms the last range, snap selector honored (Wave 2 parcel D). Lacks:
loop-from-selection (Ctrl+L equivalent), any recording (no record path at all), and a
verified *gapless* edit-while-looping guarantee (`reloadSequence` fires on edits; whether
the loop audibly hiccups is untested — tracker users assume editing mid-loop is free).

---

## 4. YM2612-specific authoring

**Patch editing during composition.** Furnace ✅: instrument editor is a normal window
(double-click instrument or Window menu), never modal, with per-FM-operator macro tabs;
macros come in sequence / ADSR / LFO forms, drawn with the mouse, with loop and release
points. Deflemask ✅: the instrument editor is a floating subwindow explicitly pitched for
tweak-while-the-song-plays; FM editor offers Sliders / Knobs Radial / Knobs Vertical,
`Ctrl+click` types exact values, and you can **mute individual operators while editing**
("the muting will not have any effect on the actual song playback") to isolate carriers vs
modulators. Both keep the editor one keypress away (`F1`/"Edit" in Deflemask).

**Voice changes mid-song.** Trackers: the pattern's **Ins column** re-programs the channel
per row ✅ (Deflemask: "This value will define the instrument that will trigger the
note"), so one channel plays bass, then brass, at zero UI cost. Deflemask additionally
exposes register-level runtime effects on Genesis ✅: `12xx–15xx` per-operator TL, `11xx`
feedback, `16xy` MULT, `19xx–1Dxx` AR, `10xy` LFO. DAWs bind one instrument per track;
mid-track voice changes need automation/program-change or a second track.

**DAC/sample channel.** Deflemask ✅: `17xx` DAC Enable flips FM6 into sample mode
mid-song; sample instruments are 8/16-bit WAVs in banks of 12 notes × 12 banks. Furnace
has a full sample editor chapter (not fetched in detail).

**Chip limits surfaced.** Both trackers are correct-by-construction by nature ✅: channel
roster is fixed by the chosen system (you cannot add an FM7), max volume is per-system
("Max Volume: 7F (Soundchip 1), F (Soundchip 2)" for Genesis), and each system carries
its own effect list window. Invalid entries are simply impossible to express.

**Seraph today:** FmEditor/PsgEditor/DacEditor live in a persistent BottomPanel — matches
the tracker always-at-hand idiom, not a modal dialog (good); channel roster is seeded from
`DriverProfile::channel_layout()` (compose-path ship, 2026-08-21) — the fixed-roster
correct-by-construction stance matches tracker expectations exactly. Lacks: per-region or
per-note patch changes on FM/PSG tracks (DAC notes carry `instrumentId`; melodic tracks
bind instrument at the track), operator mute-in-editor, and any macro/envelope layer
between "patch" and "per-note effect".

---

## 5. Velocity/dynamics on a chip with no velocity

**The chip fact:** YM2612 key-on has no velocity; loudness is carrier **TL** (0–$7F,
~0.75 dB/step, logarithmic). Everything below is UI over TL writes.

**Trackers** ✅ — dynamics live in the **volume column**, range surfaced honestly per
system (Deflemask Genesis: max `7F` on FM, `F` on PSG — the FM column *is* the TL range).
Articulation beyond that: volume macros (per-instrument envelopes, drawn in the macro
editor) and per-operator TL effects (`12xx–15xx`) when the composer wants timbre — not
just loudness — to respond. The split is clean: **column = per-note intent, macro =
per-instrument shape, effect = surgical override.**

**DAWs** ✅ — a velocity lane per note: Ableton velocity markers + `Alt+drag` on the note;
FL Alt+mouse-wheel over notes, `Shift+click`-drag to level a range, and a multi-note
ramp/slide in the event editor.

**What per-note velocity should MEAN in seraph:** carrier-TL attenuation only —
loudness, deterministic, exportable. Two design notes follow from the comparison:
(1) 0–127 velocity ≈ 0–$7F TL is a near-1:1 mapping; showing the *effective TL/hex*
somewhere (status bar or lane tooltip) speaks the language tracker users already think
in and keeps export honest. (2) If timbre-dynamics is ever wanted (velocity also opening
modulator TL, like real FM synth velocity sensitivity), it must be an explicit per-patch
opt-in ("velocity → mod depth %"), never a silent default — trackers train users that
volume never changes timbre unless they asked via a TL effect.

**Seraph today:** per-note velocity 0–127 with a VelocityLane (single-bar drag; G17
multi-note paint still open, G16-adjacent); backend maps
`(127-volume)+(127-velocity)` → capped TL offset (`src-tauri/src/sequencer/mod.rs`
`program_fm`; VGM export comments "TL with volume/velocity attenuation") — semantics
already correct; the surfacing (hex/TL readout, multi-note editing, ramps) is what lacks.

---

## 6. Song structure: patterns/orders vs regions

**Furnace** ✅ — the order list is a table where "each entry ... is the pattern that will
play during that order." **Duplicate order** reuses pattern references (edits propagate to
every order using that pattern); right-click **deep clone** "copies all patterns involved
to new ones." Reference-by-default, clone-on-demand.

**Deflemask** ✅ — pattern matrix with **per-channel orders**: each channel has its own
pattern sequence, so a 4-row bass loop repeats under an evolving 16-row lead for free.
Plus: clone-to-end button, **Alias mode** (name patterns, names shown on hover), and
Clone Pattern On Write as the safety net for editing a shared pattern.

**FL Studio** ✅ — pattern clips are references: "By default, all instances of a given
Clip share the same Channel or data"; **Make unique** "clones the original Clip so that
edits on the new Clip do not affect other instances."

**Ableton** — arrangement clips are independent copies (the outlier; general knowledge —
Live users route reuse through Session clips instead).

**Why it matters here:** chip music is the most repetition-heavy genre a DAW can host —
the tracker world settled on *reference-by-default + explicit unique* because editing the
chorus once and having all four choruses update is the daily workflow, and RAM/ROM-era
pattern reuse maps directly onto SMPS-style data reuse at export time.

**Seraph today:** regions are independent copies; `duplicate_region` IPC + `Ctrl+D` /
region `Ctrl+C`/`Ctrl+V` (Wave 2 parcel C) are all clone-semantics. No linked/aliased
regions, hence no Make Unique; no per-channel repeat structure; no region naming.

---

## 7. Navigation at song scale

**Furnace** ✅ — the order list doubles as the song map: "Follow orders" syncs it to
playback, clicking an order jumps; "Follow pattern" keeps the cursor in view. A pattern is
roughly one screen, so song-scale navigation = order-scale navigation.

**Deflemask** ✅ — pattern matrix + **Alias names** turn the matrix into a labeled section
index; "Follow cursor" setting for playback scroll.

**FL Studio** ✅ — `Ctrl+Mouse-wheel` zoom at pointer; middle-mouse drag pans both axes;
**Zoom tool (Z)** with drag-to-zoom-region and presets (`Shift+1..5`, `Page Up/Down`
around cursor); scroll-handle edge-drag = zoom; vertical zoom via MMB on the preview
keyboard.

**Ableton** — Arrangement Overview strip (clickable minimap above the timeline) +
`+`/`-` zoom keys (general knowledge; not fetched).

**Seraph today:** `Ctrl+wheel` zoom in arrangement (`useArrangementZoom`) and piano roll;
follow-playhead with 80%→10% paging and 2 s manual-scroll suspend (Wave 1 parcel A).
Lacks: vertical zoom (G16), overview/minimap strip, named section markers /
click-to-jump beyond ruler seek, zoom-to-selection; wheel-axis polish open (G31/32).
The tracker order-list habit maps onto **named section markers on the ruler** more
naturally than onto a minimap — both eventually, markers first.

---

## 8. Keyboard-centricity — the expected vocabulary

| Operation | Furnace ✅ | Deflemask ✅ | FL Studio ✅ | Ableton ✅ | Seraph today |
|---|---|---|---|---|---|
| Play/stop | `Return` toggle, `F5` from start | `Return`/`F5` song, `Alt+Return` pattern, `Shift+Return` from pos | Space (gen. knowledge) | Space | `Space` play/pause; stop double-tap = return to start (ruling) |
| Edit/record mode | `Space` = edit toggle | `Space` = recording | — (tools P/B/Y) | `B` = draw | — (no mode; double-click draws) |
| Octave shift | `Keypad *` / `Keypad /` | numpad `Multiply`/`Divide` | — | — | n/a (no entry keyboard yet) |
| Transpose sel. | `Ctrl-F1/F2` ±1, `Ctrl-F3/F4` ±oct | `Ctrl+F1..F4` same | Shift+drag / arrows | `↑/↓` ±1, `Shift+↑/↓` ±oct | `↑/↓` ±1, `Ctrl+↑/↓` ±oct |
| Nudge time | edit-step advance | Insert/Backspace row shifts | `Shift+arrows` snap, `Alt+arrows` px | `←/→` by grid | `←/→` grid, `Ctrl+←/→` 1 tick |
| Duplicate | order duplicate / deep clone | `Ctrl+D` clone-on-write | `Ctrl+B` | `Ctrl+D` | `Ctrl+D` (notes+regions) |
| Note off / delete | `OFF`/`===`/`REL` entries | `Tab` note-off, `Delete` | right-click delete | click in Draw Mode | `Delete`/`Backspace` |
| Channel mute | — (in doc'd excerpt) | `Ctrl+1..9` | — | — | (track header UI only) |
| Loop arm | repeat-pattern button | `Alt+Return` | PAT mode | `Ctrl+L` | `l` re-arm last range |
| Snap bypass | — | — | hold `Alt` | — | **conflict:** Alt = pan (Ableton-grammar ruling) |

Cross-world collisions worth knowing: `Space` means *play* to DAW hands but *edit/record
toggle* to tracker hands — seraph's DAW reading is correct, but binding **`Enter` as
play/stop too** (currently unbound in seraph) gives tracker hands their transport for
free. FL's hold-`Alt`-to-bypass-snap collides with seraph's Alt-pan ruling — bypass-snap
needs a different modifier or a snap=Off mode (already shipped in the snap selector).
Seraph has no keymap reference panel (G39) — all four comparators ship one (Furnace and
Deflemask keymaps are fully rebindable ✅).

---

## Expectations shortlist — the 10 idioms that decide "feels right"

Marked **adopt** (take as-is), **adapt** (take the intent, reshape for a piano-roll DAW),
or **reject** (consciously replace, with reason).

1. **Linked pattern reuse with Make Unique** (Furnace duplicate-vs-deep-clone, FL shared
   clips) — **adopt.** Reference-by-default regions + explicit "make unique" is the
   single highest-leverage feature for chip music's repetition; clone-only regions will
   feel broken to both tracker and FL hands. (Biggest current gap; touches S4 region
   model — flag before S4 hardens region identity.)
2. **Gapless edit-while-looping** (tracker live edit mode; Ableton brace-drag while
   playing) — **adopt** as a hard guarantee: no note/region edit, loop move, or patch
   tweak may audibly interrupt the cycling loop. Verify `reloadSequence` under loop; this
   is the tracker user's deepest assumption.
3. **QWERTY musical keyboard** (Furnace/Deflemask native; FL `Ctrl+T`) — **adopt**, and
   resolve G36 as: keys audition always; with a note selected or a step-entry mode armed,
   keys insert at the seek cursor and advance by the grid snap (the snap *is* the edit
   step). Both tracker and FL hands expect this exact loop.
4. **Audition on every pitch-changing gesture** (Ableton Preview: add/select/move;
   Furnace always-live keys) — **adopt**: extend seraph's existing audition to drag-move
   and arrow-transpose, with a global preview toggle. On FM timbres, silent transpose is
   flying blind.
5. **Paint/brush entry** (FL Paint tool B) — **adapt**: keep double-click-draw as the
   ruled primary, add drag-to-paint (repeated grid notes) as a modifier or tool, since
   chip leads/drums are runs of equal notes. Do not adopt FL's tool-mode-cycling UI.
6. **Volume column semantics in TL terms** (Deflemask 0–7F FM volume; per-op TL effects)
   — **adapt**: keep the 0–127 velocity lane (DAW skin) but surface effective TL/hex in
   the note info readout, add multi-note ramp/level tools (G17), and make any
   velocity→modulator-TL coupling an explicit per-patch opt-in, never default.
7. **Instrument editor always-at-hand, never modal** (Deflemask floating subwindow,
   Furnace window) — **adopt/keep**: seraph's BottomPanel already matches; extend with
   Deflemask's per-operator mute-while-editing (cheap, beloved isolation tool) and keep
   patch tweaks audible over the running loop (see #2).
8. **Per-row instrument changes** (tracker Ins column) — **adapt, reject the per-note
   form**: per-note patch fields on melodic tracks would fight the track-instrument model
   and the undo scope ruling; per-**region** patch assignment (region overrides track
   default) captures ~all real uses (verse bass vs chorus brass) with DAW-shaped UI.
9. **Correct-by-construction chip limits** (fixed roster, per-system volume max, per-chip
   effect vocabulary) — **adopt/keep**: seraph's seeded fixed channel roster is already
   the tracker stance; extend it to every new surface (no add-track beyond the driver
   profile, DAC-mode toggling as an explicit FM6 affordance like Deflemask's `17xx`,
   ranges clamped in-UI, nothing deferred to export-time validation).
10. **Space-as-play, not Space-as-edit** (DAW convention vs tracker `Space` =
    edit/record) — **reject the tracker binding, with compensation**: seraph is a DAW and
    Space=transport is ruled and correct; compensate tracker hands by also binding
    `Enter` to play/stop (both trackers' primary transport key ✅) and shipping a
    rebindable keymap panel (G39) — both trackers ship full key customization.

Near-misses (11–13, for the backlog): named section markers + click-to-jump (the
order-list/Alias habit, §7); `Ctrl+L` loop-the-selection (§3); metronome (G35 — both
Furnace and FL surface one prominently ✅/gen.).

---

## Source list

- Furnace manual: https://tildearrow.org/furnace/doc/latest/ — sections `3-pattern/`,
  `2-interface/keyboard.html`, `2-interface/play-edit-controls.html`,
  `2-interface/order-list.html`, `4-instrument/`.
- DefleMask Tracker Manual v2.0.0: https://deflemask.com/manual.pdf (Pattern Matrix,
  Instrument Editor Window, Default Controls, Genesis chapter).
- FL Studio manual: https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/pianoroll.htm
  and `.../playlist.htm`; typing-keyboard layout via secondary tutorials (liveaspects.com,
  itsgratuitous.com).
- Ableton Live 12 manual: https://www.ableton.com/en/manual/editing-midi/ ;
  Arrangement loop (`Ctrl+L`, brace drag) via search summaries of
  https://www.ableton.com/en/manual/arrangement-view/ .
- Seraph: this worktree at `db6f96a` (components, `src/utils/*`,
  `src-tauri/src/sequencer/mod.rs`, banking-queue Log 2026-08-21 entries).
