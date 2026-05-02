# MegaDAW Phase 4: Sequencer + Playback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add arrangement view, piano roll, and real-time playback engine that sequences notes through the YM2612/SN76489 emulators with sample-accurate timing.

**Architecture:** Sequencer lives inside the audio callback for zero-latency timing. Frontend sends high-level transport/edit commands via IPC; receives ~30Hz position updates via Tauri events. Song edits produce a `SequencerSnapshot` sent lock-free to the audio thread. UI uses HTML for track headers/controls, Canvas for timeline/note grids.

**Tech Stack:** Rust (sequencer, snapshot builder, IPC), TypeScript/React 19 (arrangement, piano roll, transport), HTML5 Canvas, CSS Modules, Tauri v2 events, cpal audio thread, rtrb ring buffer

---

## File Structure

### New Rust files
| File | Responsibility |
|------|---------------|
| `src-tauri/src/sequencer/mod.rs` | `Sequencer` struct: tick advancement, note scheduling, instrument programming, transport state |
| `src-tauri/src/sequencer/snapshot.rs` | `SequencerSnapshot`, `ChannelSequence`, `SequencerEvent`, `InstrumentData`, `OverlapWarning` types + snapshot builder from `Song` |

### Modified Rust files
| File | Changes |
|------|---------|
| `src-tauri/src/audio/command.rs` | Add `TransportPlay`, `TransportStop`, `TransportSeek`, `TransportSetLoop`, `TransportClearLoop`, `LoadSequence` variants |
| `src-tauri/src/audio/engine.rs` | Integrate `Sequencer`, call `sequencer.advance()` during `render()`, process transport commands |
| `src-tauri/src/audio/thread.rs` | Expose `AtomicU64` for tick position reporting |
| `src-tauri/src/ipc/commands.rs` | Add ~17 new IPC commands (track/region/note CRUD + transport + overlaps) |
| `src-tauri/src/ipc/mod.rs` | Re-export new commands |
| `src-tauri/src/project/manager.rs` | Add track/region/note CRUD methods + snapshot building |
| `src-tauri/src/lib.rs` | Register new commands, wire up position event emitter |

### New frontend files
| File | Responsibility |
|------|---------------|
| `src/hooks/usePlaybackPosition.ts` | Listen to `playback-position` Tauri events, interpolate cursor |
| `src/hooks/useArrangementZoom.ts` | Zoom state (`ticksPerPixel`), scroll offset, Ctrl+scroll handler |
| `src/components/TransportControls.tsx` + `.module.css` | Play/stop/loop buttons, position display |
| `src/components/ArrangementView.tsx` + `.module.css` | Orchestrate track headers + timeline + ruler |
| `src/components/TrackHeader.tsx` + `.module.css` | Single track: name, channel badge, M/S, instrument |
| `src/components/TimelineRuler.tsx` + `.module.css` | Bar/beat ruler canvas, loop markers, click-to-seek |
| `src/components/TimelineCanvas.tsx` + `.module.css` | Canvas: regions, grid, cursor, note previews |
| `src/components/AddTrackDialog.tsx` + `.module.css` | Modal: name, channel, instrument picker |
| `src/components/PianoRoll.tsx` + `.module.css` | Note editor orchestrator: header + keys + grid + velocity |
| `src/components/PianoRollCanvas.tsx` + `.module.css` | Canvas: note rectangles, grid |
| `src/components/PianoRollKeys.tsx` + `.module.css` | Pitch labels + audition clicks |
| `src/components/VelocityLane.tsx` + `.module.css` | Velocity bar editor canvas |

### Modified frontend files
| File | Changes |
|------|---------|
| `src/types/model.ts` | Add `PlaybackState`, `OverlapWarning`, `SelectedRegion` types |
| `src/api/ipc.ts` | Add ~17 new IPC wrappers |
| `src/App.tsx` | Add `selectedRegion` state, pass to BottomPanel, pass transport props |
| `src/components/TopBar.tsx` | Replace transport placeholders with `TransportControls` |
| `src/components/MainArea.tsx` | Replace placeholder with `ArrangementView` |
| `src/components/BottomPanel.tsx` | Accept `selectedRegion`, route between piano roll and instrument editor |

---

### Task 1: Sequencer Snapshot Types + Builder

**Files:**
- Create: `src-tauri/src/sequencer/mod.rs`
- Create: `src-tauri/src/sequencer/snapshot.rs`
- Modify: `src-tauri/src/lib.rs:1` (add `mod sequencer`)
- Modify: `src-tauri/src/project/manager.rs` (add `build_snapshot` method)

- [ ] **Step 1: Write snapshot type tests**

Create `src-tauri/src/sequencer/snapshot.rs` with types and tests:

```rust
use std::sync::Arc;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum ChannelType {
    Fm(u8),
    Psg(u8),
    PsgNoise,
    Dac(u8),
}

#[derive(Debug, Clone)]
pub enum InstrumentData {
    FmPatch([u8; 25]),
    PsgEnvelope { period: u16, envelope: Arc<Vec<u8>>, loop_point: Option<usize> },
    DacSample { samples: Arc<Vec<u8>>, sample_rate: u32 },
}

#[derive(Debug, Clone)]
pub enum SequencerEvent {
    NoteOn { tick: u64, pitch: u8, velocity: u8, duration_ticks: u64, instrument: InstrumentData },
    NoteOff { tick: u64, pitch: u8 },
}

impl SequencerEvent {
    pub fn tick(&self) -> u64 {
        match self {
            SequencerEvent::NoteOn { tick, .. } => *tick,
            SequencerEvent::NoteOff { tick, .. } => *tick,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlapWarning {
    pub channel_name: String,
    pub tick_start: u64,
    pub tick_end: u64,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ChannelSequence {
    pub channel_type: ChannelType,
    pub events: Vec<SequencerEvent>,
    pub overlaps: Vec<OverlapWarning>,
}

#[derive(Debug, Clone)]
pub struct SequencerSnapshot {
    pub tempo_bpm: f64,
    pub ticks_per_beat: u32,
    pub loop_start: Option<u64>,
    pub loop_end: Option<u64>,
    pub channels: Vec<ChannelSequence>,
}

impl SequencerSnapshot {
    pub fn empty() -> Self {
        Self {
            tempo_bpm: 120.0,
            ticks_per_beat: 480,
            loop_start: None,
            loop_end: None,
            channels: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_snapshot() {
        let snap = SequencerSnapshot::empty();
        assert_eq!(snap.tempo_bpm, 120.0);
        assert!(snap.channels.is_empty());
    }

    #[test]
    fn test_event_tick_accessor() {
        let on = SequencerEvent::NoteOn {
            tick: 480,
            pitch: 60,
            velocity: 100,
            duration_ticks: 240,
            instrument: InstrumentData::FmPatch([0; 25]),
        };
        assert_eq!(on.tick(), 480);
        let off = SequencerEvent::NoteOff { tick: 720, pitch: 60 };
        assert_eq!(off.tick(), 720);
    }

    #[test]
    fn test_instrument_data_clone() {
        let fm = InstrumentData::FmPatch([42; 25]);
        let fm2 = fm.clone();
        match fm2 {
            InstrumentData::FmPatch(bytes) => assert_eq!(bytes[0], 42),
            _ => panic!("wrong variant"),
        }
    }
}
```

- [ ] **Step 2: Create sequencer module file**

Create `src-tauri/src/sequencer/mod.rs`:

```rust
pub mod snapshot;

pub use snapshot::*;
```

- [ ] **Step 3: Add mod sequencer to lib.rs**

In `src-tauri/src/lib.rs`, add `mod sequencer;` after the existing mod declarations (line 8, after `mod ym2612;`):

```rust
mod sequencer;
```

- [ ] **Step 4: Run tests to verify types compile**

Run: `cd /home/volence/sonic_hacks/megadaw/src-tauri && cargo test --lib sequencer`
Expected: 3 tests pass

- [ ] **Step 5: Write snapshot builder in ProjectManager**

Add a `build_snapshot` method to `ProjectManager` in `src-tauri/src/project/manager.rs`. This method:
1. Iterates tracks, skipping muted / applying solo logic
2. Groups tracks by `ChannelAssignment`
3. For each channel, merges notes from all tracks into sorted events
4. Detects overlaps
5. Resolves instrument data (FM → 25-byte patch via driver, PSG → envelope, DAC → PCM arc)

Add these imports at the top of `manager.rs`:

```rust
use crate::sequencer::{
    SequencerSnapshot, ChannelSequence, ChannelType, SequencerEvent, InstrumentData, OverlapWarning,
};
use crate::model::song::ChannelAssignment;
use crate::audio::frequency::{midi_to_fm_freq, midi_to_psg_period};
```

Add this method to `impl ProjectManager` (before the test module):

```rust
    pub fn build_snapshot(&self) -> SequencerSnapshot {
        let metadata = match &self.metadata {
            Some(m) => m,
            None => return SequencerSnapshot::empty(),
        };

        let any_solo = self.tracks.iter().any(|t| t.solo);

        // Group tracks by channel assignment key
        let mut channel_map: std::collections::BTreeMap<String, Vec<&Track>> =
            std::collections::BTreeMap::new();

        for track in &self.tracks {
            if track.muted {
                continue;
            }
            if any_solo && !track.solo {
                continue;
            }
            let key = match &track.channel {
                ChannelAssignment::Fm(n) => format!("fm_{n}"),
                ChannelAssignment::Psg(n) => format!("psg_{n}"),
                ChannelAssignment::PsgNoise => "psg_noise".to_string(),
                ChannelAssignment::Dac(n) => format!("dac_{n}"),
            };
            channel_map.entry(key).or_default().push(track);
        }

        let driver = self.driver_registry.get(&metadata.driver_id);

        let mut channels = Vec::new();
        for (_key, tracks) in &channel_map {
            let channel_type = match &tracks[0].channel {
                ChannelAssignment::Fm(n) => ChannelType::Fm(*n),
                ChannelAssignment::Psg(n) => ChannelType::Psg(*n),
                ChannelAssignment::PsgNoise => ChannelType::PsgNoise,
                ChannelAssignment::Dac(n) => ChannelType::Dac(*n),
            };

            let mut events: Vec<SequencerEvent> = Vec::new();
            let mut overlap_sources: Vec<(u64, u64, String)> = Vec::new(); // start, end, track_id

            for track in tracks {
                let inst_data = self.resolve_instrument_data(track, driver);
                for region in &track.regions {
                    for note in &region.notes {
                        let abs_tick = region.start_tick + note.tick;
                        let end_tick = abs_tick + note.duration_ticks;
                        if let Some(ref data) = inst_data {
                            events.push(SequencerEvent::NoteOn {
                                tick: abs_tick,
                                pitch: note.pitch,
                                velocity: note.velocity,
                                duration_ticks: note.duration_ticks,
                                instrument: data.clone(),
                            });
                        }
                        events.push(SequencerEvent::NoteOff {
                            tick: end_tick,
                            pitch: note.pitch,
                        });
                        overlap_sources.push((abs_tick, end_tick, track.id.to_string()));
                    }
                }
            }

            // Sort: NoteOff before NoteOn at same tick (so key-off happens before key-on)
            events.sort_by(|a, b| {
                let ta = a.tick();
                let tb = b.tick();
                if ta != tb {
                    return ta.cmp(&tb);
                }
                let priority = |e: &SequencerEvent| -> u8 {
                    match e {
                        SequencerEvent::NoteOff { .. } => 0,
                        SequencerEvent::NoteOn { .. } => 1,
                    }
                };
                priority(a).cmp(&priority(b))
            });

            // Detect overlaps
            let mut overlaps = Vec::new();
            overlap_sources.sort_by_key(|s| s.0);
            for i in 0..overlap_sources.len() {
                for j in (i + 1)..overlap_sources.len() {
                    if overlap_sources[j].0 >= overlap_sources[i].1 {
                        break;
                    }
                    let ch_name = match &channel_type {
                        ChannelType::Fm(n) => format!("FM{}", n + 1),
                        ChannelType::Psg(n) => format!("PSG{}", n + 1),
                        ChannelType::PsgNoise => "PSG Noise".to_string(),
                        ChannelType::Dac(n) => format!("DAC{}", n + 1),
                    };
                    overlaps.push(OverlapWarning {
                        channel_name: ch_name,
                        tick_start: overlap_sources[j].0,
                        tick_end: overlap_sources[i].1.min(overlap_sources[j].1),
                        track_ids: vec![
                            overlap_sources[i].2.clone(),
                            overlap_sources[j].2.clone(),
                        ],
                    });
                }
            }

            channels.push(ChannelSequence {
                channel_type,
                events,
                overlaps,
            });
        }

        SequencerSnapshot {
            tempo_bpm: metadata.tempo,
            ticks_per_beat: metadata.ticks_per_beat,
            loop_start: None,
            loop_end: None,
            channels,
        }
    }

    fn resolve_instrument_data(
        &self,
        track: &Track,
        driver: Option<&dyn crate::model::driver::DriverProfile>,
    ) -> Option<InstrumentData> {
        let inst_id = track.instrument_id.as_ref()?;
        match &track.channel {
            ChannelAssignment::Fm(_) => {
                let inst = self.instruments.fm.iter().find(|i| &i.id == inst_id)?;
                let bytes: [u8; 25] = if let Some(drv) = driver {
                    let vec = drv.fm_to_bytes(inst);
                    let mut arr = [0u8; 25];
                    arr.copy_from_slice(&vec);
                    arr
                } else {
                    [0u8; 25]
                };
                Some(InstrumentData::FmPatch(bytes))
            }
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => {
                let inst = self.instruments.psg.iter().find(|i| &i.id == inst_id)?;
                Some(InstrumentData::PsgEnvelope {
                    period: 0, // period is per-note, set at scheduling time
                    envelope: Arc::new(inst.volume_sequence.clone()),
                    loop_point: inst.loop_point,
                })
            }
            ChannelAssignment::Dac(_) => {
                let inst = self.instruments.dac.iter().find(|i| &i.id == inst_id)?;
                let pcm = self.dac_pcm_cache.get(inst_id)?;
                Some(InstrumentData::DacSample {
                    samples: pcm.clone(),
                    sample_rate: inst.target_sample_rate,
                })
            }
        }
    }

    pub fn get_all_overlaps(&self) -> Vec<OverlapWarning> {
        let snapshot = self.build_snapshot();
        snapshot.channels.into_iter().flat_map(|ch| ch.overlaps).collect()
    }
```

- [ ] **Step 6: Write snapshot builder tests**

Add these tests at the bottom of the `tests` module in `manager.rs`:

