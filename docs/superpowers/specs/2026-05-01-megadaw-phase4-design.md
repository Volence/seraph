# MegaDAW Phase 4: Sequencer + Playback — Design Spec

## Overview

Phase 4 adds the core music creation workflow: an arrangement view for organizing tracks and regions on a timeline, a piano roll for editing notes within regions, and a playback engine that sequences notes through the Nuked OPN2 / SN76489 emulators in real-time. The sequencer runs inside the audio thread for sample-accurate timing.

**Builds on:** Phase 1 (audio engine), Phase 2 (project model + IPC), Phase 3 (DAW shell + instrument editors)

**Tech stack:** Tauri v2, React 19, TypeScript 5.8, Rust, cpal, CSS Modules, HTML5 Canvas

---

## 1. Playback Engine

### 1.1 Sequencer in the Audio Thread

The `Sequencer` struct lives inside `AudioEngine` and advances during `render()`. Every audio sample advances a fractional tick counter:

```
ticks_per_sample = (tempo_bpm / 60.0) * ticks_per_beat / sample_rate
```

At 140 BPM, 480 ticks/beat, 44100 Hz: `ticks_per_sample ≈ 0.2540`. Roughly every 4 samples a new tick fires.

When the tick counter crosses a whole tick boundary, the sequencer scans all active channel event lists for:
- Notes starting on this tick → program instrument registers (if changed) + set frequency + key-on
- Notes ending on this tick → key-off

### 1.2 SequencerSnapshot

A flattened, audio-thread-optimized representation of the song. No UUIDs, no strings — just indices and tick values.

```rust
struct SequencerSnapshot {
    tempo_bpm: f64,
    ticks_per_beat: u32,
    loop_start: Option<u64>,
    loop_end: Option<u64>,
    channels: Vec<ChannelSequence>,
}

struct ChannelSequence {
    channel_type: ChannelType,  // Fm(u8), Psg(u8), PsgNoise, Dac(u8)
    events: Vec<SequencerEvent>,  // sorted by tick
    overlaps: Vec<OverlapWarning>,  // for frontend reporting
}

enum SequencerEvent {
    NoteOn { tick: u64, pitch: u8, velocity: u8, duration_ticks: u64, instrument: InstrumentData },
    NoteOff { tick: u64, pitch: u8 },
}
```

`InstrumentData` is an enum holding the pre-computed register values:
- `FmPatch([u8; 25])` — the 25-byte Flamedriver patch
- `PsgEnvelope { period: u16, envelope: Arc<Vec<u8>>, loop_point: Option<usize> }`
- `DacSample { samples: Arc<Vec<u8>>, sample_rate: u32 }`

### 1.3 Song Data Transfer

When the user edits the song (adds notes, moves regions, changes instruments), the frontend calls IPC to mutate the model in `ProjectManager`. A new `SequencerSnapshot` is built from the current song state and sent to the audio thread via the existing ring buffer as `AudioCommand::LoadSequence { snapshot }`. The audio thread swaps in the new snapshot at a safe point (between tick boundaries). Lock-free — no mutex on the audio thread.

### 1.4 Snapshot Building & Channel Merging

The snapshot builder:
1. Iterates all tracks in the song
2. Skips muted tracks; if any track is solo'd, skips non-solo'd tracks
3. Groups remaining tracks by their `ChannelAssignment`
4. For each hardware channel, merges all notes from all tracks targeting it into a single sorted event list (NoteOn at `start_tick`, NoteOff at `start_tick + duration_ticks`)
5. Detects overlaps: if a NoteOn occurs while another note is still active on the same channel, records an `OverlapWarning` with the tick range and source track IDs

### 1.5 Instrument Programming

When a NoteOn fires, the sequencer programs the hardware channel:

**FM channels:**
1. Check if the instrument changed since the last note on this channel (cache last instrument data per channel)
2. If changed: write all 25 register bytes (DT/MUL, RS/AR, AM/D1R, D2R, SL/RR, TL for each op, then FB/ALG)
3. Write frequency registers (block + F-num from MIDI note lookup)
4. Key-on (register $28 with operator mask)

**PSG channels:**
1. Write tone period registers
2. Start volume envelope stepping (reuse existing PSG envelope preview mechanism)

**DAC channel:**
1. Trigger sample playback via existing `DacPlayback` path

On NoteOff: FM → key-off (register $28 with 0 operator mask). PSG → set attenuation to $F (silence). DAC → no action (samples play to completion).

### 1.6 Transport Commands

New `AudioCommand` variants:
- `TransportPlay` — start playback from current tick position
- `TransportStop` — stop, key-off all active notes on all channels, reset PSG
- `TransportSeek { tick: u64 }` — jump to tick position (key-off all, reset state)
- `TransportSetLoop { start_tick: u64, end_tick: u64 }` — when playback reaches end_tick, jump to start_tick
- `TransportClearLoop` — disable looping
- `LoadSequence { snapshot: SequencerSnapshot }` — replace the current song data