```rust
    #[test]
    fn test_build_snapshot_empty_project() {
        let path = temp_project_path("snap_empty");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Empty", "flamedriver", 140.0, (4, 4)).unwrap();

        let snap = mgr.build_snapshot();
        assert_eq!(snap.tempo_bpm, 140.0);
        assert_eq!(snap.ticks_per_beat, 480);
        assert!(snap.channels.is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_skips_muted_tracks() {
        let path = temp_project_path("snap_mute");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Mute Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        mgr.tracks.push(Track {
            id: Uuid::new_v4(),
            name: "FM1".into(),
            channel: ChannelAssignment::Fm(0),
            instrument_id: Some(fm_id),
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: 480,
                notes: vec![Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 240 }],
            }],
            muted: true,
            solo: false,
            volume: 100,
            pan: Pan::Center,
        });

        let snap = mgr.build_snapshot();
        assert!(snap.channels.is_empty(), "muted track should be excluded");

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_solo_filters() {
        let path = temp_project_path("snap_solo");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Solo Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        let note = Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 240 };
        let region = Region {
            id: Uuid::new_v4(),
            start_tick: 0,
            duration_ticks: 480,
            notes: vec![note.clone()],
        };

        mgr.tracks.push(Track {
            id: Uuid::new_v4(),
            name: "FM1-Solo".into(),
            channel: ChannelAssignment::Fm(0),
            instrument_id: Some(fm_id),
            regions: vec![region.clone()],
            muted: false,
            solo: true,
            volume: 100,
            pan: Pan::Center,
        });
        mgr.tracks.push(Track {
            id: Uuid::new_v4(),
            name: "FM2-NotSolo".into(),
            channel: ChannelAssignment::Fm(1),
            instrument_id: Some(fm_id),
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: 480,
                notes: vec![note],
            }],
            muted: false,
            solo: false,
            volume: 100,
            pan: Pan::Center,
        });

        let snap = mgr.build_snapshot();
        assert_eq!(snap.channels.len(), 1, "only solo'd track should appear");

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_detects_overlaps() {
        let path = temp_project_path("snap_overlap");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Overlap Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        // Two tracks both on FM1 with overlapping notes
        mgr.tracks.push(Track {
            id: Uuid::new_v4(),
            name: "FM1-A".into(),
            channel: ChannelAssignment::Fm(0),
            instrument_id: Some(fm_id),
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: 960,
                notes: vec![Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 480 }],
            }],
            muted: false,
            solo: false,
            volume: 100,
            pan: Pan::Center,
        });
        mgr.tracks.push(Track {
            id: Uuid::new_v4(),
            name: "FM1-B".into(),
            channel: ChannelAssignment::Fm(0),
            instrument_id: Some(fm_id),
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: 960,
                notes: vec![Note { tick: 240, pitch: 64, velocity: 100, duration_ticks: 480 }],
            }],
            muted: false,
            solo: false,
            volume: 100,
            pan: Pan::Center,
        });

        let snap = mgr.build_snapshot();
        assert_eq!(snap.channels.len(), 1);
        assert!(!snap.channels[0].overlaps.is_empty(), "should detect overlap");

        cleanup(&path);
    }

    #[test]
    fn test_build_snapshot_events_sorted() {
        let path = temp_project_path("snap_sorted");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Sort Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let fm_inst = FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 0,
            feedback: 0,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        };
        let fm_id = mgr.add_fm_instrument(fm_inst);

        mgr.tracks.push(Track {
            id: Uuid::new_v4(),
            name: "FM1".into(),
            channel: ChannelAssignment::Fm(0),
            instrument_id: Some(fm_id),
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: 1920,
                notes: vec![
                    Note { tick: 480, pitch: 60, velocity: 100, duration_ticks: 240 },
                    Note { tick: 0, pitch: 48, velocity: 100, duration_ticks: 480 },
                ],
            }],
            muted: false,
            solo: false,
            volume: 100,
            pan: Pan::Center,
        });

        let snap = mgr.build_snapshot();
        let ticks: Vec<u64> = snap.channels[0].events.iter().map(|e| e.tick()).collect();
        assert!(ticks.windows(2).all(|w| w[0] <= w[1]), "events should be sorted by tick");

        cleanup(&path);
    }
```

- [ ] **Step 7: Run all tests**

Run: `cd /home/volence/sonic_hacks/megadaw/src-tauri && cargo test`
Expected: All existing 54 tests + 8 new tests pass (62 total)

- [ ] **Step 8: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src-tauri/src/sequencer/ src-tauri/src/lib.rs src-tauri/src/project/manager.rs
git commit -m "feat(sequencer): snapshot types + builder with overlap detection"
```

---

### Task 2: Transport AudioCommands + Sequencer Core

**Files:**
- Modify: `src-tauri/src/audio/command.rs` (add transport variants)
- Modify: `src-tauri/src/audio/engine.rs` (integrate Sequencer)
- Modify: `src-tauri/src/audio/thread.rs` (expose AtomicU64 position)
- Modify: `src-tauri/src/sequencer/mod.rs` (add Sequencer struct with tick loop)

- [ ] **Step 1: Add transport commands to AudioCommand**

In `src-tauri/src/audio/command.rs`, add these variants to the `AudioCommand` enum (after `StopPreview`):

```rust
    TransportPlay,
    TransportStop,
    TransportSeek { tick: u64 },
    TransportSetLoop { start_tick: u64, end_tick: u64 },
    TransportClearLoop,
    LoadSequence { snapshot: crate::sequencer::SequencerSnapshot },
```

- [ ] **Step 2: Run existing tests to verify compilation**

Run: `cd /home/volence/sonic_hacks/megadaw/src-tauri && cargo test audio::command`
Expected: 2 existing tests pass

- [ ] **Step 3: Write the Sequencer struct**

Replace `src-tauri/src/sequencer/mod.rs` with the full sequencer implementation:

```rust
pub mod snapshot;

pub use snapshot::*;

use crate::audio::frequency::{midi_to_fm_freq, midi_to_psg_period};
use std::sync::Arc;

const OP_REG_OFFSETS: [u8; 4] = [0x00, 0x08, 0x04, 0x0C];

pub struct Sequencer {
    snapshot: SequencerSnapshot,
    playing: bool,
    current_tick: f64,
    ticks_per_sample: f64,
    sample_rate: f64,
    // Per-channel playback cursors (index into events vec)
    channel_cursors: Vec<usize>,
    // Cache last-programmed FM patch per YM2612 channel (0-5) to avoid redundant writes
    last_fm_patch: [[u8; 25]; 6],
    // Track which notes are currently sounding per channel for key-off
    active_notes: Vec<Option<u8>>, // pitch per channel
}

#[derive(Debug, Clone)]
pub struct FmRegisterWrite {
    pub port: u32,
    pub data: u8,
}

#[derive(Debug, Clone)]
pub enum SequencerOutput {
    FmWrite(FmRegisterWrite),
    PsgWrite(u8),
    DacPlayback { samples: Arc<Vec<u8>>, sample_rate: u32 },
}

impl Sequencer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            snapshot: SequencerSnapshot::empty(),
            playing: false,
            current_tick: 0.0,
            ticks_per_sample: 0.0,
            sample_rate: sample_rate as f64,
            channel_cursors: Vec::new(),
            last_fm_patch: [[0xFF; 25]; 6],
            active_notes: Vec::new(),
        }
    }

    pub fn load_snapshot(&mut self, snapshot: SequencerSnapshot) {
        self.ticks_per_sample =
            (snapshot.tempo_bpm / 60.0) * snapshot.ticks_per_beat as f64 / self.sample_rate;
        self.channel_cursors = vec![0; snapshot.channels.len()];
        self.active_notes = vec![None; snapshot.channels.len()];
        self.snapshot = snapshot;
    }

    pub fn play(&mut self) {
        if self.snapshot.channels.is_empty() && self.snapshot.tempo_bpm == 120.0 {
            return;
        }
        self.playing = true;
        self.seek_cursors();
    }

    pub fn stop(&mut self, output: &mut Vec<SequencerOutput>) {
        self.playing = false;
        self.silence_all(output);
    }

    pub fn seek(&mut self, tick: u64, output: &mut Vec<SequencerOutput>) {
        self.current_tick = tick as f64;
        self.silence_all(output);
        self.seek_cursors();
    }

    pub fn set_loop(&mut self, start: u64, end: u64) {
        self.snapshot.loop_start = Some(start);
        self.snapshot.loop_end = Some(end);
    }

    pub fn clear_loop(&mut self) {
        self.snapshot.loop_start = None;
        self.snapshot.loop_end = None;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn current_tick_u64(&self) -> u64 {
        self.current_tick as u64
    }

    /// Advance by one audio sample. Returns register writes to execute.
    pub fn advance(&mut self, output: &mut Vec<SequencerOutput>) {
        if !self.playing {
            return;
        }

        let prev_tick = self.current_tick as u64;
        self.current_tick += self.ticks_per_sample;
        let new_tick = self.current_tick as u64;

        // Check loop
        if let (Some(loop_start), Some(loop_end)) = (self.snapshot.loop_start, self.snapshot.loop_end) {
            if new_tick >= loop_end {
                self.current_tick = loop_start as f64 + (self.current_tick - loop_end as f64);
                self.silence_all(output);
                self.seek_cursors();
                return;
            }
        }

        if new_tick == prev_tick {
            return;
        }

        // Process events for ticks (prev_tick, new_tick]
        for ch_idx in 0..self.snapshot.channels.len() {
            let cursor = self.channel_cursors[ch_idx];
            let events = &self.snapshot.channels[ch_idx].events;
            let channel_type = &self.snapshot.channels[ch_idx].channel_type;

            let mut new_cursor = cursor;
            for i in cursor..events.len() {
                let event = &events[i];
                if event.tick() > new_tick {
                    break;
                }
                if event.tick() > prev_tick && event.tick() <= new_tick {
                    self.process_event(ch_idx, channel_type, event, output);
                }
                new_cursor = i + 1;
            }
            self.channel_cursors[ch_idx] = new_cursor;
        }
    }

    fn process_event(
        &mut self,
        ch_idx: usize,
        channel_type: &ChannelType,
        event: &SequencerEvent,
        output: &mut Vec<SequencerOutput>,
    ) {
        match event {
            SequencerEvent::NoteOn { pitch, velocity, instrument, .. } => {
                // Key-off any currently active note on this channel
                if self.active_notes[ch_idx].is_some() {
                    self.key_off_channel(ch_idx, channel_type, output);
                }

                match channel_type {
                    ChannelType::Fm(hw_ch) => {
                        self.program_fm(*hw_ch, *pitch, instrument, output);
                    }
                    ChannelType::Psg(hw_ch) => {
                        self.program_psg(*hw_ch, *pitch, instrument, output);
                    }
                    ChannelType::PsgNoise => {
                        self.program_psg_noise(instrument, output);
                    }
                    ChannelType::Dac(_) => {
                        if let InstrumentData::DacSample { samples, sample_rate } = instrument {
                            output.push(SequencerOutput::DacPlayback {
                                samples: samples.clone(),
                                sample_rate: *sample_rate,
                            });
                        }
                    }
                }
                self.active_notes[ch_idx] = Some(*pitch);
            }
            SequencerEvent::NoteOff { .. } => {
                self.key_off_channel(ch_idx, channel_type, output);
                self.active_notes[ch_idx] = None;
            }
        }
    }

    fn program_fm(&mut self, hw_ch: u8, pitch: u8, instrument: &InstrumentData, output: &mut Vec<SequencerOutput>) {
        let (port_base, ch_offset) = if hw_ch < 3 { (0u32, hw_ch) } else { (2u32, hw_ch - 3) };

        if let InstrumentData::FmPatch(patch) = instrument {
            // Only reprogram if patch changed
            if self.last_fm_patch[hw_ch as usize] != *patch {
                // DT/MUL (4 ops) at offsets 0-3
                for (op_idx, &reg_off) in OP_REG_OFFSETS.iter().enumerate() {
                    let slot = reg_off + ch_offset;
                    self.fm_write(port_base, 0x30 + slot, patch[op_idx], output);      // DT/MUL
                    self.fm_write(port_base, 0x40 + slot, patch[4 + op_idx], output);  // TL
                    self.fm_write(port_base, 0x50 + slot, patch[8 + op_idx], output);  // RS/AR
                    self.fm_write(port_base, 0x60 + slot, patch[12 + op_idx], output); // AM/D1R
                    self.fm_write(port_base, 0x70 + slot, patch[16 + op_idx], output); // D2R
                    self.fm_write(port_base, 0x80 + slot, patch[20 + op_idx], output); // SL/RR
                }
                // FB/ALG
                self.fm_write(port_base, 0xB0 + ch_offset, patch[24], output);
                // L+R stereo
                self.fm_write(port_base, 0xB4 + ch_offset, 0xC0, output);
                self.last_fm_patch[hw_ch as usize] = *patch;
            }
        }

        // Frequency
        let (block, fnum) = midi_to_fm_freq(pitch);
        let freq_msb = (block << 3) | ((fnum >> 8) as u8 & 0x07);
        let freq_lsb = (fnum & 0xFF) as u8;
        self.fm_write(port_base, 0xA4 + ch_offset, freq_msb, output);
        self.fm_write(port_base, 0xA0 + ch_offset, freq_lsb, output);

        // Key-on: all operators
        let ch_encoded = if hw_ch < 3 { hw_ch } else { hw_ch + 1 };
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 0, data: 0x28 }));
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 1, data: 0xF0 | ch_encoded }));
    }

    fn program_psg(&self, hw_ch: u8, pitch: u8, _instrument: &InstrumentData, output: &mut Vec<SequencerOutput>) {
        let period = midi_to_psg_period(pitch);
        let low_nibble = (period & 0x0F) as u8;
        let high_bits = ((period >> 4) & 0x3F) as u8;
        output.push(SequencerOutput::PsgWrite(0x80 | (hw_ch << 5) | low_nibble));
        output.push(SequencerOutput::PsgWrite(high_bits));
        // Volume on (attenuation 0)
        output.push(SequencerOutput::PsgWrite(0x90 | (hw_ch << 5) | 0x00));
    }

    fn program_psg_noise(&self, instrument: &InstrumentData, output: &mut Vec<SequencerOutput>) {
        // Noise channel is PSG channel 3
        if let InstrumentData::PsgEnvelope { .. } = instrument {
            // White noise, medium rate
            output.push(SequencerOutput::PsgWrite(0xE0 | 0x04));
            output.push(SequencerOutput::PsgWrite(0x90 | (3 << 5) | 0x00));
        }
    }

    fn key_off_channel(&self, _ch_idx: usize, channel_type: &ChannelType, output: &mut Vec<SequencerOutput>) {
        match channel_type {
            ChannelType::Fm(hw_ch) => {
                let ch_encoded = if *hw_ch < 3 { *hw_ch } else { *hw_ch + 1 };
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 0, data: 0x28 }));
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 1, data: ch_encoded }));
            }
            ChannelType::Psg(hw_ch) => {
                output.push(SequencerOutput::PsgWrite(0x90 | (hw_ch << 5) | 0x0F));
            }
            ChannelType::PsgNoise => {
                output.push(SequencerOutput::PsgWrite(0x90 | (3 << 5) | 0x0F));
            }
            ChannelType::Dac(_) => {} // DAC samples play to completion
        }
    }

    fn silence_all(&mut self, output: &mut Vec<SequencerOutput>) {
        for ch_idx in 0..self.active_notes.len() {
            if self.active_notes[ch_idx].is_some() {
                let ct = self.snapshot.channels[ch_idx].channel_type.clone();
                self.key_off_channel(ch_idx, &ct, output);
                self.active_notes[ch_idx] = None;
            }
        }
    }

    fn seek_cursors(&mut self) {
        let tick = self.current_tick as u64;
        for (ch_idx, ch) in self.snapshot.channels.iter().enumerate() {
            self.channel_cursors[ch_idx] = ch
                .events
                .partition_point(|e| e.tick() <= tick);
        }
    }

    fn fm_write(&self, port_base: u32, addr: u8, data: u8, output: &mut Vec<SequencerOutput>) {
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: port_base, data: addr }));
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: port_base + 1, data }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_fm_snapshot() -> SequencerSnapshot {
        SequencerSnapshot {
            tempo_bpm: 120.0,
            ticks_per_beat: 480,
            loop_start: None,
            loop_end: None,
            channels: vec![ChannelSequence {
                channel_type: ChannelType::Fm(0),
                events: vec![
                    SequencerEvent::NoteOn {
                        tick: 0,
                        pitch: 60,
                        velocity: 100,
                        duration_ticks: 480,
                        instrument: InstrumentData::FmPatch([0; 25]),
                    },
                    SequencerEvent::NoteOff { tick: 480, pitch: 60 },
                ],
                overlaps: vec![],
            }],
        }
    }

    #[test]
    fn test_sequencer_starts_stopped() {
        let seq = Sequencer::new(44100);
        assert!(!seq.is_playing());
        assert_eq!(seq.current_tick_u64(), 0);
    }

    #[test]
    fn test_sequencer_play_advances_tick() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());
        seq.play();
        assert!(seq.is_playing());

        let mut output = Vec::new();
        // Advance many samples to cross tick boundaries
        for _ in 0..1000 {
            seq.advance(&mut output);
        }
        assert!(seq.current_tick_u64() > 0);
    }

    #[test]
    fn test_sequencer_emits_note_on() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());
        seq.play();

        let mut output = Vec::new();
        // The first note is at tick 0, so advancing past tick 0→1 should trigger it
        for _ in 0..100 {
            seq.advance(&mut output);
        }
        let has_fm_write = output.iter().any(|o| matches!(o, SequencerOutput::FmWrite(_)));
        assert!(has_fm_write, "should emit FM register writes for NoteOn");
    }

    #[test]
    fn test_sequencer_stop_silences() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());
        seq.play();

        let mut output = Vec::new();
        for _ in 0..100 {
            seq.advance(&mut output);
        }
        output.clear();

        seq.stop(&mut output);
        assert!(!seq.is_playing());
    }

    #[test]
    fn test_sequencer_seek() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());

        let mut output = Vec::new();
        seq.seek(240, &mut output);
        assert_eq!(seq.current_tick_u64(), 240);
    }

    #[test]
    fn test_sequencer_loop() {
        let mut seq = Sequencer::new(44100);
        let mut snap = make_fm_snapshot();
        snap.loop_start = Some(0);
        snap.loop_end = Some(480);
        seq.load_snapshot(snap);
        seq.play();

        let mut output = Vec::new();
        // Advance way past loop end — at 120 BPM, 480 ticks/beat = 1 beat = 0.5s = 22050 samples
        for _ in 0..50000 {
            seq.advance(&mut output);
        }
        // Tick should have looped back and be within [0, 480)
        assert!(seq.current_tick_u64() < 480, "should have looped: tick={}", seq.current_tick_u64());
    }
}
```

- [ ] **Step 4: Integrate Sequencer into AudioEngine**

In `src-tauri/src/audio/engine.rs`, add the sequencer field and process its output during render:

Add this import at the top:
```rust
use crate::sequencer::Sequencer;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
```

Add `sequencer` and `position` fields to `AudioEngine`:
```rust
    sequencer: Sequencer,
    position_tick: Arc<AtomicU64>,
    sequencer_output_buf: Vec<crate::sequencer::SequencerOutput>,
```

In `AudioEngine::new()`, initialize them:
```rust
        let sequencer = Sequencer::new(sample_rate);
        let position_tick = Arc::new(AtomicU64::new(0));
```

Add a public accessor:
```rust
    pub fn position_tick(&self) -> Arc<AtomicU64> {
        self.position_tick.clone()
    }
```

In `process_command`, add handlers for the new transport commands:
```rust
            AudioCommand::TransportPlay => {
                self.sequencer.play();
            }
            AudioCommand::TransportStop => {
                let mut output = Vec::new();
                self.sequencer.stop(&mut output);
                self.apply_sequencer_output(&mut output);
            }
            AudioCommand::TransportSeek { tick } => {
                let mut output = Vec::new();
                self.sequencer.seek(tick, &mut output);
                self.apply_sequencer_output(&mut output);
                self.position_tick.store(tick, Ordering::Relaxed);
            }
            AudioCommand::TransportSetLoop { start_tick, end_tick } => {
                self.sequencer.set_loop(start_tick, end_tick);
            }
            AudioCommand::TransportClearLoop => {
                self.sequencer.clear_loop();
            }
            AudioCommand::LoadSequence { snapshot } => {
                self.sequencer.load_snapshot(snapshot);
            }
```

Add a helper method to AudioEngine for applying sequencer output:
```rust
    fn apply_sequencer_output(&mut self, output: &mut Vec<crate::sequencer::SequencerOutput>) {
        for cmd in output.drain(..) {
            match cmd {
                crate::sequencer::SequencerOutput::FmWrite(w) => {
                    self.ym2612.write(w.port, w.data);
                    for _ in 0..24 { self.ym2612.clock(); }
                }
                crate::sequencer::SequencerOutput::PsgWrite(data) => {
                    self.sn76489.write(data);
                }
                crate::sequencer::SequencerOutput::DacPlayback { samples, sample_rate } => {
                    self.dac_samples = Some(samples);
                    self.dac_position = 0.0;
                    self.dac_step = sample_rate as f64 / self.sample_rate;
                }
            }
        }
    }
```

In `render()`, advance the sequencer once per sample (before the existing YM2612/PSG clock section, inside the `for frame in 0..frame_count` loop):
```rust
            // --- Sequencer ---
            self.sequencer_output_buf.clear();
            self.sequencer.advance(&mut self.sequencer_output_buf);
            if !self.sequencer_output_buf.is_empty() {
                let mut buf = std::mem::take(&mut self.sequencer_output_buf);
                self.apply_sequencer_output(&mut buf);
                self.sequencer_output_buf = buf;
            }
            if self.sequencer.is_playing() {
                self.position_tick.store(self.sequencer.current_tick_u64(), Ordering::Relaxed);
            }
```

- [ ] **Step 5: Expose position AtomicU64 from AudioThread**

In `src-tauri/src/audio/thread.rs`, add a `position_tick` field to `AudioThread`:

```rust
    position_tick: Arc<AtomicU64>,
```

Import `AtomicU64`:
```rust
use std::sync::atomic::AtomicU64;
```

In `AudioThread::new()`, capture the position from the engine before moving it into the closure. Since `AudioEngine` is created inside `new()` and then moved into the closure, capture `position_tick` before the move:

After `let mut engine = AudioEngine::new(sample_rate);`:
```rust
        let position_tick = engine.position_tick();
```

Store it in the returned struct:
```rust
        Ok(Self {
            producer,
            _stream: stream,
            running,
            position_tick,
        })
```

Add a public accessor:
```rust
    pub fn position_tick(&self) -> &Arc<AtomicU64> {
        &self.position_tick
    }
```

- [ ] **Step 6: Run all tests**

Run: `cd /home/volence/sonic_hacks/megadaw/src-tauri && cargo test`
Expected: All previous tests + 6 new sequencer tests pass

- [ ] **Step 7: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src-tauri/src/audio/ src-tauri/src/sequencer/
git commit -m "feat(sequencer): tick-based sequencer in audio thread with transport commands"
```

---

### Task 3: Track/Region/Note CRUD IPC Commands

**Files:**
- Modify: `src-tauri/src/project/manager.rs` (add track/region/note CRUD methods)
- Modify: `src-tauri/src/ipc/commands.rs` (add ~17 IPC commands)
- Modify: `src-tauri/src/ipc/mod.rs` (re-export new commands)
- Modify: `src-tauri/src/lib.rs` (register new commands)

- [ ] **Step 1: Add track/region/note CRUD to ProjectManager**

Add these methods to `impl ProjectManager` in `manager.rs`:

```rust
    // --- Track CRUD ---

    pub fn add_track(&mut self, name: String, channel: ChannelAssignment, instrument_id: Option<Uuid>) -> Uuid {
        let id = Uuid::new_v4();
        self.tracks.push(Track {
            id,
            name,
            channel,
            instrument_id,
            regions: Vec::new(),
            muted: false,
            solo: false,
            volume: 100,
            pan: Pan::Center,
        });
        id
    }

    pub fn update_track(
        &mut self,
        id: Uuid,
        name: String,
        channel: ChannelAssignment,
        instrument_id: Option<Uuid>,
        muted: bool,
        solo: bool,
        volume: u8,
        pan: Pan,
    ) -> Result<(), String> {
        let track = self.tracks.iter_mut().find(|t| t.id == id)
            .ok_or("track not found")?;
        track.name = name;
        track.channel = channel;
        track.instrument_id = instrument_id;
        track.muted = muted;
        track.solo = solo;
        track.volume = volume;
        track.pan = pan;
        Ok(())
    }

    pub fn delete_track(&mut self, id: Uuid) -> Result<(), String> {
        let pos = self.tracks.iter().position(|t| t.id == id)
            .ok_or("track not found")?;
        self.tracks.remove(pos);
        Ok(())
    }

    pub fn list_tracks(&self) -> &[Track] {
        &self.tracks
    }

    // --- Region CRUD ---

    pub fn add_region(&mut self, track_id: Uuid, start_tick: u64, duration_ticks: u64) -> Result<Uuid, String> {
        let track = self.tracks.iter_mut().find(|t| t.id == track_id)
            .ok_or("track not found")?;
        let id = Uuid::new_v4();
        track.regions.push(Region {
            id,
            start_tick,
            duration_ticks,
            notes: Vec::new(),
        });
        Ok(id)
    }

    pub fn update_region(&mut self, track_id: Uuid, region_id: Uuid, start_tick: u64, duration_ticks: u64) -> Result<(), String> {
        let track = self.tracks.iter_mut().find(|t| t.id == track_id)
            .ok_or("track not found")?;
        let region = track.regions.iter_mut().find(|r| r.id == region_id)
            .ok_or("region not found")?;
        region.start_tick = start_tick;
        region.duration_ticks = duration_ticks;
        Ok(())
    }

    pub fn delete_region(&mut self, track_id: Uuid, region_id: Uuid) -> Result<(), String> {
        let track = self.tracks.iter_mut().find(|t| t.id == track_id)
            .ok_or("track not found")?;
        let pos = track.regions.iter().position(|r| r.id == region_id)
            .ok_or("region not found")?;
        track.regions.remove(pos);
        Ok(())
    }

    // --- Note CRUD ---

    pub fn add_note(
        &mut self,
        track_id: Uuid,
        region_id: Uuid,
        tick: u64,
        pitch: u8,
        velocity: u8,
        duration_ticks: u64,
    ) -> Result<usize, String> {
        let track = self.tracks.iter_mut().find(|t| t.id == track_id)
            .ok_or("track not found")?;
        let region = track.regions.iter_mut().find(|r| r.id == region_id)
            .ok_or("region not found")?;
        let idx = region.notes.len();
        region.notes.push(Note { tick, pitch, velocity, duration_ticks });
        Ok(idx)
    }

    pub fn update_note(
        &mut self,
        track_id: Uuid,
        region_id: Uuid,
        note_index: usize,
        tick: u64,
        pitch: u8,
        velocity: u8,
        duration_ticks: u64,
    ) -> Result<(), String> {
        let track = self.tracks.iter_mut().find(|t| t.id == track_id)
            .ok_or("track not found")?;
        let region = track.regions.iter_mut().find(|r| r.id == region_id)
            .ok_or("region not found")?;
        let note = region.notes.get_mut(note_index)
            .ok_or("note index out of range")?;
        note.tick = tick;
        note.pitch = pitch;
        note.velocity = velocity;
        note.duration_ticks = duration_ticks;
        Ok(())
    }

    pub fn delete_note(
        &mut self,
        track_id: Uuid,
        region_id: Uuid,
        note_index: usize,
    ) -> Result<(), String> {
        let track = self.tracks.iter_mut().find(|t| t.id == track_id)
            .ok_or("track not found")?;
        let region = track.regions.iter_mut().find(|r| r.id == region_id)
            .ok_or("region not found")?;
        if note_index >= region.notes.len() {
            return Err("note index out of range".into());
        }
        region.notes.remove(note_index);
        Ok(())
    }
```

- [ ] **Step 2: Write CRUD tests**

Add to the `tests` module in `manager.rs`:

```rust
    #[test]
    fn test_track_crud() {
        let path = temp_project_path("track_crud");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Track Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let id = mgr.add_track("FM1-Bass".into(), ChannelAssignment::Fm(0), None);
        assert_eq!(mgr.list_tracks().len(), 1);
        assert_eq!(mgr.list_tracks()[0].name, "FM1-Bass");

        mgr.update_track(id, "FM1-Lead".into(), ChannelAssignment::Fm(0), None, true, false, 80, Pan::Left).unwrap();
        assert_eq!(mgr.list_tracks()[0].name, "FM1-Lead");
        assert!(mgr.list_tracks()[0].muted);

        mgr.delete_track(id).unwrap();
        assert!(mgr.list_tracks().is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_region_crud() {
        let path = temp_project_path("region_crud");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Region Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        assert_eq!(mgr.list_tracks()[0].regions.len(), 1);

        mgr.update_region(track_id, region_id, 480, 960).unwrap();
        assert_eq!(mgr.list_tracks()[0].regions[0].start_tick, 480);

        mgr.delete_region(track_id, region_id).unwrap();
        assert!(mgr.list_tracks()[0].regions.is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_note_crud() {
        let path = temp_project_path("note_crud");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Note Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();

        let idx = mgr.add_note(track_id, region_id, 0, 60, 100, 240).unwrap();
        assert_eq!(idx, 0);
        assert_eq!(mgr.list_tracks()[0].regions[0].notes.len(), 1);

        mgr.update_note(track_id, region_id, 0, 120, 64, 80, 480).unwrap();
        assert_eq!(mgr.list_tracks()[0].regions[0].notes[0].pitch, 64);

        mgr.delete_note(track_id, region_id, 0).unwrap();
        assert!(mgr.list_tracks()[0].regions[0].notes.is_empty());

        cleanup(&path);
    }

    #[test]
    fn test_track_save_load_round_trip() {
        let path = temp_project_path("track_save");
        let mut mgr = ProjectManager::new(test_registry());
        mgr.create(&path, "Save Test", "flamedriver", 120.0, (4, 4)).unwrap();

        let track_id = mgr.add_track("FM1".into(), ChannelAssignment::Fm(0), None);
        let region_id = mgr.add_region(track_id, 0, 1920).unwrap();
        mgr.add_note(track_id, region_id, 0, 60, 100, 480).unwrap();
        mgr.add_note(track_id, region_id, 480, 64, 80, 240).unwrap();

        mgr.save().unwrap();
        mgr.close();

        let song = mgr.open(&path).unwrap();
        assert_eq!(song.tracks.len(), 1);
        assert_eq!(song.tracks[0].regions.len(), 1);
        assert_eq!(song.tracks[0].regions[0].notes.len(), 2);
        assert_eq!(song.tracks[0].regions[0].notes[0].pitch, 60);

        cleanup(&path);
    }
```