### 1.7 Position Reporting

The sequencer writes the current tick to an `AtomicU64` on every tick advancement. A Tauri-side timer (~30Hz) reads this atomic and emits a `playback-position` event to the frontend with the current tick and playing state.

---

## 2. Song Model

### 2.1 Existing Model (No Changes)

The existing Rust structs are already sufficient:
- `Track` — id, name, channel, instrument_id, regions, muted, solo, volume, pan
- `Region` — id, start_tick, duration_ticks, notes
- `Note` — tick, pitch, velocity, duration_ticks
- `SongMetadata` — name, tempo, time_signature, ticks_per_beat, driver_id

### 2.2 New IPC Commands (~15)

**Track CRUD:**
- `add_track(name: String, channel: ChannelAssignment, instrument_id: Option<String>)` → String (track ID)
- `update_track(id: String, name: String, channel: ChannelAssignment, instrument_id: Option<String>, muted: bool, solo: bool, volume: u8, pan: Pan)`
- `delete_track(id: String)`
- `list_tracks()` → Vec<Track>

**Region CRUD:**
- `add_region(track_id: String, start_tick: u64, duration_ticks: u64)` → String (region ID)
- `update_region(track_id: String, region_id: String, start_tick: u64, duration_ticks: u64)`
- `delete_region(track_id: String, region_id: String)`

**Note CRUD:**
- `add_note(track_id: String, region_id: String, tick: u64, pitch: u8, velocity: u8, duration_ticks: u64)` → usize (note index)
- `update_note(track_id: String, region_id: String, note_index: usize, tick: u64, pitch: u8, velocity: u8, duration_ticks: u64)`
- `delete_note(track_id: String, region_id: String, note_index: usize)`

**Transport:**
- `transport_play()`
- `transport_stop()`
- `transport_seek(tick: u64)`
- `transport_set_loop(start_tick: u64, end_tick: u64)`
- `transport_clear_loop()`
- `get_playback_state()` → `{ playing: bool, tick: u64, loop_start: Option<u64>, loop_end: Option<u64> }`

**Validation:**
- `get_channel_overlaps()` → Vec of overlap warnings with tick ranges and track IDs

---

## 3. Arrangement View

### 3.1 Layout

The `MainArea` placeholder is replaced by the arrangement view when a project is open.

```
┌─────────────────────────────────────────────────┐
│ [Ruler: bars/beats, loop markers, click-to-seek]│
├────────────┬────────────────────────────────────┤
│ Track      │  Timeline canvas                   │
│ Headers    │  (regions as colored blocks,        │
│ (HTML/CSS) │   playback cursor, grid lines)     │
│            │                                    │
│ FM1-Bass   │  ██████░░░░░░████████░░░░░░░░░░░  │
│  M S       │                                    │
│ FM2-Lead   │  ░░░░░░░░████████████░░░░░░░░░░░  │
│  M S       │                                    │
│ PSG1       │  ██░░██░░██░░██░░██░░░░░░░░░░░░░  │
│  M S       │                                    │
├────────────┴────────────────────────────────────┤
│ + Add Track                                     │
└─────────────────────────────────────────────────┘
```

### 3.2 Track Headers (HTML/CSS)

Each track header shows:
- Track name (click to rename inline)
- Channel assignment badge (colored: blue `#4a9eff` for FM, green `#44cc66` for PSG, orange `#ff8844` for DAC)
- Mute (M) / Solo (S) toggle buttons
- Instrument selector dropdown (filtered by channel type)
- Right-click context menu: Rename, Delete, Change Channel

### 3.3 Timeline Canvas

Rendered with HTML5 Canvas for performance. Redraws on scroll, zoom, edit, or playback position change.

**Rendering layers (back to front):**
1. Background + row alternation (subtle stripe per track)
2. Grid lines (bar lines bold, beat lines lighter, sub-beat lines lightest at high zoom)
3. Loop region (shaded band if loop is active)
4. Regions (colored rectangles matching channel type, slightly rounded corners)
5. Note previews inside regions (thin horizontal bars, visible at medium-high zoom)
6. Overlap warnings (red border on conflicting regions)
7. Selection highlight (blue border on selected region)
8. Playback cursor (vertical white line)

### 3.4 Timeline Ruler

Sits above the timeline canvas. Shows bar numbers and beat subdivisions. Click to set playback position. Loop markers rendered as bracket handles that can be dragged.

### 3.5 Interactions

- **Double-click** empty space on a track → create 1-bar region at that position
- **Click** region → select it
- **Drag** region body → move in time (snaps to grid)
- **Drag** region left/right edge → resize
- **Double-click** region → open piano roll in BottomPanel
- **Delete** key → remove selected region
- **Ctrl+scroll** → horizontal zoom (zoom toward cursor position)
- **Shift+scroll** or horizontal scroll → timeline pan
- **Click** ruler → seek playback position