- [ ] **Step 3: Run manager tests**

Run: `cd /home/volence/sonic_hacks/megadaw/src-tauri && cargo test project`
Expected: All existing + 4 new tests pass

- [ ] **Step 4: Add IPC commands for track/region/note CRUD + transport + overlaps**

Append to `src-tauri/src/ipc/commands.rs` (after the existing `get_dac_pcm_data` function):

```rust
// --- Track CRUD ---

#[tauri::command]
pub fn add_track(
    state: State<'_, ProjectState>,
    name: String,
    channel: crate::model::song::ChannelAssignment,
    instrument_id: Option<String>,
) -> Result<String, String> {
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let inst_uuid = instrument_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| format!("invalid UUID: {e}"))?;
    let id = mgr.add_track(name, channel, inst_uuid);
    Ok(id.to_string())
}

#[tauri::command]
pub fn update_track(
    state: State<'_, ProjectState>,
    id: String,
    name: String,
    channel: crate::model::song::ChannelAssignment,
    instrument_id: Option<String>,
    muted: bool,
    solo: bool,
    volume: u8,
    pan: crate::model::song::Pan,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let inst_uuid = instrument_id
        .map(|s| Uuid::parse_str(&s))
        .transpose()
        .map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_track(uuid, name, channel, inst_uuid, muted, solo, volume, pan)
}

#[tauri::command]
pub fn delete_track(state: State<'_, ProjectState>, id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_track(uuid)
}

#[tauri::command]
pub fn list_tracks(state: State<'_, ProjectState>) -> Result<Vec<crate::model::song::Track>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.list_tracks().to_vec())
}

// --- Region CRUD ---

#[tauri::command]
pub fn add_region(
    state: State<'_, ProjectState>,
    track_id: String,
    start_tick: u64,
    duration_ticks: u64,
) -> Result<String, String> {
    let uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let id = mgr.add_region(uuid, start_tick, duration_ticks)?;
    Ok(id.to_string())
}

#[tauri::command]
pub fn update_region(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    start_tick: u64,
    duration_ticks: u64,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_region(t_uuid, r_uuid, start_tick, duration_ticks)
}

#[tauri::command]
pub fn delete_region(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_region(t_uuid, r_uuid)
}

// --- Note CRUD ---

#[tauri::command]
pub fn add_note(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    tick: u64,
    pitch: u8,
    velocity: u8,
    duration_ticks: u64,
) -> Result<usize, String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.add_note(t_uuid, r_uuid, tick, pitch, velocity, duration_ticks)
}

#[tauri::command]
pub fn update_note(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    note_index: usize,
    tick: u64,
    pitch: u8,
    velocity: u8,
    duration_ticks: u64,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.update_note(t_uuid, r_uuid, note_index, tick, pitch, velocity, duration_ticks)
}

#[tauri::command]
pub fn delete_note(
    state: State<'_, ProjectState>,
    track_id: String,
    region_id: String,
    note_index: usize,
) -> Result<(), String> {
    let t_uuid = Uuid::parse_str(&track_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let r_uuid = Uuid::parse_str(&region_id).map_err(|e| format!("invalid UUID: {e}"))?;
    let mut mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    mgr.delete_note(t_uuid, r_uuid, note_index)
}

// --- Transport ---

#[tauri::command]
pub fn transport_play(
    audio_state: State<'_, AudioState>,
    project_state: State<'_, ProjectState>,
) -> Result<(), String> {
    let mgr = project_state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let snapshot = mgr.build_snapshot();
    drop(mgr);

    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::LoadSequence { snapshot });
    thread.send(AudioCommand::TransportPlay);
    Ok(())
}

#[tauri::command]
pub fn transport_stop(audio_state: State<'_, AudioState>) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportStop);
    Ok(())
}

#[tauri::command]
pub fn transport_seek(audio_state: State<'_, AudioState>, tick: u64) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportSeek { tick });
    Ok(())
}

#[tauri::command]
pub fn transport_set_loop(audio_state: State<'_, AudioState>, start_tick: u64, end_tick: u64) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportSetLoop { start_tick, end_tick });
    Ok(())
}

#[tauri::command]
pub fn transport_clear_loop(audio_state: State<'_, AudioState>) -> Result<(), String> {
    let mut thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    thread.send(AudioCommand::TransportClearLoop);
    Ok(())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackState {
    pub playing: bool,
    pub tick: u64,
    pub loop_start: Option<u64>,
    pub loop_end: Option<u64>,
}

#[tauri::command]
pub fn get_playback_state(audio_state: State<'_, AudioState>) -> Result<PlaybackState, String> {
    let thread = audio_state.thread.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let tick = thread.position_tick().load(std::sync::atomic::Ordering::Relaxed);
    Ok(PlaybackState {
        playing: false, // We can't easily query this from the audio thread; frontend tracks it
        tick,
        loop_start: None,
        loop_end: None,
    })
}

#[tauri::command]
pub fn get_channel_overlaps(state: State<'_, ProjectState>) -> Result<Vec<crate::sequencer::OverlapWarning>, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    Ok(mgr.get_all_overlaps())
}
```

- [ ] **Step 5: Update ipc/mod.rs re-exports**

Replace `src-tauri/src/ipc/mod.rs` with:

```rust
pub mod commands;

pub use commands::{
    AudioState, ProjectState,
    // Phase 1
    play_fm_test_tone, play_psg_test_tone, stop_all_sound,
    // Project management
    create_project, open_project, save_project, close_project, get_project_info,
    // Driver info
    list_drivers, get_driver_info,
    // FM instruments
    add_fm_instrument, update_fm_instrument, delete_fm_instrument,
    list_fm_instruments, preview_fm_instrument,
    // PSG instruments
    add_psg_instrument, update_psg_instrument, delete_psg_instrument,
    list_psg_instruments, preview_psg_instrument,
    // DAC instruments
    import_dac_wav, import_dac_raw, update_dac_instrument, reconvert_dac,
    delete_dac_instrument, list_dac_instruments, preview_dac,
    get_dac_pcm_data,
    // Track CRUD
    add_track, update_track, delete_track, list_tracks,
    // Region CRUD
    add_region, update_region, delete_region,
    // Note CRUD
    add_note, update_note, delete_note,
    // Transport
    transport_play, transport_stop, transport_seek,
    transport_set_loop, transport_clear_loop, get_playback_state,
    // Validation
    get_channel_overlaps,
};
```

- [ ] **Step 6: Register new commands in lib.rs**

Update `src-tauri/src/lib.rs` to import and register all new commands. Add these to the import block:

```rust
    // Track CRUD
    add_track, update_track, delete_track, list_tracks,
    // Region CRUD
    add_region, update_region, delete_region,
    // Note CRUD
    add_note, update_note, delete_note,
    // Transport
    transport_play, transport_stop, transport_seek,
    transport_set_loop, transport_clear_loop, get_playback_state,
    // Validation
    get_channel_overlaps,
```

Add them all to `tauri::generate_handler![]`:

```rust
            // Track CRUD
            add_track,
            update_track,
            delete_track,
            list_tracks,
            // Region CRUD
            add_region,
            update_region,
            delete_region,
            // Note CRUD
            add_note,
            update_note,
            delete_note,
            // Transport
            transport_play,
            transport_stop,
            transport_seek,
            transport_set_loop,
            transport_clear_loop,
            get_playback_state,
            // Validation
            get_channel_overlaps,
```

- [ ] **Step 7: Add position event emitter to lib.rs**

In `lib.rs`, after creating the `audio_thread`, set up a background thread to emit position events at ~30Hz. This requires access to the `AudioThread`'s `position_tick` before wrapping it in a Mutex.

Restructure `run()`:

```rust
pub fn run() {
    let audio_thread = AudioThread::new().expect("failed to initialize audio thread");
    let position_tick = audio_thread.position_tick().clone();

    let mut registry = DriverRegistry::new();
    registry.register(Box::new(FlamedriverProfile));
    let project_manager = ProjectManager::new(registry);

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AudioState {
            thread: Mutex::new(audio_thread),
        })
        .manage(ProjectState {
            manager: Mutex::new(project_manager),
        })
        .setup(move |app| {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(33)); // ~30Hz
                    let tick = position_tick.load(std::sync::atomic::Ordering::Relaxed);
                    let _ = handle.emit("playback-position", tick);
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // ... all commands ...
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

Add `use tauri::Emitter;` to the imports at the top of `lib.rs`.

- [ ] **Step 8: Run all Rust tests**

Run: `cd /home/volence/sonic_hacks/megadaw/src-tauri && cargo test`
Expected: All tests pass (previous + 4 new CRUD tests)

- [ ] **Step 9: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src-tauri/
git commit -m "feat(ipc): track/region/note CRUD + transport + position events"
```

---

### Task 4: Frontend Types + IPC Wrappers

**Files:**
- Modify: `src/types/model.ts` (add PlaybackState, OverlapWarning, SelectedRegion)
- Modify: `src/api/ipc.ts` (add ~17 new IPC wrappers)

- [ ] **Step 1: Add new TypeScript types to model.ts**

Add to the end of `src/types/model.ts`:

```typescript
export interface PlaybackState {
  playing: boolean;
  tick: number;
  loopStart: number | null;
  loopEnd: number | null;
}

export interface OverlapWarning {
  channelName: string;
  tickStart: number;
  tickEnd: number;
  trackIds: string[];
}

export interface SelectedRegion {
  trackId: string;
  trackName: string;
  regionId: string;
  channelType: "fm" | "psg" | "dac";
  startTick: number;
  durationTicks: number;
}
```

- [ ] **Step 2: Add new IPC wrappers**

Append to `src/api/ipc.ts`:

```typescript
import type {
  FmInstrument,
  PsgInstrument,
  DacInstrument,
  Song,
  SongMetadata,
  DriverInfo,
  DriverDetail,
  Track,
  ChannelAssignment,
  Pan,
  PlaybackState,
  OverlapWarning,
} from "../types/model";
```

(Update the existing import to include `Track`, `ChannelAssignment`, `Pan`, `PlaybackState`, `OverlapWarning`.)

Add these functions:

```typescript
// --- Track CRUD ---

export async function addTrack(
  name: string,
  channel: ChannelAssignment,
  instrumentId: string | null,
): Promise<string> {
  return invoke<string>("add_track", { name, channel, instrumentId });
}

export async function updateTrack(
  id: string,
  name: string,
  channel: ChannelAssignment,
  instrumentId: string | null,
  muted: boolean,
  solo: boolean,
  volume: number,
  pan: Pan,
): Promise<void> {
  return invoke("update_track", { id, name, channel, instrumentId, muted, solo, volume, pan });
}

export async function deleteTrack(id: string): Promise<void> {
  return invoke("delete_track", { id });
}

export async function listTracks(): Promise<Track[]> {
  return invoke<Track[]>("list_tracks");
}

// --- Region CRUD ---

export async function addRegion(
  trackId: string,
  startTick: number,
  durationTicks: number,
): Promise<string> {
  return invoke<string>("add_region", { trackId, startTick, durationTicks });
}

export async function updateRegion(
  trackId: string,
  regionId: string,
  startTick: number,
  durationTicks: number,
): Promise<void> {
  return invoke("update_region", { trackId, regionId, startTick, durationTicks });
}

export async function deleteRegion(trackId: string, regionId: string): Promise<void> {
  return invoke("delete_region", { trackId, regionId });
}

// --- Note CRUD ---

export async function addNote(
  trackId: string,
  regionId: string,
  tick: number,
  pitch: number,
  velocity: number,
  durationTicks: number,
): Promise<number> {
  return invoke<number>("add_note", { trackId, regionId, tick, pitch, velocity, durationTicks });
}

export async function updateNote(
  trackId: string,
  regionId: string,
  noteIndex: number,
  tick: number,
  pitch: number,
  velocity: number,
  durationTicks: number,
): Promise<void> {
  return invoke("update_note", { trackId, regionId, noteIndex, tick, pitch, velocity, durationTicks });
}

export async function deleteNote(
  trackId: string,
  regionId: string,
  noteIndex: number,
): Promise<void> {
  return invoke("delete_note", { trackId, regionId, noteIndex });
}

// --- Transport ---

export async function transportPlay(): Promise<void> {
  return invoke("transport_play");
}

export async function transportStop(): Promise<void> {
  return invoke("transport_stop");
}

export async function transportSeek(tick: number): Promise<void> {
  return invoke("transport_seek", { tick });
}

export async function transportSetLoop(startTick: number, endTick: number): Promise<void> {
  return invoke("transport_set_loop", { startTick, endTick });
}

export async function transportClearLoop(): Promise<void> {
  return invoke("transport_clear_loop");
}

export async function getPlaybackState(): Promise<PlaybackState> {
  return invoke<PlaybackState>("get_playback_state");
}

// --- Validation ---

export async function getChannelOverlaps(): Promise<OverlapWarning[]> {
  return invoke<OverlapWarning[]>("get_channel_overlaps");
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src/types/model.ts src/api/ipc.ts
git commit -m "feat(frontend): TypeScript types + IPC wrappers for Phase 4"
```

---

### Task 5: Playback Position Hook + Transport Controls

**Files:**
- Create: `src/hooks/usePlaybackPosition.ts`
- Create: `src/components/TransportControls.tsx`
- Create: `src/components/TransportControls.module.css`
- Modify: `src/components/TopBar.tsx`
- Modify: `src/components/TopBar.module.css`

- [ ] **Step 1: Create usePlaybackPosition hook**

Create `src/hooks/usePlaybackPosition.ts`:

```typescript
import { useState, useEffect, useRef, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";

interface PlaybackPosition {
  currentTick: number;
  interpolatedTick: number;
}

export function usePlaybackPosition(
  playing: boolean,
  tempoBpm: number,
  ticksPerBeat: number,
): PlaybackPosition {
  const [currentTick, setCurrentTick] = useState(0);
  const lastEventRef = useRef<{ tick: number; time: number }>({ tick: 0, time: 0 });
  const interpolatedRef = useRef(0);
  const animFrameRef = useRef(0);

  useEffect(() => {
    const unlisten = listen<number>("playback-position", (event) => {
      const tick = event.payload;
      setCurrentTick(tick);
      lastEventRef.current = { tick, time: performance.now() };
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  const ticksPerMs = (tempoBpm / 60000) * ticksPerBeat;

  const animate = useCallback(() => {
    if (playing) {
      const elapsed = performance.now() - lastEventRef.current.time;
      interpolatedRef.current = lastEventRef.current.tick + elapsed * ticksPerMs;
    }
    animFrameRef.current = requestAnimationFrame(animate);
  }, [playing, ticksPerMs]);

  useEffect(() => {
    animFrameRef.current = requestAnimationFrame(animate);
    return () => cancelAnimationFrame(animFrameRef.current);
  }, [animate]);

  return {
    currentTick,
    interpolatedTick: playing ? interpolatedRef.current : currentTick,
  };
}
```

- [ ] **Step 2: Create TransportControls component**

Create `src/components/TransportControls.tsx`:

```typescript
import type { SongMetadata } from "../types/model";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import * as ipc from "../api/ipc";
import styles from "./TransportControls.module.css";

interface TransportControlsProps {
  projectMeta: SongMetadata;
  playing: boolean;
  loopEnabled: boolean;
  onPlayingChange: (playing: boolean) => void;
  onLoopChange: (enabled: boolean) => void;
}

function tickToBarBeatTick(
  tick: number,
  ticksPerBeat: number,
  beatsPerBar: number,
): string {
  const totalBeats = Math.floor(tick / ticksPerBeat);
  const bar = Math.floor(totalBeats / beatsPerBar) + 1;
  const beat = (totalBeats % beatsPerBar) + 1;
  const subTick = Math.floor(tick % ticksPerBeat);
  return `${bar}:${beat}:${String(subTick).padStart(3, "0")}`;
}

export function TransportControls({
  projectMeta,
  playing,
  loopEnabled,
  onPlayingChange,
  onLoopChange,
}: TransportControlsProps) {
  const { currentTick } = usePlaybackPosition(
    playing,
    projectMeta.tempo,
    projectMeta.ticksPerBeat,
  );

  async function handlePlayStop() {
    if (playing) {
      await ipc.transportStop();
      onPlayingChange(false);
    } else {
      await ipc.transportPlay();
      onPlayingChange(true);
    }
  }

  async function handleLoop() {
    if (loopEnabled) {
      await ipc.transportClearLoop();
      onLoopChange(false);
    } else {
      const ticksPerBar = projectMeta.ticksPerBeat * projectMeta.timeSignature[0];
      await ipc.transportSetLoop(0, ticksPerBar * 4);
      onLoopChange(true);
    }
  }

  async function handleHome() {
    await ipc.transportSeek(0);
  }

  const position = tickToBarBeatTick(
    currentTick,
    projectMeta.ticksPerBeat,
    projectMeta.timeSignature[0],
  );

  return (
    <div className={styles.transport}>
      <button
        className={`${styles.btn} ${playing ? styles.active : ""}`}
        onClick={handlePlayStop}
        title={playing ? "Stop (Space)" : "Play (Space)"}
      >
        {playing ? "■" : "▶"}
      </button>
      <button
        className={`${styles.btn} ${loopEnabled ? styles.active : ""}`}
        onClick={handleLoop}
        title="Loop (L)"
      >
        {"↻"}
      </button>
      <button className={styles.btn} onClick={handleHome} title="Home">
        {"⏮"}
      </button>
      <span className={styles.position}>{position}</span>
    </div>
  );
}
```

- [ ] **Step 3: Create TransportControls CSS**

Create `src/components/TransportControls.module.css`:

```css
.transport {
  display: flex;
  align-items: center;
  gap: 4px;
}

.btn {
  width: 32px;
  height: 28px;
  background: var(--bg-surface);
  color: var(--text-primary);
  border: 1px solid var(--border);
  border-radius: 3px;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn:hover {
  background: var(--border);
}

.btn.active {
  background: var(--accent-fm);
  color: #fff;
  border-color: var(--accent-fm);
}

.position {
  font-family: monospace;
  font-size: 13px;
  color: var(--text-primary);
  padding: 0 8px;
  min-width: 80px;
}
```

- [ ] **Step 4: Update TopBar to use TransportControls**

Replace the transport section in `src/components/TopBar.tsx`. The new TopBar accepts additional props and delegates to `TransportControls`:

```typescript
import type { SongMetadata } from "../types/model";
import { TransportControls } from "./TransportControls";
import styles from "./TopBar.module.css";

interface TopBarProps {
  projectMeta: SongMetadata | null;
  onNewProject: () => void;
  onOpenProject: () => void;
  onSave: () => void;
  showSaved: boolean;
  playing: boolean;
  loopEnabled: boolean;
  onPlayingChange: (playing: boolean) => void;
  onLoopChange: (enabled: boolean) => void;
}

export function TopBar({
  projectMeta,
  onNewProject,
  onOpenProject,
  onSave,
  showSaved,
  playing,
  loopEnabled,
  onPlayingChange,
  onLoopChange,
}: TopBarProps) {
  return (
    <div className={styles.topBar}>
      <div className={styles.projectInfo}>
        <span className={styles.projectName}>{projectMeta?.name ?? "MegaDAW"}</span>
        {projectMeta && (
          <>
            <span className={styles.separator}>|</span>
            <span className={styles.detail}>{projectMeta.tempo} BPM</span>
            <span className={styles.detail}>
              {projectMeta.timeSignature[0]}/{projectMeta.timeSignature[1]}
            </span>
            <span className={styles.driverBadge}>Flamedriver</span>
          </>
        )}
      </div>
      <div className={styles.actions}>
        <button className={styles.btn} onClick={onNewProject}>New</button>
        <button className={styles.btn} onClick={onOpenProject}>Open</button>
        {projectMeta && (
          <button className={styles.btn} onClick={onSave}>Save</button>
        )}
        {showSaved && <span className={styles.saved}>Saved</span>}
      </div>
      {projectMeta ? (
        <TransportControls
          projectMeta={projectMeta}
          playing={playing}
          loopEnabled={loopEnabled}
          onPlayingChange={onPlayingChange}
          onLoopChange={onLoopChange}
        />
      ) : (
        <div className={styles.transport}>
          <button className={styles.transportBtn} disabled>&#9654;</button>
          <button className={styles.transportBtn} disabled>&#9632;</button>
          <button className={styles.transportBtn} disabled>&#8635;</button>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 5: Update App.tsx for transport state + keyboard shortcuts**

In `src/App.tsx`, add `playing` and `loopEnabled` state, pass to TopBar, add Space/L/Home keyboard handlers:

Add state:
```typescript
  const [playing, setPlaying] = useState(false);
  const [loopEnabled, setLoopEnabled] = useState(false);
  const [selectedRegion, setSelectedRegion] = useState<SelectedRegion | null>(null);
```

Add import for `SelectedRegion` from `"./types/model"`.

Update the keyboard handler to include Space/L/Home:
```typescript
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if ((e.ctrlKey || e.metaKey) && e.key === "s") {
        e.preventDefault();
        handleSave();
      }
      if (e.key === " " && projectMeta) {
        e.preventDefault();
        if (playing) {
          ipc.transportStop();
          setPlaying(false);
        } else {
          ipc.transportPlay();
          setPlaying(true);
        }
      }
      if (e.key === "l" && projectMeta && !e.ctrlKey && !e.metaKey) {
        if (loopEnabled) {
          ipc.transportClearLoop();
          setLoopEnabled(false);
        } else {
          const ticksPerBar = projectMeta.ticksPerBeat * projectMeta.timeSignature[0];
          ipc.transportSetLoop(0, ticksPerBar * 4);
          setLoopEnabled(true);
        }
      }
      if (e.key === "Home" && projectMeta) {
        ipc.transportSeek(0);
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleSave, playing, loopEnabled, projectMeta]);
```

Pass the new props to TopBar:
```typescript
      <TopBar
        projectMeta={projectMeta}
        onNewProject={() => setShowNewProject(true)}
        onOpenProject={handleOpenProject}
        onSave={handleSave}
        showSaved={showSaved}
        playing={playing}
        loopEnabled={loopEnabled}
        onPlayingChange={setPlaying}
        onLoopChange={setLoopEnabled}
      />
```

Stop playback when project closes/changes:
In `handleOpenProject`, add `setPlaying(false)` before opening.
In `handleProjectCreated`, add `setPlaying(false)`.

- [ ] **Step 6: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 7: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src/hooks/ src/components/TransportControls.tsx src/components/TransportControls.module.css src/components/TopBar.tsx src/App.tsx
git commit -m "feat(ui): transport controls — play/stop/loop, position display, keyboard shortcuts"
```

---

### Task 6: Arrangement View — Zoom Hook + Track Headers + Layout

**Files:**
- Create: `src/hooks/useArrangementZoom.ts`
- Create: `src/components/ArrangementView.tsx` + `.module.css`
- Create: `src/components/TrackHeader.tsx` + `.module.css`
- Create: `src/components/AddTrackDialog.tsx` + `.module.css`
- Modify: `src/components/MainArea.tsx`

- [ ] **Step 1: Create useArrangementZoom hook**

Create `src/hooks/useArrangementZoom.ts`:

```typescript
import { useState, useCallback, useRef } from "react";

interface ZoomState {
  ticksPerPixel: number;
  scrollLeft: number;
  setScrollLeft: (v: number) => void;
  handleWheel: (e: React.WheelEvent) => void;
  tickToPixel: (tick: number) => number;
  pixelToTick: (px: number) => number;
}

export function useArrangementZoom(ticksPerBeat: number): ZoomState {
  const ticksPerBar = ticksPerBeat * 4;
  const defaultTicksPerPixel = (ticksPerBar * 16) / 1200;
  const [ticksPerPixel, setTicksPerPixel] = useState(defaultTicksPerPixel);
  const [scrollLeft, setScrollLeft] = useState(0);

  const handleWheel = useCallback(
    (e: React.WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
        const zoomFactor = e.deltaY > 0 ? 1.15 : 0.87;
        setTicksPerPixel((prev) => {
          const next = prev * zoomFactor;
          return Math.max(0.05, Math.min(next, ticksPerBar));
        });
      } else if (e.shiftKey) {
        setScrollLeft((prev) => Math.max(0, prev + e.deltaY));
      } else {
        setScrollLeft((prev) => Math.max(0, prev + e.deltaX));
      }
    },
    [ticksPerBar],
  );

  const tickToPixel = useCallback(
    (tick: number) => tick / ticksPerPixel - scrollLeft,
    [ticksPerPixel, scrollLeft],
  );

  const pixelToTick = useCallback(
    (px: number) => (px + scrollLeft) * ticksPerPixel,
    [ticksPerPixel, scrollLeft],
  );

  return { ticksPerPixel, scrollLeft, setScrollLeft, handleWheel, tickToPixel, pixelToTick };
}
```

- [ ] **Step 2: Create TrackHeader component**

Create `src/components/TrackHeader.tsx`:

```typescript
import { useState } from "react";
import type { Track, FmInstrument, PsgInstrument, DacInstrument } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./TrackHeader.module.css";

interface TrackHeaderProps {
  track: Track;
  fmInstruments: FmInstrument[];
  psgInstruments: PsgInstrument[];
  dacInstruments: DacInstrument[];
  onUpdate: () => void;
  onDelete: () => void;
}

function channelColor(track: Track): string {
  if ("Fm" in track.channel) return "var(--accent-fm)";
  if ("Psg" in track.channel || track.channel === "PsgNoise") return "var(--accent-psg)";
  return "var(--accent-dac)";
}

function channelLabel(track: Track): string {
  if ("Fm" in track.channel) return `FM${(track.channel as { Fm: number }).Fm + 1}`;
  if ("Psg" in track.channel) return `PSG${(track.channel as { Psg: number }).Psg + 1}`;
  if (track.channel === "PsgNoise") return "Noise";
  if ("Dac" in track.channel) return "DAC";
  return "?";
}

function channelType(track: Track): "fm" | "psg" | "dac" {
  if ("Fm" in track.channel) return "fm";
  if ("Psg" in track.channel || track.channel === "PsgNoise") return "psg";
  return "dac";
}

export function TrackHeader({
  track,
  fmInstruments,
  psgInstruments,
  dacInstruments,
  onUpdate,
  onDelete,
}: TrackHeaderProps) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(track.name);

  const ct = channelType(track);
  const instruments =
    ct === "fm" ? fmInstruments :
    ct === "psg" ? psgInstruments :
    dacInstruments;

  async function toggleMute() {
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      !track.muted, track.solo, track.volume, track.pan,
    );
    onUpdate();
  }

  async function toggleSolo() {
    await ipc.updateTrack(
      track.id, track.name, track.channel, track.instrumentId,
      track.muted, !track.solo, track.volume, track.pan,
    );
    onUpdate();
  }

  async function commitRename() {
    setEditing(false);
    if (name.trim() && name !== track.name) {
      await ipc.updateTrack(
        track.id, name.trim(), track.channel, track.instrumentId,
        track.muted, track.solo, track.volume, track.pan,
      );
      onUpdate();
    }
  }

  async function changeInstrument(instId: string) {
    await ipc.updateTrack(
      track.id, track.name, track.channel, instId || null,
      track.muted, track.solo, track.volume, track.pan,
    );
    onUpdate();
  }

  return (
    <div className={styles.header} onContextMenu={(e) => { e.preventDefault(); onDelete(); }}>
      <div className={styles.top}>
        <span className={styles.badge} style={{ background: channelColor(track) }}>
          {channelLabel(track)}
        </span>
        {editing ? (
          <input
            className={styles.nameInput}
            value={name}
            onChange={(e) => setName(e.target.value)}
            onBlur={commitRename}
            onKeyDown={(e) => e.key === "Enter" && commitRename()}
            autoFocus
          />
        ) : (
          <span className={styles.name} onDoubleClick={() => setEditing(true)}>
            {track.name}
          </span>
        )}
      </div>
      <div className={styles.controls}>
        <button
          className={`${styles.muteBtn} ${track.muted ? styles.active : ""}`}
          onClick={toggleMute}
        >
          M
        </button>
        <button
          className={`${styles.soloBtn} ${track.solo ? styles.active : ""}`}
          onClick={toggleSolo}
        >
          S
        </button>
        <select
          className={styles.instSelect}
          value={track.instrumentId ?? ""}
          onChange={(e) => changeInstrument(e.target.value)}
        >
          <option value="">-- None --</option>
          {instruments.map((inst) => (
            <option key={inst.id} value={inst.id}>{inst.name}</option>
          ))}
        </select>
      </div>
    </div>
  );
}
```

- [ ] **Step 3: Create TrackHeader CSS**

Create `src/components/TrackHeader.module.css`:

```css
.header {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 6px 8px;
  border-bottom: 1px solid var(--border);
  height: 60px;
  justify-content: center;
}

.top {
  display: flex;
  align-items: center;
  gap: 6px;
}

.badge {
  font-size: 10px;
  padding: 1px 5px;
  border-radius: 3px;
  color: #fff;
  font-weight: 600;
  flex-shrink: 0;
}

.name {
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  cursor: default;
}

.nameInput {
  font-size: 12px;
  padding: 1px 4px;
  width: 100%;
  background: var(--bg-input);
  color: var(--text-primary);
  border: 1px solid var(--border-focus);
  border-radius: 2px;
}

.controls {
  display: flex;
  align-items: center;
  gap: 4px;
}

.muteBtn, .soloBtn {
  width: 20px;
  height: 18px;
  font-size: 10px;
  font-weight: 600;
  background: var(--bg-surface);
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 2px;
  padding: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.muteBtn.active {
  background: var(--error);
  color: #fff;
  border-color: var(--error);
}

.soloBtn.active {
  background: var(--carrier-highlight);
  color: #000;
  border-color: var(--carrier-highlight);
}

.instSelect {
  flex: 1;
  font-size: 11px;
  min-width: 0;
  padding: 1px 4px;
}
```