### 3.6 Zoom State

Zoom is stored as `ticksPerPixel` (how many ticks one pixel represents). Zooming in decreases this value (more detail), zooming out increases it. Default zoom shows roughly 16 bars across the viewport.

---

## 4. Piano Roll

### 4.1 Layout

Opens in the BottomPanel when a region is double-clicked. Replaces the instrument editor view.

```
┌─────────────────────────────────────────────────┐
│ [Region: FM1-Bass | Bars 3-6] [Grid: 1/16 ▼] [x]│
├──────┬──────────────────────────────────────────┤
│Piano │  Note grid (canvas)                      │
│Keys  │  colored note rectangles on pitch rows   │
│      │  with bar/beat grid lines                │
│  B5  │                                          │
│  A5  │  ░░░░██████░░░░░░░░░░░░░░░░░░░░░░░░░░  │
│  G5  │  ░░░░░░░░░░░░████░░░░░░░░░░░░░░░░░░░░  │
│  ... │                                          │
├──────┴──────────────────────────────────────────┤
│ Velocity lane (vertical bars per note)           │
└─────────────────────────────────────────────────┘
```

### 4.2 Header Bar

- Region label: track name + bar range (e.g., "FM1-Bass | Bars 3-6")
- Grid snap selector: 1/1, 1/2, 1/4, 1/8, 1/16, 1/32, 1/4T, 1/8T (triplets), Off
- Close button → returns to instrument editor view in BottomPanel

### 4.3 Piano Keys (Left Column)

- Channel-aware pitch range:
  - FM tracks: C1-B7 (MIDI 24-95)
  - PSG tracks: A1-B7 (MIDI 33-95)
  - DAC tracks: single row (sample trigger, pitch = 0, no vertical range)
- White/black key visual distinction
- Click a key to audition the note using the track's assigned instrument (sends preview command through existing preview IPC)

### 4.4 Note Grid (Canvas)

- Horizontal axis: ticks within the region (0 to region.duration_ticks)
- Vertical axis: pitch (one row per semitone, only the channel-valid range)
- Notes rendered as colored rectangles (channel color, darker on selected)
- Row shading: white key rows slightly lighter than black key rows for orientation
- Grid lines at bar/beat boundaries within the region

### 4.5 Note Interactions

- **Click** empty cell → place note (default velocity 100, duration = grid snap value)
- **Click** existing note → select it
- **Drag** right edge → resize duration (snaps to grid)
- **Drag** note body → move in pitch and time (snaps to grid)
- **Shift+click** → multi-select
- **Delete** key → remove selected notes
- **Ctrl+scroll** → horizontal zoom
- **Scroll** → vertical scroll (pitch)

### 4.6 Velocity Lane

- Bottom strip below the note grid
- One vertical bar per note, positioned horizontally aligned with the note
- Bar height = velocity / 127 * lane height
- Color intensity reflects velocity (brighter = louder)
- Drag bar top to adjust velocity

### 4.7 BottomPanel Routing

BottomPanel becomes a multi-mode container:
- **Piano roll mode:** when a region is double-clicked in the arrangement
- **Instrument editor mode:** when an instrument is selected in the sidebar browser
- Piano roll close button returns to instrument editor (if an instrument is selected) or empty state
- Opening an instrument while the piano roll is open switches to instrument editor
- Double-clicking a region while the instrument editor is open switches to piano roll

---

## 5. Transport Bar

### 5.1 Layout (Integrated into TopBar)

The existing TopBar transport placeholder area becomes functional:

```
│  ▶ ■ 🔁  │  1:1:000  │  140 BPM  │  4/4  │
│ play stop │  position │  tempo    │ time  │
│   loop    │ bar:bt:tk │           │ sig   │
```

### 5.2 Controls

- **Play/Stop** button: toggles playback. Shows play icon (▶) when stopped, stop icon (■) when playing. Sends `transport_play` / `transport_stop` IPC.
- **Loop toggle** button: enables/disables loop playback. When enabled, loop markers appear on the arrangement ruler. Sends `transport_set_loop` / `transport_clear_loop` IPC.
- **Position display**: read-only `Bar:Beat:Tick` display updated from `playback-position` events (~30Hz). Computed from current tick using ticks_per_beat and time signature.

### 5.3 Keyboard Shortcuts

- `Space` — play/stop toggle
- `L` — toggle loop mode
- `Home` — seek to tick 0

### 5.4 Frontend Position Sync