- [ ] **Step 4: Create AddTrackDialog**

Create `src/components/AddTrackDialog.tsx`:

```typescript
import { useState, useEffect } from "react";
import type { ChannelLayout, ChannelAssignment } from "../types/model";
import * as ipc from "../api/ipc";
import styles from "./AddTrackDialog.module.css";

interface AddTrackDialogProps {
  driverId: string;
  onClose: () => void;
  onCreated: () => void;
}

export function AddTrackDialog({ driverId, onClose, onCreated }: AddTrackDialogProps) {
  const [layout, setLayout] = useState<ChannelLayout | null>(null);
  const [name, setName] = useState("");
  const [channelKey, setChannelKey] = useState("fm_0");

  useEffect(() => {
    ipc.getDriverInfo(driverId).then((d) => {
      setLayout(d.layout);
    });
  }, [driverId]);

  function parseChannel(key: string): ChannelAssignment {
    if (key === "psg_noise") return "PsgNoise";
    const [type_, idx] = key.split("_");
    const n = parseInt(idx);
    if (type_ === "fm") return { Fm: n };
    if (type_ === "psg") return { Psg: n };
    return { Dac: n };
  }

  function suggestName(key: string): string {
    const ch = parseChannel(key);
    if (ch === "PsgNoise") return "PSG Noise - Untitled";
    if ("Fm" in ch) return `FM${ch.Fm + 1} - Untitled`;
    if ("Psg" in ch) return `PSG${ch.Psg + 1} - Untitled`;
    return "DAC - Untitled";
  }

  async function handleCreate() {
    const trackName = name.trim() || suggestName(channelKey);
    const channel = parseChannel(channelKey);
    await ipc.addTrack(trackName, channel, null);
    onCreated();
  }

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.dialog} onClick={(e) => e.stopPropagation()}>
        <h3 className={styles.title}>Add Track</h3>
        <label className={styles.label}>
          Name
          <input
            className={styles.input}
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder={suggestName(channelKey)}
            autoFocus
          />
        </label>
        <label className={styles.label}>
          Channel
          <select
            className={styles.select}
            value={channelKey}
            onChange={(e) => setChannelKey(e.target.value)}
          >
            {layout && (
              <>
                <optgroup label="FM">
                  {layout.fmChannels.map((ch) => (
                    <option key={`fm_${ch.index}`} value={`fm_${ch.index}`}>{ch.name}</option>
                  ))}
                </optgroup>
                <optgroup label="PSG">
                  {layout.psgChannels.map((ch) => (
                    <option
                      key={ch.isNoise ? "psg_noise" : `psg_${ch.index}`}
                      value={ch.isNoise ? "psg_noise" : `psg_${ch.index}`}
                    >
                      {ch.name}
                    </option>
                  ))}
                </optgroup>
                <optgroup label="DAC">
                  {layout.dacChannels.map((ch) => (
                    <option key={`dac_${ch.index}`} value={`dac_${ch.index}`}>{ch.name}</option>
                  ))}
                </optgroup>
              </>
            )}
          </select>
        </label>
        <div className={styles.buttons}>
          <button className={styles.cancelBtn} onClick={onClose}>Cancel</button>
          <button className={styles.createBtn} onClick={handleCreate}>Create</button>
        </div>
      </div>
    </div>
  );
}
```

- [ ] **Step 5: Create AddTrackDialog CSS**

Create `src/components/AddTrackDialog.module.css`:

```css
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 300;
}

.dialog {
  background: var(--bg-panel);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 20px;
  width: 340px;
  max-width: 90vw;
}

.title {
  margin: 0 0 14px;
  font-size: 16px;
  font-weight: 500;
}

.label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 12px;
  font-size: 12px;
  color: var(--text-secondary);
}

.input {
  padding: 6px 8px;
  font-size: 13px;
}

.select {
  padding: 6px 8px;
  font-size: 13px;
}

.buttons {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 16px;
}

.cancelBtn {
  padding: 6px 14px;
  background: none;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 3px;
}

.createBtn {
  padding: 6px 16px;
  background: var(--accent-fm);
  color: #fff;
  border: none;
  border-radius: 3px;
  font-weight: 500;
}
```

- [ ] **Step 6: Create ArrangementView component (layout shell)**

Create `src/components/ArrangementView.tsx`:

```typescript
import { useState, useEffect, useCallback } from "react";
import type { Track, SongMetadata, FmInstrument, PsgInstrument, DacInstrument, SelectedRegion } from "../types/model";
import { useArrangementZoom } from "../hooks/useArrangementZoom";
import { usePlaybackPosition } from "../hooks/usePlaybackPosition";
import { TrackHeader } from "./TrackHeader";
import { TimelineRuler } from "./TimelineRuler";
import { TimelineCanvas } from "./TimelineCanvas";
import { AddTrackDialog } from "./AddTrackDialog";
import * as ipc from "../api/ipc";
import styles from "./ArrangementView.module.css";

interface ArrangementViewProps {
  projectMeta: SongMetadata;
  playing: boolean;
  onSelectRegion: (region: SelectedRegion | null) => void;
  selectedRegion: SelectedRegion | null;
}

export function ArrangementView({ projectMeta, playing, onSelectRegion, selectedRegion }: ArrangementViewProps) {
  const [tracks, setTracks] = useState<Track[]>([]);
  const [fmInstruments, setFmInstruments] = useState<FmInstrument[]>([]);
  const [psgInstruments, setPsgInstruments] = useState<PsgInstrument[]>([]);
  const [dacInstruments, setDacInstruments] = useState<DacInstrument[]>([]);
  const [showAddTrack, setShowAddTrack] = useState(false);
  const zoom = useArrangementZoom(projectMeta.ticksPerBeat);
  const { interpolatedTick } = usePlaybackPosition(playing, projectMeta.tempo, projectMeta.ticksPerBeat);
  const trackHeight = 60;

  const refresh = useCallback(async () => {
    const [t, fm, psg, dac] = await Promise.all([
      ipc.listTracks(),
      ipc.listFmInstruments(),
      ipc.listPsgInstruments(),
      ipc.listDacInstruments(),
    ]);
    setTracks(t);
    setFmInstruments(fm);
    setPsgInstruments(psg);
    setDacInstruments(dac);
  }, []);

  useEffect(() => { refresh(); }, [refresh]);

  async function handleDeleteTrack(id: string) {
    await ipc.deleteTrack(id);
    refresh();
  }

  function handleRegionDoubleClick(trackId: string, regionId: string) {
    const track = tracks.find((t) => t.id === trackId);
    if (!track) return;
    const region = track.regions.find((r) => r.id === regionId);
    if (!region) return;
    const ct = "Fm" in track.channel ? "fm" as const :
               "Psg" in track.channel || track.channel === "PsgNoise" ? "psg" as const : "dac" as const;
    onSelectRegion({
      trackId,
      trackName: track.name,
      regionId,
      channelType: ct,
      startTick: region.startTick,
      durationTicks: region.durationTicks,
    });
  }

  async function handleCreateRegion(trackId: string, startTick: number) {
    const ticksPerBar = projectMeta.ticksPerBeat * projectMeta.timeSignature[0];
    const snapped = Math.floor(startTick / ticksPerBar) * ticksPerBar;
    await ipc.addRegion(trackId, snapped, ticksPerBar);
    refresh();
  }

  async function handleSeek(tick: number) {
    await ipc.transportSeek(tick);
  }

  return (
    <div className={styles.arrangement} onWheel={zoom.handleWheel}>
      <div className={styles.rulerRow}>
        <div className={styles.headerSpacer} />
        <TimelineRuler
          ticksPerPixel={zoom.ticksPerPixel}
          scrollLeft={zoom.scrollLeft}
          ticksPerBeat={projectMeta.ticksPerBeat}
          beatsPerBar={projectMeta.timeSignature[0]}
          onSeek={handleSeek}
        />
      </div>
      <div className={styles.body}>
        <div className={styles.headers}>
          {tracks.map((track) => (
            <TrackHeader
              key={track.id}
              track={track}
              fmInstruments={fmInstruments}
              psgInstruments={psgInstruments}
              dacInstruments={dacInstruments}
              onUpdate={refresh}
              onDelete={() => handleDeleteTrack(track.id)}
            />
          ))}
          <button className={styles.addTrackBtn} onClick={() => setShowAddTrack(true)}>
            + Add Track
          </button>
        </div>
        <TimelineCanvas
          tracks={tracks}
          ticksPerPixel={zoom.ticksPerPixel}
          scrollLeft={zoom.scrollLeft}
          trackHeight={trackHeight}
          playbackTick={playing ? interpolatedTick : 0}
          playing={playing}
          selectedRegion={selectedRegion}
          onRegionClick={(trackId, regionId) => {
            const track = tracks.find((t) => t.id === trackId);
            if (!track) return;
            const region = track.regions.find((r) => r.id === regionId);
            if (!region) return;
            const ct = "Fm" in track.channel ? "fm" as const :
                       "Psg" in track.channel || track.channel === "PsgNoise" ? "psg" as const : "dac" as const;
            onSelectRegion({
              trackId, trackName: track.name, regionId, channelType: ct,
              startTick: region.startTick, durationTicks: region.durationTicks,
            });
          }}
          onRegionDoubleClick={handleRegionDoubleClick}
          onEmptyDoubleClick={handleCreateRegion}
        />
      </div>
      {showAddTrack && (
        <AddTrackDialog
          driverId={projectMeta.driverId}
          onClose={() => setShowAddTrack(false)}
          onCreated={() => { setShowAddTrack(false); refresh(); }}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 7: Create ArrangementView CSS**

Create `src/components/ArrangementView.module.css`:

```css
.arrangement {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}

.rulerRow {
  display: flex;
  flex-shrink: 0;
  height: 28px;
  border-bottom: 1px solid var(--border);
}

.headerSpacer {
  width: 180px;
  flex-shrink: 0;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
}

.body {
  display: flex;
  flex: 1;
  overflow: hidden;
}

.headers {
  width: 180px;
  flex-shrink: 0;
  overflow-y: auto;
  background: var(--bg-panel);
  border-right: 1px solid var(--border);
}

.addTrackBtn {
  width: 100%;
  padding: 8px;
  background: none;
  color: var(--text-secondary);
  border: none;
  border-bottom: 1px solid var(--border);
  text-align: left;
  font-size: 12px;
}

.addTrackBtn:hover {
  color: var(--text-primary);
  background: var(--bg-surface);
}
```

- [ ] **Step 8: Update MainArea to render ArrangementView**

Replace `src/components/MainArea.tsx`:

```typescript
import type { SongMetadata, SelectedRegion } from "../types/model";
import { ArrangementView } from "./ArrangementView";
import styles from "./MainArea.module.css";

interface MainAreaProps {
  projectOpen: boolean;
  projectMeta: SongMetadata | null;
  playing: boolean;
  onNewProject: () => void;
  onOpenProject: () => void;
  onSelectRegion: (region: SelectedRegion | null) => void;
  selectedRegion: SelectedRegion | null;
}

export function MainArea({
  projectOpen,
  projectMeta,
  playing,
  onNewProject,
  onOpenProject,
  onSelectRegion,
  selectedRegion,
}: MainAreaProps) {
  if (!projectOpen || !projectMeta) {
    return (
      <div className={styles.welcome}>
        <h1 className={styles.title}>MegaDAW</h1>
        <p className={styles.subtitle}>Mega Drive Digital Audio Workstation</p>
        <div className={styles.welcomeActions}>
          <button className={styles.welcomeBtn} onClick={onNewProject}>New Project</button>
          <button className={styles.welcomeBtn} onClick={onOpenProject}>Open Project</button>
        </div>
      </div>
    );
  }

  return (
    <ArrangementView
      projectMeta={projectMeta}
      playing={playing}
      onSelectRegion={onSelectRegion}
      selectedRegion={selectedRegion}
    />
  );
}
```

Update `App.tsx` to pass the new props to `MainArea`:

```typescript
        <MainArea
          projectOpen={projectOpen}
          projectMeta={projectMeta}
          playing={playing}
          onNewProject={() => setShowNewProject(true)}
          onOpenProject={handleOpenProject}
          onSelectRegion={setSelectedRegion}
          selectedRegion={selectedRegion}
        />
```

- [ ] **Step 9: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: Errors about missing `TimelineRuler` and `TimelineCanvas` (expected — created in next task). Create stub files:

Create `src/components/TimelineRuler.tsx`:
```typescript
import styles from "./TimelineRuler.module.css";
interface TimelineRulerProps { ticksPerPixel: number; scrollLeft: number; ticksPerBeat: number; beatsPerBar: number; onSeek: (tick: number) => void; }
export function TimelineRuler(_props: TimelineRulerProps) { return <canvas className={styles.ruler} />; }
```

Create `src/components/TimelineRuler.module.css`:
```css
.ruler { width: 100%; height: 100%; }
```

Create `src/components/TimelineCanvas.tsx`:
```typescript
import type { Track, SelectedRegion } from "../types/model";
import styles from "./TimelineCanvas.module.css";
interface TimelineCanvasProps { tracks: Track[]; ticksPerPixel: number; scrollLeft: number; trackHeight: number; playbackTick: number; playing: boolean; selectedRegion: SelectedRegion | null; onRegionClick: (trackId: string, regionId: string) => void; onRegionDoubleClick: (trackId: string, regionId: string) => void; onEmptyDoubleClick: (trackId: string, startTick: number) => void; }
export function TimelineCanvas(_props: TimelineCanvasProps) { return <canvas className={styles.canvas} />; }
```

Create `src/components/TimelineCanvas.module.css`:
```css
.canvas { width: 100%; height: 100%; }
```

- [ ] **Step 10: Verify TypeScript compiles with stubs**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 11: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src/hooks/ src/components/ src/App.tsx
git commit -m "feat(ui): arrangement view layout — track headers, zoom hook, add track dialog"
```

---

### Task 7: Timeline Ruler + Timeline Canvas

**Files:**
- Modify: `src/components/TimelineRuler.tsx` (replace stub with full implementation)
- Modify: `src/components/TimelineRuler.module.css`
- Modify: `src/components/TimelineCanvas.tsx` (replace stub with full implementation)
- Modify: `src/components/TimelineCanvas.module.css`

- [ ] **Step 1: Implement TimelineRuler**

Replace `src/components/TimelineRuler.tsx`:

```typescript
import { useRef, useEffect, useCallback } from "react";
import styles from "./TimelineRuler.module.css";

interface TimelineRulerProps {
  ticksPerPixel: number;
  scrollLeft: number;
  ticksPerBeat: number;
  beatsPerBar: number;
  onSeek: (tick: number) => void;
}

export function TimelineRuler({ ticksPerPixel, scrollLeft, ticksPerBeat, beatsPerBar, onSeek }: TimelineRulerProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = rect.height * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${rect.height}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = rect.height;
    ctx.clearRect(0, 0, w, h);

    const ticksPerBar = ticksPerBeat * beatsPerBar;
    const startTick = scrollLeft * ticksPerPixel;
    const endTick = startTick + w * ticksPerPixel;

    const firstBar = Math.floor(startTick / ticksPerBar);
    const lastBar = Math.ceil(endTick / ticksPerBar);

    ctx.fillStyle = "#888888";
    ctx.font = "10px sans-serif";

    for (let bar = firstBar; bar <= lastBar; bar++) {
      const tick = bar * ticksPerBar;
      const x = (tick - startTick) / ticksPerPixel;

      ctx.strokeStyle = "#555555";
      ctx.beginPath();
      ctx.moveTo(x, h - 8);
      ctx.lineTo(x, h);
      ctx.stroke();

      ctx.fillText(`${bar + 1}`, x + 3, 12);

      if (ticksPerPixel < ticksPerBeat) {
        for (let beat = 1; beat < beatsPerBar; beat++) {
          const bx = ((tick + beat * ticksPerBeat) - startTick) / ticksPerPixel;
          ctx.strokeStyle = "#3a3a3a";
          ctx.beginPath();
          ctx.moveTo(bx, h - 4);
          ctx.lineTo(bx, h);
          ctx.stroke();
        }
      }
    }
  }, [ticksPerPixel, scrollLeft, ticksPerBeat, beatsPerBar]);

  useEffect(() => {
    draw();
    const obs = new ResizeObserver(draw);
    if (containerRef.current) obs.observe(containerRef.current);
    return () => obs.disconnect();
  }, [draw]);

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const tick = (x + scrollLeft) * ticksPerPixel;
    onSeek(Math.max(0, Math.round(tick)));
  }

  return (
    <div ref={containerRef} className={styles.rulerContainer} onClick={handleClick}>
      <canvas ref={canvasRef} className={styles.ruler} />
    </div>
  );
}
```

Update `src/components/TimelineRuler.module.css`:

```css
.rulerContainer {
  flex: 1;
  overflow: hidden;
  cursor: pointer;
  background: var(--bg-panel);
}

.ruler {
  display: block;
}
```

- [ ] **Step 2: Implement TimelineCanvas**

Replace `src/components/TimelineCanvas.tsx`:

```typescript
import { useRef, useEffect, useCallback } from "react";
import type { Track, SelectedRegion } from "../types/model";
import styles from "./TimelineCanvas.module.css";

interface TimelineCanvasProps {
  tracks: Track[];
  ticksPerPixel: number;
  scrollLeft: number;
  trackHeight: number;
  playbackTick: number;
  playing: boolean;
  selectedRegion: SelectedRegion | null;
  onRegionClick: (trackId: string, regionId: string) => void;
  onRegionDoubleClick: (trackId: string, regionId: string) => void;
  onEmptyDoubleClick: (trackId: string, startTick: number) => void;
}

const CHANNEL_COLORS: Record<string, string> = {
  fm: "#4a9eff",
  psg: "#44cc66",
  dac: "#ff8844",
};

function trackChannelType(track: Track): string {
  if ("Fm" in track.channel) return "fm";
  if ("Psg" in track.channel || track.channel === "PsgNoise") return "psg";
  return "dac";
}

export function TimelineCanvas({
  tracks,
  ticksPerPixel,
  scrollLeft,
  trackHeight,
  playbackTick,
  playing,
  selectedRegion,
  onRegionClick,
  onRegionDoubleClick,
  onEmptyDoubleClick,
}: TimelineCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);
  const animRef = useRef(0);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    const container = containerRef.current;
    if (!canvas || !container) return;

    const rect = container.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    canvas.width = rect.width * dpr;
    canvas.height = Math.max(rect.height, tracks.length * trackHeight) * dpr;
    canvas.style.width = `${rect.width}px`;
    canvas.style.height = `${Math.max(rect.height, tracks.length * trackHeight)}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    const w = rect.width;
    const h = Math.max(rect.height, tracks.length * trackHeight);
    const startTick = scrollLeft * ticksPerPixel;

    ctx.clearRect(0, 0, w, h);

    // Track row backgrounds
    for (let i = 0; i < tracks.length; i++) {
      ctx.fillStyle = i % 2 === 0 ? "#1e1e1e" : "#222222";
      ctx.fillRect(0, i * trackHeight, w, trackHeight);
      ctx.strokeStyle = "#2a2a2a";
      ctx.beginPath();
      ctx.moveTo(0, (i + 1) * trackHeight);
      ctx.lineTo(w, (i + 1) * trackHeight);
      ctx.stroke();
    }

    // Regions
    for (let i = 0; i < tracks.length; i++) {
      const track = tracks[i];
      const color = CHANNEL_COLORS[trackChannelType(track)] || "#888";
      const y = i * trackHeight + 2;
      const rh = trackHeight - 4;

      for (const region of track.regions) {
        const x = (region.startTick - startTick) / ticksPerPixel;
        const rw = region.durationTicks / ticksPerPixel;

        if (x + rw < 0 || x > w) continue;

        ctx.fillStyle = color + "33";
        ctx.strokeStyle = color;
        ctx.lineWidth = 1;

        const rx = Math.round(x);
        const rrw = Math.round(rw);
        ctx.fillRect(rx, y, rrw, rh);
        ctx.strokeRect(rx + 0.5, y + 0.5, rrw - 1, rh - 1);

        // Selected highlight
        if (selectedRegion?.trackId === track.id && selectedRegion?.regionId === region.id) {
          ctx.strokeStyle = "#ffffff";
          ctx.lineWidth = 2;
          ctx.strokeRect(rx + 1, y + 1, rrw - 2, rh - 2);
        }

        // Note previews
        if (ticksPerPixel < 4 && region.notes.length > 0) {
          ctx.fillStyle = color + "88";
          for (const note of region.notes) {
            const nx = rx + note.tick / ticksPerPixel;
            const nw = Math.max(1, note.durationTicks / ticksPerPixel);
            const noteY = y + rh - ((note.pitch - 24) / 96) * rh;
            ctx.fillRect(nx, noteY, nw, 2);
          }
        }
      }
    }

    // Playback cursor
    if (playing) {
      const cx = (playbackTick - startTick) / ticksPerPixel;
      if (cx >= 0 && cx <= w) {
        ctx.strokeStyle = "#ffffff";
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(cx, 0);
        ctx.lineTo(cx, h);
        ctx.stroke();
      }
    }
  }, [tracks, ticksPerPixel, scrollLeft, trackHeight, playbackTick, playing, selectedRegion]);

  useEffect(() => {
    function animate() {
      draw();
      if (playing) {
        animRef.current = requestAnimationFrame(animate);
      }
    }
    draw();
    if (playing) {
      animRef.current = requestAnimationFrame(animate);
    }
    return () => cancelAnimationFrame(animRef.current);
  }, [draw, playing]);

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const trackIdx = Math.floor(y / trackHeight);
    if (trackIdx < 0 || trackIdx >= tracks.length) return;

    const tick = (x + scrollLeft) * ticksPerPixel;
    const track = tracks[trackIdx];

    for (const region of track.regions) {
      const rx = (region.startTick - scrollLeft * ticksPerPixel) / ticksPerPixel;
      const rw = region.durationTicks / ticksPerPixel;
      if (x >= rx && x <= rx + rw) {
        onRegionClick(track.id, region.id);
        return;
      }
    }
  }

  function handleDoubleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const trackIdx = Math.floor(y / trackHeight);
    if (trackIdx < 0 || trackIdx >= tracks.length) return;

    const startTick = scrollLeft * ticksPerPixel;
    const tick = x * ticksPerPixel + startTick;
    const track = tracks[trackIdx];

    for (const region of track.regions) {
      const rx = (region.startTick - startTick) / ticksPerPixel;
      const rw = region.durationTicks / ticksPerPixel;
      if (x >= rx && x <= rx + rw) {
        onRegionDoubleClick(track.id, region.id);
        return;
      }
    }

    onEmptyDoubleClick(track.id, tick);
  }

  return (
    <div ref={containerRef} className={styles.container}>
      <canvas
        ref={canvasRef}
        className={styles.canvas}
        onClick={handleClick}
        onDoubleClick={handleDoubleClick}
      />
    </div>
  );
}
```

Update `src/components/TimelineCanvas.module.css`:

```css
.container {
  flex: 1;
  overflow-y: auto;
  overflow-x: hidden;
  background: var(--bg-app);
}

.canvas {
  display: block;
}
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src/components/TimelineRuler.tsx src/components/TimelineRuler.module.css src/components/TimelineCanvas.tsx src/components/TimelineCanvas.module.css
git commit -m "feat(ui): timeline ruler + canvas — regions, grid, playback cursor, note previews"
```

---

### Task 8: Piano Roll — Canvas, Keys, Velocity Lane

**Files:**
- Create: `src/components/PianoRoll.tsx` + `.module.css`
- Create: `src/components/PianoRollCanvas.tsx` + `.module.css`
- Create: `src/components/PianoRollKeys.tsx` + `.module.css`
- Create: `src/components/VelocityLane.tsx` + `.module.css`

- [ ] **Step 1: Create PianoRollKeys**

Create `src/components/PianoRollKeys.tsx`:

```typescript
import { useRef, useEffect, useCallback } from "react";
import styles from "./PianoRollKeys.module.css";

interface PianoRollKeysProps {
  minPitch: number;
  maxPitch: number;
  rowHeight: number;
  scrollTop: number;
  onAudition: (pitch: number) => void;
}

const NOTE_NAMES = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];

function isBlackKey(pitch: number): boolean {
  return [1, 3, 6, 8, 10].includes(pitch % 12);
}

export function PianoRollKeys({ minPitch, maxPitch, rowHeight, scrollTop, onAudition }: PianoRollKeysProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const totalNotes = maxPitch - minPitch + 1;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    const w = 48;
    const h = totalNotes * rowHeight;
    canvas.width = w * dpr;
    canvas.height = h * dpr;
    canvas.style.width = `${w}px`;
    canvas.style.height = `${h}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    for (let i = 0; i < totalNotes; i++) {
      const pitch = maxPitch - i;
      const y = i * rowHeight;
      const black = isBlackKey(pitch);

      ctx.fillStyle = black ? "#1a1a1a" : "#2a2a2a";
      ctx.fillRect(0, y, w, rowHeight);
      ctx.strokeStyle = "#333";
      ctx.beginPath();
      ctx.moveTo(0, y + rowHeight);
      ctx.lineTo(w, y + rowHeight);
      ctx.stroke();

      const octave = Math.floor(pitch / 12) - 1;
      const name = NOTE_NAMES[pitch % 12];
      ctx.fillStyle = black ? "#666" : "#999";
      ctx.font = "9px sans-serif";
      ctx.fillText(`${name}${octave}`, 4, y + rowHeight - 3);
    }
  }, [totalNotes, rowHeight, maxPitch]);

  useEffect(() => { draw(); }, [draw]);

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const y = e.clientY - rect.top;
    const idx = Math.floor(y / rowHeight);
    const pitch = maxPitch - idx;
    if (pitch >= minPitch && pitch <= maxPitch) {
      onAudition(pitch);
    }
  }

  return (
    <div ref={containerRef} className={styles.keys} style={{ marginTop: -scrollTop }}>
      <canvas ref={canvasRef} onClick={handleClick} />
    </div>
  );
}
```

Create `src/components/PianoRollKeys.module.css`:
```css
.keys {
  width: 48px;
  flex-shrink: 0;
  overflow: hidden;
}
```

- [ ] **Step 2: Create PianoRollCanvas**

Create `src/components/PianoRollCanvas.tsx`:

```typescript
import { useRef, useEffect, useCallback } from "react";
import type { Note } from "../types/model";
import styles from "./PianoRollCanvas.module.css";

interface PianoRollCanvasProps {
  notes: Note[];
  minPitch: number;
  maxPitch: number;
  durationTicks: number;
  ticksPerPixel: number;
  rowHeight: number;
  gridSnapTicks: number;
  channelColor: string;
  selectedNotes: Set<number>;
  onNoteClick: (index: number) => void;
  onNoteAdd: (tick: number, pitch: number) => void;
  onScrollTopChange: (scrollTop: number) => void;
}

function isBlackKey(pitch: number): boolean {
  return [1, 3, 6, 8, 10].includes(pitch % 12);
}

export function PianoRollCanvas({
  notes,
  minPitch,
  maxPitch,
  durationTicks,
  ticksPerPixel,
  rowHeight,
  gridSnapTicks,
  channelColor,
  selectedNotes,
  onNoteClick,
  onNoteAdd,
  onScrollTopChange,
}: PianoRollCanvasProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  const totalNotes = maxPitch - minPitch + 1;
  const canvasWidth = durationTicks / ticksPerPixel;
  const canvasHeight = totalNotes * rowHeight;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = canvasWidth * dpr;
    canvas.height = canvasHeight * dpr;
    canvas.style.width = `${canvasWidth}px`;
    canvas.style.height = `${canvasHeight}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    // Row backgrounds
    for (let i = 0; i < totalNotes; i++) {
      const pitch = maxPitch - i;
      const y = i * rowHeight;
      ctx.fillStyle = isBlackKey(pitch) ? "#1a1a1a" : "#1e1e1e";
      ctx.fillRect(0, y, canvasWidth, rowHeight);
      ctx.strokeStyle = "#2a2a2a";
      ctx.beginPath();
      ctx.moveTo(0, y + rowHeight);
      ctx.lineTo(canvasWidth, y + rowHeight);
      ctx.stroke();
    }

    // Grid lines
    const gridPx = gridSnapTicks / ticksPerPixel;
    if (gridPx > 4) {
      ctx.strokeStyle = "#2a2a2a";
      for (let x = 0; x < canvasWidth; x += gridPx) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, canvasHeight);
        ctx.stroke();
      }
    }

    // Notes
    for (let i = 0; i < notes.length; i++) {
      const note = notes[i];
      if (note.pitch < minPitch || note.pitch > maxPitch) continue;
      const x = note.tick / ticksPerPixel;
      const w = Math.max(2, note.durationTicks / ticksPerPixel);
      const row = maxPitch - note.pitch;
      const y = row * rowHeight + 1;
      const h = rowHeight - 2;

      const selected = selectedNotes.has(i);
      ctx.fillStyle = selected ? channelColor : channelColor + "cc";
      ctx.fillRect(x, y, w, h);
      ctx.strokeStyle = selected ? "#ffffff" : channelColor;
      ctx.lineWidth = 1;
      ctx.strokeRect(x + 0.5, y + 0.5, w - 1, h - 1);
    }
  }, [notes, minPitch, maxPitch, durationTicks, ticksPerPixel, rowHeight, gridSnapTicks, channelColor, selectedNotes, canvasWidth, canvasHeight, totalNotes]);

  useEffect(() => { draw(); }, [draw]);

  function handleClick(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    const clickTick = x * ticksPerPixel;
    const clickRow = Math.floor(y / rowHeight);
    const clickPitch = maxPitch - clickRow;

    for (let i = 0; i < notes.length; i++) {
      const n = notes[i];
      if (n.pitch !== clickPitch) continue;
      const nx = n.tick / ticksPerPixel;
      const nw = n.durationTicks / ticksPerPixel;
      if (x >= nx && x <= nx + nw) {
        onNoteClick(i);
        return;
      }
    }

    const snapped = Math.floor(clickTick / gridSnapTicks) * gridSnapTicks;
    if (clickPitch >= minPitch && clickPitch <= maxPitch) {
      onNoteAdd(snapped, clickPitch);
    }
  }

  function handleScroll() {
    if (containerRef.current) {
      onScrollTopChange(containerRef.current.scrollTop);
    }
  }

  return (
    <div ref={containerRef} className={styles.container} onScroll={handleScroll}>
      <canvas ref={canvasRef} className={styles.canvas} onClick={handleClick} />
    </div>
  );
}
```

Create `src/components/PianoRollCanvas.module.css`:
```css
.container {
  flex: 1;
  overflow: auto;
}

.canvas {
  display: block;
}
```

- [ ] **Step 3: Create VelocityLane**

Create `src/components/VelocityLane.tsx`:

```typescript
import { useRef, useEffect, useCallback } from "react";
import type { Note } from "../types/model";
import styles from "./VelocityLane.module.css";

interface VelocityLaneProps {
  notes: Note[];
  durationTicks: number;
  ticksPerPixel: number;
  channelColor: string;
  onVelocityChange: (noteIndex: number, velocity: number) => void;
}

const LANE_HEIGHT = 60;

export function VelocityLane({ notes, durationTicks, ticksPerPixel, channelColor, onVelocityChange }: VelocityLaneProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  const canvasWidth = durationTicks / ticksPerPixel;

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const dpr = window.devicePixelRatio || 1;
    canvas.width = canvasWidth * dpr;
    canvas.height = LANE_HEIGHT * dpr;
    canvas.style.width = `${canvasWidth}px`;
    canvas.style.height = `${LANE_HEIGHT}px`;

    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    ctx.scale(dpr, dpr);

    ctx.fillStyle = "#1a1a1a";
    ctx.fillRect(0, 0, canvasWidth, LANE_HEIGHT);

    for (const note of notes) {
      const x = note.tick / ticksPerPixel;
      const barHeight = (note.velocity / 127) * (LANE_HEIGHT - 4);
      const alpha = 0.5 + (note.velocity / 127) * 0.5;
      ctx.fillStyle = channelColor + Math.round(alpha * 255).toString(16).padStart(2, "0");
      ctx.fillRect(x, LANE_HEIGHT - barHeight - 2, Math.max(4, note.durationTicks / ticksPerPixel * 0.8), barHeight);
    }
  }, [notes, durationTicks, ticksPerPixel, channelColor, canvasWidth]);

  useEffect(() => { draw(); }, [draw]);

  function handleMouseDown(e: React.MouseEvent) {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const rect = canvas.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;

    for (let i = 0; i < notes.length; i++) {
      const nx = notes[i].tick / ticksPerPixel;
      const nw = Math.max(4, notes[i].durationTicks / ticksPerPixel * 0.8);
      if (x >= nx && x <= nx + nw) {
        const vel = Math.round((1 - y / LANE_HEIGHT) * 127);
        onVelocityChange(i, Math.max(1, Math.min(127, vel)));
        return;
      }
    }
  }

  return (
    <div className={styles.lane}>
      <canvas ref={canvasRef} onMouseDown={handleMouseDown} />
    </div>
  );
}
```

Create `src/components/VelocityLane.module.css`:
```css
.lane {
  height: 60px;
  flex-shrink: 0;
  border-top: 1px solid var(--border);
  overflow-x: auto;
  overflow-y: hidden;
}
```

- [ ] **Step 4: Create PianoRoll orchestrator**

Create `src/components/PianoRoll.tsx`:

```typescript
import { useState, useEffect, useCallback } from "react";
import type { Note, SelectedRegion } from "../types/model";
import { PianoRollKeys } from "./PianoRollKeys";
import { PianoRollCanvas } from "./PianoRollCanvas";
import { VelocityLane } from "./VelocityLane";
import * as ipc from "../api/ipc";
import styles from "./PianoRoll.module.css";

interface PianoRollProps {
  region: SelectedRegion;
  onClose: () => void;
}

const GRID_OPTIONS: { label: string; divisor: number }[] = [
  { label: "1/1", divisor: 1 },
  { label: "1/2", divisor: 2 },
  { label: "1/4", divisor: 4 },
  { label: "1/8", divisor: 8 },
  { label: "1/16", divisor: 16 },
  { label: "1/32", divisor: 32 },
  { label: "1/4T", divisor: 6 },
  { label: "1/8T", divisor: 12 },
];

const CHANNEL_COLORS: Record<string, string> = {
  fm: "#4a9eff",
  psg: "#44cc66",
  dac: "#ff8844",
};

const PITCH_RANGES: Record<string, [number, number]> = {
  fm: [24, 95],
  psg: [33, 95],
  dac: [0, 0],
};

export function PianoRoll({ region, onClose }: PianoRollProps) {
  const [notes, setNotes] = useState<Note[]>([]);
  const [selectedNotes, setSelectedNotes] = useState<Set<number>>(new Set());
  const [gridIdx, setGridIdx] = useState(4); // 1/16
  const [scrollTop, setScrollTop] = useState(0);
  const ticksPerBeat = 480;
  const gridSnapTicks = Math.round(ticksPerBeat * 4 / GRID_OPTIONS[gridIdx].divisor);
  const [minPitch, maxPitch] = PITCH_RANGES[region.channelType] || [24, 95];
  const rowHeight = 14;
  const ticksPerPixel = region.durationTicks / 800;
  const channelColor = CHANNEL_COLORS[region.channelType] || "#888";

  const refresh = useCallback(async () => {
    const tracks = await ipc.listTracks();
    const track = tracks.find((t) => t.id === region.trackId);
    if (!track) return;
    const r = track.regions.find((r) => r.id === region.regionId);
    if (!r) return;
    setNotes(r.notes);
  }, [region.trackId, region.regionId]);

  useEffect(() => { refresh(); }, [refresh]);

  async function handleNoteAdd(tick: number, pitch: number) {
    await ipc.addNote(region.trackId, region.regionId, tick, pitch, 100, gridSnapTicks);
    refresh();
  }

  async function handleNoteClick(index: number) {
    setSelectedNotes(new Set([index]));
  }

  async function handleVelocityChange(index: number, velocity: number) {
    const note = notes[index];
    if (!note) return;
    await ipc.updateNote(region.trackId, region.regionId, index, note.tick, note.pitch, velocity, note.durationTicks);
    refresh();
  }

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Delete" && selectedNotes.size > 0) {
        const sorted = Array.from(selectedNotes).sort((a, b) => b - a);
        (async () => {
          for (const idx of sorted) {
            await ipc.deleteNote(region.trackId, region.regionId, idx);
          }
          setSelectedNotes(new Set());
          refresh();
        })();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [selectedNotes, region.trackId, region.regionId, refresh]);

  async function handleAudition(pitch: number) {
    const tracks = await ipc.listTracks();
    const track = tracks.find((t) => t.id === region.trackId);
    if (!track?.instrumentId) return;
    if (region.channelType === "fm") {
      await ipc.previewFmInstrument(track.instrumentId, pitch);
    } else if (region.channelType === "psg") {
      await ipc.previewPsgInstrument(track.instrumentId, pitch);
    } else {
      await ipc.previewDac(track.instrumentId);
    }
  }

  const barStart = Math.floor(region.startTick / (ticksPerBeat * 4)) + 1;
  const barEnd = Math.ceil((region.startTick + region.durationTicks) / (ticksPerBeat * 4));

  return (
    <div className={styles.pianoRoll}>
      <div className={styles.header}>
        <span className={styles.label}>
          {region.trackName} | Bars {barStart}-{barEnd}
        </span>
        <select
          className={styles.gridSelect}
          value={gridIdx}
          onChange={(e) => setGridIdx(parseInt(e.target.value))}
        >
          {GRID_OPTIONS.map((opt, i) => (
            <option key={opt.label} value={i}>{opt.label}</option>
          ))}
        </select>
        <button className={styles.closeBtn} onClick={onClose}>x</button>
      </div>
      <div className={styles.body}>
        <PianoRollKeys
          minPitch={minPitch}
          maxPitch={maxPitch}
          rowHeight={rowHeight}
          scrollTop={scrollTop}
          onAudition={handleAudition}
        />
        <PianoRollCanvas
          notes={notes}
          minPitch={minPitch}
          maxPitch={maxPitch}
          durationTicks={region.durationTicks}
          ticksPerPixel={ticksPerPixel}
          rowHeight={rowHeight}
          gridSnapTicks={gridSnapTicks}
          channelColor={channelColor}
          selectedNotes={selectedNotes}
          onNoteClick={handleNoteClick}
          onNoteAdd={handleNoteAdd}
          onScrollTopChange={setScrollTop}
        />
      </div>
      <VelocityLane
        notes={notes}
        durationTicks={region.durationTicks}
        ticksPerPixel={ticksPerPixel}
        channelColor={channelColor}
        onVelocityChange={handleVelocityChange}
      />
    </div>
  );
}
```

Create `src/components/PianoRoll.module.css`:
```css
.pianoRoll {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.header {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 8px;
  border-bottom: 1px solid var(--border);
  flex-shrink: 0;
}

.label {
  font-size: 12px;
  color: var(--text-secondary);
  flex: 1;
}

.gridSelect {
  font-size: 11px;
  padding: 2px 6px;
}

.closeBtn {
  width: 24px;
  height: 24px;
  background: none;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 3px;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.closeBtn:hover {
  color: var(--text-primary);
  background: var(--bg-surface);
}

.body {
  display: flex;
  flex: 1;
  overflow: hidden;
}
```

- [ ] **Step 5: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src/components/PianoRoll*.tsx src/components/PianoRoll*.module.css src/components/PianoRollKeys.tsx src/components/PianoRollKeys.module.css src/components/VelocityLane.tsx src/components/VelocityLane.module.css
git commit -m "feat(ui): piano roll — note grid, keys, velocity lane with grid snap"
```

---

### Task 9: BottomPanel Routing + App Wiring

**Files:**
- Modify: `src/components/BottomPanel.tsx`
- Modify: `src/components/BottomPanel.module.css`
- Modify: `src/App.tsx`

- [ ] **Step 1: Update BottomPanel for dual-mode routing**

Replace `src/components/BottomPanel.tsx`:

```typescript
import { useState } from "react";
import type { SelectedInstrument, SelectedRegion } from "../types/model";
import { FmEditor } from "./FmEditor";
import { PsgEditor } from "./PsgEditor";
import { DacEditor } from "./DacEditor";
import { PianoRoll } from "./PianoRoll";
import styles from "./BottomPanel.module.css";

interface BottomPanelProps {
  selectedInstrument: SelectedInstrument | null;
  selectedRegion: SelectedRegion | null;
  onCloseRegion: () => void;
}

export function BottomPanel({ selectedInstrument, selectedRegion, onCloseRegion }: BottomPanelProps) {
  const [collapsed, setCollapsed] = useState(false);

  const showPianoRoll = selectedRegion !== null;
  const headerText = showPianoRoll ? "Piano Roll" : "Instrument Editor";

  return (
    <div className={`${styles.panel} ${collapsed ? styles.collapsed : ""}`}>
      <div className={styles.header} onClick={() => setCollapsed(!collapsed)}>
        <span className={styles.toggle}>{collapsed ? "▶" : "▼"}</span>
        <span>{headerText}</span>
      </div>
      {!collapsed && (
        <div className={styles.editor}>
          {showPianoRoll ? (
            <PianoRoll region={selectedRegion} onClose={onCloseRegion} />
          ) : (
            <>
              {!selectedInstrument && (
                <div className={styles.empty}>Select an instrument to edit</div>
              )}
              {selectedInstrument?.type === "fm" && (
                <FmEditor instrumentId={selectedInstrument.id} />
              )}
              {selectedInstrument?.type === "psg" && (
                <PsgEditor instrumentId={selectedInstrument.id} />
              )}
              {selectedInstrument?.type === "dac" && (
                <DacEditor instrumentId={selectedInstrument.id} />
              )}
            </>
          )}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Update App.tsx final wiring**

Ensure `App.tsx` passes all needed props. The complete updated render section:

```typescript
  return (
    <div className={styles.app}>
      <TopBar
        projectMeta={projectMeta}
        onNewProject={() => setShowNewProject(true)}
        onOpenProject={handleOpenProject}
        onSave={handleSave}
        showSaved={showSaved}
        playing={playing}
        loopEnabled={loopEnabled}
        onPlayingChange={setPlaying}
        onLoopChange={setLoopEnabled}
      />
      <div className={styles.body}>
        {projectOpen && (
          <Sidebar
            projectMeta={projectMeta!}
            selectedInstrument={selectedInstrument}
            onSelectInstrument={(inst) => {
              setSelectedInstrument(inst);
              setSelectedRegion(null);
            }}
          />
        )}
        <MainArea
          projectOpen={projectOpen}
          projectMeta={projectMeta}
          playing={playing}
          onNewProject={() => setShowNewProject(true)}
          onOpenProject={handleOpenProject}
          onSelectRegion={(region) => {
            setSelectedRegion(region);
            if (region) setSelectedInstrument(null);
          }}
          selectedRegion={selectedRegion}
        />
      </div>
      {projectOpen && (
        <BottomPanel
          selectedInstrument={selectedInstrument}
          selectedRegion={selectedRegion}
          onCloseRegion={() => setSelectedRegion(null)}
        />
      )}
      {showNewProject && (
        <NewProjectDialog
          onClose={() => setShowNewProject(false)}
          onCreated={handleProjectCreated}
        />
      )}
    </div>
  );
```

Import `SelectedRegion` at the top of `App.tsx`:
```typescript
import type { SongMetadata, SelectedInstrument, SelectedRegion } from "./types/model";
```

- [ ] **Step 3: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add src/components/BottomPanel.tsx src/App.tsx
git commit -m "feat(ui): BottomPanel dual-mode routing — piano roll vs. instrument editor"
```

---

### Task 10: Build Verification + Manual Testing

**Files:** None (testing only)

- [ ] **Step 1: Run all Rust tests**

Run: `cd /home/volence/sonic_hacks/megadaw/src-tauri && cargo test`
Expected: All tests pass (54 original + ~17 new)

- [ ] **Step 2: Run TypeScript type check**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Run dev server**

Run: `cd /home/volence/sonic_hacks/megadaw && WEBKIT_DISABLE_COMPOSITING_MODE=1 cargo tauri dev`
Expected: App launches, no console errors

- [ ] **Step 4: Manual test sequence**

1. Create a new project
2. Click "Add Track", create an FM track on FM1
3. Assign an FM instrument to the track
4. Double-click empty space in the timeline → region appears
5. Double-click the region → piano roll opens in bottom panel
6. Click in the piano roll to place notes
7. Close piano roll → instrument editor returns
8. Press Space → playback starts, cursor moves, notes sound through emulator
9. Press Space again → stops
10. Test mute/solo buttons on track header
11. Test loop toggle

- [ ] **Step 5: Fix any issues found during manual testing**

Address compilation errors, UI bugs, or integration issues.

- [ ] **Step 6: Final commit**

```bash
cd /home/volence/sonic_hacks/megadaw && git add -A
git commit -m "fix: Phase 4 integration fixes from manual testing"
```