The `usePlaybackPosition` hook:
1. Listens to `playback-position` Tauri events (emitted ~30Hz from Rust)
2. Stores the last received tick + timestamp
3. Between events, interpolates the cursor position using `requestAnimationFrame` + known tempo (purely cosmetic smoothing)
4. Exposes `currentTick` to the arrangement view and piano roll for cursor rendering

---

## 6. Track Management

### 6.1 Add Track Dialog

Triggered by "+ Add Track" button below the track list.

Fields:
- **Name**: text input, auto-suggested based on channel (e.g., "FM1 - Untitled")
- **Channel**: dropdown grouped by type (FM1-FM6, PSG1-PSG3, PSG Noise, DAC), populated from driver channel layout
- **Instrument**: dropdown filtered by channel type (FM instruments for FM channels, PSG for PSG, DAC for DAC). Optional — can be set later.

### 6.2 Channel-Instrument Enforcement

The instrument dropdown only shows instruments compatible with the selected channel type. If a track's channel assignment changes from FM to PSG, the instrument is cleared (set to null) since it's no longer valid.

### 6.3 Mute/Solo Logic

- Mute: track excluded from sequencer snapshot
- Solo: only solo'd tracks included (when any solo is active)
- Mute + Solo: mute wins
- Applied at snapshot build time

### 6.4 Channel Overlap Validation

When building the SequencerSnapshot, overlapping notes on the same hardware channel are detected and reported:
- Overlapping regions get a red border in the arrangement view
- Warning indicator on affected track headers
- Playback still works — later note cuts the earlier one (key-off then key-on), matching hardware behavior
- `get_channel_overlaps()` IPC returns the list for frontend rendering

---

## 7. File Structure

### 7.1 New Rust Modules

```
src-tauri/src/
  sequencer/
    mod.rs              — Sequencer struct, tick advancement, note scheduling
    snapshot.rs         — SequencerSnapshot builder, channel merging, overlap detection
  audio/
    command.rs          — add transport + LoadSequence variants (modify existing)
    engine.rs           — integrate Sequencer into render loop (modify existing)
  ipc/
    commands.rs         — add ~15 new commands (modify existing)
  project/
    manager.rs          — add track/region/note CRUD + snapshot building (modify existing)
```

### 7.2 New Frontend Components

```
src/
  components/
    ArrangementView.tsx      — orchestrates track headers + timeline canvas
    ArrangementView.module.css
    TrackHeader.tsx           — single track: name, channel badge, M/S, instrument
    TrackHeader.module.css
    TimelineCanvas.tsx        — canvas: regions, grid, cursor, notes preview
    TimelineCanvas.module.css
    TimelineRuler.tsx         — bar/beat ruler, loop markers, click-to-seek
    TimelineRuler.module.css
    PianoRoll.tsx             — note editor: header + keys + grid + velocity
    PianoRoll.module.css
    PianoRollCanvas.tsx       — canvas: note rectangles, grid
    PianoRollCanvas.module.css
    PianoRollKeys.tsx         — pitch labels + audition clicks
    PianoRollKeys.module.css
    VelocityLane.tsx          — velocity bar editor (canvas)
    VelocityLane.module.css
    AddTrackDialog.tsx        — channel + name + instrument picker dialog
    AddTrackDialog.module.css
    TransportControls.tsx     — play/stop/loop buttons, position display
    TransportControls.module.css
  hooks/
    usePlaybackPosition.ts   — Tauri event listener + interpolation
    useArrangementZoom.ts    — zoom level + scroll state + Ctrl+scroll handler
```

### 7.3 Modified Files

- `App.tsx` — add selected region state, route BottomPanel between piano roll and instrument editor
- `BottomPanel.tsx` — accept `selectedRegion` prop, render PianoRoll or instrument editor
- `TopBar.tsx` — replace transport placeholders with TransportControls
- `MainArea.tsx` — replace placeholder with ArrangementView
- `model.ts` — add new TypeScript types (PlaybackState, OverlapWarning, etc.)
- `ipc.ts` — add ~15 new IPC wrappers

---

## 8. Scope

### 8.1 In Phase 4

- Arrangement view (tracks, regions, timeline ruler, zoom/scroll)
- Piano roll (note drawing, velocity lane, grid snap, channel-aware pitch range)
- Playback engine (tick sequencer in audio thread, tempo-synced, instrument programming)
- Transport (play/stop/loop, position display, keyboard shortcuts)
- Track management (create/delete/rename, channel routing, instrument assignment, mute/solo)
- Channel overlap validation (visual warnings, non-blocking)
- ~15 new IPC commands (total ~43)
- ~12 new frontend components + 2 hooks

### 8.2 Deferred

- Real-time recording (arm track, play notes from keyboard to record)
- Copy/paste/duplicate regions across tracks
- Undo/redo system
- Export to Flamedriver format
- Tempo changes mid-song (tempo automation)
- FM3 special mode (multi-frequency per channel)
- Track reordering via drag
- Region split/merge
