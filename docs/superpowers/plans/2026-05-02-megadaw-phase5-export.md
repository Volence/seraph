# MegaDAW Phase 5: Flamedriver Export Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Export MegaDAW compositions to Flamedriver SMPS assembly format that assembles directly into the ROM.

**Architecture:** Extend `DriverProfile` trait with `export_song()`. Flamedriver implementation converts Song → SMPS assembly text files. New `export/` module handles shared types. Frontend adds Export button in TopBar with directory picker.

**Tech Stack:** Rust, Tauri v2, React 19, TypeScript, AS Macro Assembler (target format)

---

## File Structure

### New Files
- `src-tauri/src/export/mod.rs` — ExportResult, ExportError types
- `src-tauri/src/export/smps.rs` — SMPS assembly generation (tick mapping, note encoding, voice bank, file writing)
- `src/components/ExportDialog.tsx` — Error display dialog
- `src/components/ExportDialog.module.css` — Styles

### Modified Files
- `src-tauri/src/model/driver.rs` — Add `export_song()` to DriverProfile trait
- `src-tauri/src/driver/flamedriver.rs` — Implement `export_song()` delegating to smps module
- `src-tauri/src/driver/mod.rs` — Re-export
- `src-tauri/src/ipc/commands.rs` — Add `export_song` IPC command
- `src-tauri/src/ipc/mod.rs` — Re-export new command
- `src-tauri/src/lib.rs` — Register command + add export module
- `src/api/ipc.ts` — Add `exportSong()` wrapper
- `src/components/TopBar.tsx` — Add Export button
- `src/App.tsx` — Wire export handler

---

### Task 1: Export Types & Trait Extension

**Files:**
- Create: `src-tauri/src/export/mod.rs`
- Modify: `src-tauri/src/model/driver.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Create export module with shared types**

Create `src-tauri/src/export/mod.rs`:

```rust
pub mod smps;

use serde::Serialize;
use std::path::Path;

use crate::model::instrument::InstrumentBank;
use crate::model::song::Song;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportResult {
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportError {
    pub track_name: String,
    pub region_index: Option<usize>,
    pub note_index: Option<usize>,
    pub message: String,
}

pub trait SongExporter {
    fn export_song(
        &self,
        song: &Song,
        instruments: &InstrumentBank,
        output_dir: &Path,
    ) -> Result<ExportResult, Vec<ExportError>>;
}
```

- [ ] **Step 2: Add export_song to DriverProfile trait**

In `src-tauri/src/model/driver.rs`, add the import and trait method:

Add to imports:
```rust
use crate::export::{ExportResult, ExportError};
use crate::model::instrument::InstrumentBank;
use crate::model::song::Song;
use std::path::Path;
```

Add to `DriverProfile` trait:
```rust
fn export_song(
    &self,
    song: &Song,
    instruments: &InstrumentBank,
    output_dir: &Path,
) -> Result<ExportResult, Vec<ExportError>>;
```

- [ ] **Step 3: Add stub implementation to FlamedriverProfile**

In `src-tauri/src/driver/flamedriver.rs`, add the import and stub:

```rust
use crate::export::{ExportResult, ExportError};
use crate::model::song::Song;
use std::path::Path;
```

Add to the `impl DriverProfile for FlamedriverProfile` block:
```rust
fn export_song(
    &self,
    _song: &Song,
    _instruments: &InstrumentBank,
    _output_dir: &Path,
) -> Result<ExportResult, Vec<ExportError>> {
    Err(vec![ExportError {
        track_name: String::new(),
        region_index: None,
        note_index: None,
        message: "Export not yet implemented".into(),
    }])
}
```

- [ ] **Step 4: Register export module in lib.rs**

Add `mod export;` to `src-tauri/src/lib.rs` alongside the other module declarations.

- [ ] **Step 5: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles with warnings only

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/export/ src-tauri/src/model/driver.rs src-tauri/src/driver/flamedriver.rs src-tauri/src/lib.rs
git commit -m "feat(export): add ExportResult/ExportError types and DriverProfile::export_song() trait method"
```

---

### Task 2: SMPS Tick Mapping

**Files:**
- Create: `src-tauri/src/export/smps.rs`

- [ ] **Step 1: Write tests for tick mapping**

Create `src-tauri/src/export/smps.rs` with the tick mapping logic and tests:

```rust
use crate::export::{ExportError, ExportResult};
use crate::model::instrument::InstrumentBank;
use crate::model::song::Song;
use std::path::Path;

/// SMPS tempo parameters chosen to best represent the song's BPM.
#[derive(Debug, Clone, Copy)]
pub struct SmpsTempoParams {
    pub divider: u8,
    pub modifier: u8,
    /// How many DAW ticks equal one SMPS tick
    pub daw_ticks_per_smps_tick: f64,
}

/// Find the best (divider, modifier) pair for the given BPM and DAW ticks_per_beat.
/// Minimizes quantization error for common subdivisions (quarter, eighth, sixteenth, triplet).
pub fn compute_tempo_params(bpm: f64, ticks_per_beat: u32) -> SmpsTempoParams {
    let tpb = ticks_per_beat as f64;
    let beats_per_second = bpm / 60.0;

    let mut best_divider = 1u8;
    let mut best_modifier = 1u8;
    let mut best_error = f64::MAX;

    for divider in 1..=4u8 {
        for modifier in 1..=255u8 {
            let smps_ticks_per_sec = (modifier as f64 / 256.0) * 60.0;
            let smps_ticks_per_beat = smps_ticks_per_sec / beats_per_second;
            let daw_per_smps = tpb / smps_ticks_per_beat;

            // Check quantization error for common durations
            let test_durations = [
                tpb,            // quarter note
                tpb / 2.0,      // eighth
                tpb / 4.0,      // sixteenth
                tpb / 3.0,      // triplet
                tpb * 2.0,      // half note
            ];

            let mut total_error = 0.0;
            let mut valid = true;
            for &dur in &test_durations {
                let smps_dur = dur / daw_per_smps;
                let rounded = smps_dur.round();
                if rounded < 1.0 {
                    valid = false;
                    break;
                }
                let err = (smps_dur - rounded).abs() / smps_dur;
                total_error += err;
            }

            if valid && total_error < best_error {
                best_error = total_error;
                best_divider = divider;
                best_modifier = modifier;
            }
        }
    }

    let smps_ticks_per_sec = (best_modifier as f64 / 256.0) * 60.0;
    let smps_ticks_per_beat = smps_ticks_per_sec / beats_per_second;
    let daw_per_smps = tpb / smps_ticks_per_beat;

    SmpsTempoParams {
        divider: best_divider,
        modifier: best_modifier,
        daw_ticks_per_smps_tick: daw_per_smps,
    }
}

/// Convert a DAW tick duration to SMPS ticks. Returns None if it rounds to 0.
pub fn daw_to_smps_duration(daw_ticks: u64, params: &SmpsTempoParams) -> Option<u64> {
    let smps = (daw_ticks as f64 / params.daw_ticks_per_smps_tick).round() as u64;
    if smps == 0 { None } else { Some(smps) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tempo_120bpm_480tpb() {
        let params = compute_tempo_params(120.0, 480);
        assert!(params.modifier > 0);
        assert!(params.divider >= 1);
        // Quarter note should convert to a whole number
        let quarter = daw_to_smps_duration(480, &params);
        assert!(quarter.is_some());
        let q = quarter.unwrap();
        assert!(q > 0 && q <= 127, "quarter note = {q} SMPS ticks");
    }

    #[test]
    fn test_tempo_140bpm_480tpb() {
        let params = compute_tempo_params(140.0, 480);
        let quarter = daw_to_smps_duration(480, &params).unwrap();
        assert!(quarter > 0 && quarter <= 127);
    }

    #[test]
    fn test_eighth_note_converts() {
        let params = compute_tempo_params(120.0, 480);
        let eighth = daw_to_smps_duration(240, &params);
        assert!(eighth.is_some());
    }

    #[test]
    fn test_zero_duration_returns_none() {
        let params = compute_tempo_params(120.0, 480);
        // A 1-tick DAW duration is likely too short
        let result = daw_to_smps_duration(0, &params);
        assert!(result.is_none());
    }

    #[test]
    fn test_long_note_converts() {
        let params = compute_tempo_params(120.0, 480);
        // 4 beats = whole note
        let whole = daw_to_smps_duration(1920, &params).unwrap();
        assert!(whole > 0);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd src-tauri && cargo test --lib export::smps::tests`
Expected: All 5 tests pass

- [ ] **Step 3: Commit**

```bash
git add src-tauri/src/export/smps.rs
git commit -m "feat(export): SMPS tick mapping with tempo parameter search"
```

---

### Task 3: SMPS Note Encoding

**Files:**
- Modify: `src-tauri/src/export/smps.rs`

- [ ] **Step 1: Add MIDI-to-SMPS pitch mapping and note encoding**

Add to `smps.rs`:

```rust
/// SMPS note constants: nC0 = 0x81, each semitone increments by 1.
/// Valid MIDI range: 12 (C0) through 95 (Bb7).
pub fn midi_to_smps_note(midi_pitch: u8) -> Option<u8> {
    if midi_pitch < 12 || midi_pitch > 95 {
        return None;
    }
    Some(0x81 + (midi_pitch - 12))
}

/// SMPS rest constant.
const SMPS_REST: u8 = 0x80;
/// Maximum duration in a single SMPS byte.
const SMPS_MAX_DURATION: u64 = 127;
/// No-attack flag (tie).
const SMPS_NO_ATTACK: &str = "smpsNoAttack";

/// SMPS note name for assembly output.
pub fn smps_note_name(midi_pitch: u8) -> String {
    let semitone = (midi_pitch - 12) % 12;
    let octave = (midi_pitch - 12) / 12;
    let name = match semitone {
        0 => "nC", 1 => "nCs", 2 => "nD", 3 => "nDs",
        4 => "nE", 5 => "nF", 6 => "nFs", 7 => "nG",
        8 => "nGs", 9 => "nA", 10 => "nAs", 11 => "nB",
        _ => unreachable!(),
    };
    format!("{name}{octave}")
}

/// A single event in the SMPS output stream.
#[derive(Debug, Clone)]
pub enum SmpsEvent {
    /// Note with pitch name and duration in SMPS ticks
    Note { pitch_name: String, duration: u64 },
    /// Rest with duration in SMPS ticks
    Rest { duration: u64 },
    /// Tie (no-attack flag) — continues previous note
    Tie,
    /// Set FM voice index
    SetVoice(u8),
    /// Set panning
    SetPan(u8),
    /// Set PSG voice index
    SetPsgVoice(u8),
    /// End of channel
    Stop,
}

/// Encode a sequence of note-on/note-off events into SmpsEvents.
/// `events` must be sorted by tick. Each event is (absolute_tick, pitch, duration_ticks) in DAW ticks.
/// Gaps between notes become rests.
pub fn encode_channel_events(
    notes: &[(u64, u8, u64)],
    region_duration: u64,
    params: &SmpsTempoParams,
) -> Result<Vec<SmpsEvent>, ExportError> {
    let mut out = Vec::new();
    let mut cursor: u64 = 0; // current position in DAW ticks

    for &(tick, pitch, dur_ticks) in notes {
        // Insert rest for gap before this note
        if tick > cursor {
            let gap = tick - cursor;
            if let Some(smps_gap) = daw_to_smps_duration(gap, params) {
                emit_duration_events(&mut out, None, smps_gap);
            }
        }

        let smps_dur = daw_to_smps_duration(dur_ticks, params)
            .ok_or_else(|| ExportError {
                track_name: String::new(),
                region_index: None,
                note_index: None,
                message: format!("Note duration {dur_ticks} DAW ticks rounds to 0 SMPS ticks"),
            })?;

        let pitch_name = smps_note_name(pitch);
        emit_duration_events(&mut out, Some(pitch_name), smps_dur);

        cursor = tick + dur_ticks;
    }

    // Trailing rest if notes don't fill the region
    if cursor < region_duration {
        let gap = region_duration - cursor;
        if let Some(smps_gap) = daw_to_smps_duration(gap, params) {
            emit_duration_events(&mut out, None, smps_gap);
        }
    }

    Ok(out)
}

/// Emit note or rest events, splitting at SMPS_MAX_DURATION and using ties for notes.
fn emit_duration_events(out: &mut Vec<SmpsEvent>, pitch_name: Option<String>, total: u64) {
    let mut remaining = total;
    let mut first = true;

    while remaining > 0 {
        let chunk = remaining.min(SMPS_MAX_DURATION);

        if !first {
            if pitch_name.is_some() {
                out.push(SmpsEvent::Tie);
            }
        }

        match &pitch_name {
            Some(name) => out.push(SmpsEvent::Note {
                pitch_name: name.clone(),
                duration: chunk,
            }),
            None => out.push(SmpsEvent::Rest { duration: chunk }),
        }

        remaining -= chunk;
        first = false;
    }
}
```

- [ ] **Step 2: Add tests for note encoding**

Add to the `tests` module:

```rust
    #[test]
    fn test_midi_to_smps_note_c4() {
        assert_eq!(midi_to_smps_note(60), Some(0x81 + 48)); // C4
    }

    #[test]
    fn test_midi_to_smps_note_out_of_range() {
        assert_eq!(midi_to_smps_note(11), None);  // below C0
        assert_eq!(midi_to_smps_note(96), None);  // above Bb7
    }

    #[test]
    fn test_smps_note_name() {
        assert_eq!(smps_note_name(60), "nC4");
        assert_eq!(smps_note_name(61), "nCs4");
        assert_eq!(smps_note_name(12), "nC0");
    }

    #[test]
    fn test_encode_single_note() {
        let params = compute_tempo_params(120.0, 480);
        let notes = vec![(0u64, 60u8, 480u64)]; // C4, quarter note
        let events = encode_channel_events(&notes, 480, &params).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Note { .. })));
    }

    #[test]
    fn test_encode_gap_produces_rest() {
        let params = compute_tempo_params(120.0, 480);
        // Note starts at tick 480, so there's a 480-tick gap at the start
        let notes = vec![(480u64, 60u8, 480u64)];
        let events = encode_channel_events(&notes, 960, &params).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Rest { .. })));
    }

    #[test]
    fn test_long_note_splits_with_tie() {
        let params = SmpsTempoParams { divider: 1, modifier: 128, daw_ticks_per_smps_tick: 1.0 };
        // 200 SMPS ticks > 127 max, should split
        let notes = vec![(0u64, 60u8, 200u64)];
        let events = encode_channel_events(&notes, 200, &params).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Tie)));
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib export::smps::tests`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/export/smps.rs
git commit -m "feat(export): SMPS note encoding with pitch mapping, rests, and tie splitting"
```

---

### Task 4: SMPS Voice Bank Generation

**Files:**
- Modify: `src-tauri/src/export/smps.rs`

- [ ] **Step 1: Add voice bank assembly generation**

Add to `smps.rs`:

```rust
use crate::model::instrument::{FmInstrument, InstrumentBank, PsgInstrument};
use crate::model::song::{ChannelAssignment, Pan, Track};
use std::collections::HashMap;
use uuid::Uuid;

/// Build a deduplicated voice index: instrument UUID → voice index.
/// Returns (index_map, ordered list of FM instruments).
pub fn build_voice_index(tracks: &[Track], instruments: &InstrumentBank) -> (HashMap<Uuid, u8>, Vec<&FmInstrument>) {
    let mut map = HashMap::new();
    let mut voices: Vec<&FmInstrument> = Vec::new();

    for track in tracks {
        if track.muted { continue; }
        if !matches!(track.channel, ChannelAssignment::Fm(_)) { continue; }
        if let Some(inst_id) = &track.instrument_id {
            if map.contains_key(inst_id) { continue; }
            if let Some(inst) = instruments.fm.iter().find(|i| &i.id == inst_id) {
                let idx = voices.len() as u8;
                map.insert(*inst_id, idx);
                voices.push(inst);
            }
        }
    }

    (map, voices)
}

/// Generate the voice bank assembly text.
pub fn generate_voice_bank_asm(song_label: &str, voices: &[&FmInstrument], driver: &crate::driver::flamedriver::FlamedriverProfile) -> String {
    use crate::model::driver::DriverProfile;
    let mut asm = String::new();
    asm.push_str(&format!("; ============================================================\n"));
    asm.push_str(&format!("; Voice Bank: {song_label}\n"));
    asm.push_str(&format!("; Exported from MegaDAW\n"));
    asm.push_str(&format!("; ============================================================\n\n"));
    asm.push_str(&format!("Snd_{song_label}_Voices:\n"));

    for (i, inst) in voices.iter().enumerate() {
        let bytes = driver.fm_to_bytes(inst);
        // bytes layout (Flamedriver order: op4, op3, op2, op1):
        //   [0..4]   DT/MUL
        //   [4..8]   RS/AR
        //   [8..12]  AM/D1R
        //   [12..16] D2R
        //   [16..20] SL/RR
        //   [20..24] TL (bit 7 = carrier flag)
        //   [24]     FB/ALG

        let alg = bytes[24] & 0x07;
        let fb = (bytes[24] >> 3) & 0x07;

        asm.push_str(&format!("\n; Voice {i} - \"{}\"\n", inst.name));
        asm.push_str(&format!("\tsmpsVcAlgorithm\t\t${alg:02X}\n"));
        asm.push_str(&format!("\tsmpsVcFeedback\t\t${fb:02X}\n"));
        asm.push_str(&format!("\tsmpsVcUnusedBits\t$00\n"));

        // DT/MUL: bytes[0..4] → split into detune (high nibble) and coarse freq (low nibble)
        let dt: Vec<String> = (0..4).map(|j| format!("${:02X}", (bytes[j] >> 4) & 0x07)).collect();
        let mul: Vec<String> = (0..4).map(|j| format!("${:02X}", bytes[j] & 0x0F)).collect();
        asm.push_str(&format!("\tsmpsVcDetune\t\t{}\n", dt.join(", ")));
        asm.push_str(&format!("\tsmpsVcCoarseFreq\t{}\n", mul.join(", ")));

        // RS/AR: bytes[4..8]
        let rs: Vec<String> = (4..8).map(|j| format!("${:02X}", (bytes[j] >> 6) & 0x03)).collect();
        let ar: Vec<String> = (4..8).map(|j| format!("${:02X}", bytes[j] & 0x1F)).collect();
        asm.push_str(&format!("\tsmpsVcRateScale\t\t{}\n", rs.join(", ")));
        asm.push_str(&format!("\tsmpsVcAttackRate\t{}\n", ar.join(", ")));

        // AM/D1R: bytes[8..12]
        let am: Vec<String> = (8..12).map(|j| format!("${:02X}", (bytes[j] >> 7) & 0x01)).collect();
        let d1r: Vec<String> = (8..12).map(|j| format!("${:02X}", bytes[j] & 0x1F)).collect();
        asm.push_str(&format!("\tsmpsVcAmpMod\t\t{}\n", am.join(", ")));
        asm.push_str(&format!("\tsmpsVcDecayRate1\t{}\n", d1r.join(", ")));

        // D2R: bytes[12..16]
        let d2r: Vec<String> = (12..16).map(|j| format!("${:02X}", bytes[j] & 0x1F)).collect();
        asm.push_str(&format!("\tsmpsVcDecayRate2\t{}\n", d2r.join(", ")));

        // SL/RR: bytes[16..20]
        let sl: Vec<String> = (16..20).map(|j| format!("${:02X}", (bytes[j] >> 4) & 0x0F)).collect();
        let rr: Vec<String> = (16..20).map(|j| format!("${:02X}", bytes[j] & 0x0F)).collect();
        asm.push_str(&format!("\tsmpsVcDecayLevel\t{}\n", sl.join(", ")));
        asm.push_str(&format!("\tsmpsVcReleaseRate\t{}\n", rr.join(", ")));

        // TL: bytes[20..24] (strip carrier flag bit 7)
        let tl: Vec<String> = (20..24).map(|j| format!("${:02X}", bytes[j] & 0x7F)).collect();
        asm.push_str(&format!("\tsmpsVcTotalLevel\t{}\n", tl.join(", ")));
    }

    asm
}
```

- [ ] **Step 2: Add tests**

```rust
    #[test]
    fn test_build_voice_index_deduplicates() {
        use crate::model::song::{Track, Region, ChannelAssignment, Pan};
        use crate::model::instrument::*;
        use uuid::Uuid;

        let inst_id = Uuid::new_v4();
        let tracks = vec![
            Track {
                id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
                instrument_id: Some(inst_id), regions: vec![], muted: false, solo: false,
                volume: 100, pan: Pan::Center,
            },
            Track {
                id: Uuid::new_v4(), name: "FM2".into(), channel: ChannelAssignment::Fm(1),
                instrument_id: Some(inst_id), regions: vec![], muted: false, solo: false,
                volume: 100, pan: Pan::Center,
            },
        ];
        let bank = InstrumentBank {
            fm: vec![FmInstrument {
                id: inst_id, name: "Test".into(), algorithm: 0, feedback: 0,
                operators: [FmOperator::default(); 4],
                metadata: InstrumentMetadata { category: String::new(), author: String::new(), tags: vec![] },
            }],
            psg: vec![], dac: vec![],
        };
        let (map, voices) = build_voice_index(&tracks, &bank);
        assert_eq!(voices.len(), 1);
        assert_eq!(map[&inst_id], 0);
    }

    #[test]
    fn test_muted_tracks_excluded_from_voice_index() {
        use crate::model::song::{Track, ChannelAssignment, Pan};
        use crate::model::instrument::*;
        use uuid::Uuid;

        let inst_id = Uuid::new_v4();
        let tracks = vec![Track {
            id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
            instrument_id: Some(inst_id), regions: vec![], muted: true, solo: false,
            volume: 100, pan: Pan::Center,
        }];
        let bank = InstrumentBank {
            fm: vec![FmInstrument {
                id: inst_id, name: "Test".into(), algorithm: 0, feedback: 0,
                operators: [FmOperator::default(); 4],
                metadata: InstrumentMetadata { category: String::new(), author: String::new(), tags: vec![] },
            }],
            psg: vec![], dac: vec![],
        };
        let (map, voices) = build_voice_index(&tracks, &bank);
        assert!(voices.is_empty());
        assert!(map.is_empty());
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib export::smps::tests`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/export/smps.rs
git commit -m "feat(export): voice bank generation with deduplication"
```

---

### Task 5: Pre-Export Validation

**Files:**
- Modify: `src-tauri/src/export/smps.rs`

- [ ] **Step 1: Add validation function**

Add to `smps.rs`:

```rust
/// Validate the song for SMPS export. Returns a list of errors (empty = valid).
pub fn validate_for_export(
    song: &Song,
    instruments: &InstrumentBank,
    params: &SmpsTempoParams,
) -> Vec<ExportError> {
    let mut errors = Vec::new();

    for track in &song.tracks {
        if track.muted { continue; }

        // Check instrument assignment
        if track.instrument_id.is_none() {
            errors.push(ExportError {
                track_name: track.name.clone(),
                region_index: None,
                note_index: None,
                message: "No instrument assigned".into(),
            });
            continue;
        }

        let inst_id = track.instrument_id.as_ref().unwrap();

        // Check instrument exists
        let inst_exists = match &track.channel {
            ChannelAssignment::Fm(_) => instruments.fm.iter().any(|i| &i.id == inst_id),
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => instruments.psg.iter().any(|i| &i.id == inst_id),
            ChannelAssignment::Dac(_) => instruments.dac.iter().any(|i| &i.id == inst_id),
        };
        if !inst_exists {
            errors.push(ExportError {
                track_name: track.name.clone(),
                region_index: None,
                note_index: None,
                message: "Assigned instrument not found".into(),
            });
            continue;
        }

        // Check for empty track (no notes at all)
        let has_notes = track.regions.iter().any(|r| !r.notes.is_empty());
        if !has_notes {
            errors.push(ExportError {
                track_name: track.name.clone(),
                region_index: None,
                note_index: None,
                message: "Track has no notes".into(),
            });
            continue;
        }

        // Check overlapping regions on the same track
        let mut sorted_regions: Vec<&crate::model::song::Region> = track.regions.iter().collect();
        sorted_regions.sort_by_key(|r| r.start_tick);
        for w in sorted_regions.windows(2) {
            let a_end = w[0].start_tick + w[0].duration_ticks;
            if a_end > w[1].start_tick {
                errors.push(ExportError {
                    track_name: track.name.clone(),
                    region_index: None,
                    note_index: None,
                    message: format!("Overlapping regions at tick {}", w[1].start_tick),
                });
            }
        }

        // Check each note
        for (ri, region) in track.regions.iter().enumerate() {
            for (ni, note) in region.notes.iter().enumerate() {
                // Pitch range
                let is_dac = matches!(track.channel, ChannelAssignment::Dac(_));
                if !is_dac && midi_to_smps_note(note.pitch).is_none() {
                    errors.push(ExportError {
                        track_name: track.name.clone(),
                        region_index: Some(ri),
                        note_index: Some(ni),
                        message: format!("Pitch {} is outside SMPS range (C0-Bb7, MIDI 12-95)", note.pitch),
                    });
                }

                // Duration quantization
                if !is_dac {
                    if daw_to_smps_duration(note.duration_ticks, params).is_none() {
                        errors.push(ExportError {
                            track_name: track.name.clone(),
                            region_index: Some(ri),
                            note_index: Some(ni),
                            message: format!("Note duration {} DAW ticks rounds to 0 SMPS ticks", note.duration_ticks),
                        });
                    }
                }
            }
        }
    }

    errors
}
```

- [ ] **Step 2: Add tests**

```rust
    #[test]
    fn test_validate_missing_instrument() {
        use crate::model::song::*;
        let song = Song {
            metadata: SongMetadata {
                name: "Test".into(), tempo: 120.0, time_signature: (4, 4),
                ticks_per_beat: 480, driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
                instrument_id: None, regions: vec![], muted: false, solo: false,
                volume: 100, pan: Pan::Center,
            }],
            instruments: InstrumentBank { fm: vec![], psg: vec![], dac: vec![] },
        };
        let params = compute_tempo_params(120.0, 480);
        let errors = validate_for_export(&song, &song.instruments, &params);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("No instrument assigned"));
    }

    #[test]
    fn test_validate_pitch_out_of_range() {
        use crate::model::song::*;
        let inst_id = Uuid::new_v4();
        let song = Song {
            metadata: SongMetadata {
                name: "Test".into(), tempo: 120.0, time_signature: (4, 4),
                ticks_per_beat: 480, driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
                instrument_id: Some(inst_id), regions: vec![Region {
                    id: Uuid::new_v4(), start_tick: 0, duration_ticks: 480,
                    notes: vec![Note { tick: 0, pitch: 5, velocity: 100, duration_ticks: 480 }],
                }],
                muted: false, solo: false, volume: 100, pan: Pan::Center,
            }],
            instruments: InstrumentBank {
                fm: vec![FmInstrument {
                    id: inst_id, name: "Test".into(), algorithm: 0, feedback: 0,
                    operators: [FmOperator::default(); 4],
                    metadata: InstrumentMetadata { category: String::new(), author: String::new(), tags: vec![] },
                }],
                psg: vec![], dac: vec![],
            },
        };
        let params = compute_tempo_params(120.0, 480);
        let errors = validate_for_export(&song, &song.instruments, &params);
        assert!(errors.iter().any(|e| e.message.contains("outside SMPS range")));
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib export::smps::tests`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/export/smps.rs
git commit -m "feat(export): pre-export validation for SMPS constraints"
```

---

### Task 6: Full SMPS Assembly File Generation

**Files:**
- Modify: `src-tauri/src/export/smps.rs`

- [ ] **Step 1: Add music file assembly generation**

Add to `smps.rs`:

```rust
/// Convert pan setting to SMPS panning byte.
fn pan_to_smps(pan: &Pan) -> &'static str {
    match pan {
        Pan::Left => "panLeft",
        Pan::Right => "panRight",
        Pan::Center => "panCenter",
    }
}

/// Format SmpsEvents into dc.b lines of assembly.
fn format_channel_data(events: &[SmpsEvent]) -> String {
    let mut asm = String::new();
    let mut line_items: Vec<String> = Vec::new();
    let mut last_duration: Option<u64> = None;

    for event in events {
        match event {
            SmpsEvent::SetVoice(idx) => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsSetvoice\t${idx:02X}\n"));
            }
            SmpsEvent::SetPan(pan_byte) => {
                flush_line(&mut asm, &mut line_items);
                let pan_name = match pan_byte {
                    0x80 => "panLeft",
                    0x40 => "panRight",
                    0xC0 => "panCenter",
                    _ => "panCenter",
                };
                asm.push_str(&format!("\tsmpsPan\t\t\t{pan_name}, $00\n"));
            }
            SmpsEvent::SetPsgVoice(idx) => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsSetPSGVoice\t${idx:02X}\n"));
            }
            SmpsEvent::Tie => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\t{SMPS_NO_ATTACK}\n"));
            }
            SmpsEvent::Note { pitch_name, duration } => {
                if last_duration == Some(*duration) {
                    line_items.push(pitch_name.clone());
                } else {
                    line_items.push(pitch_name.clone());
                    line_items.push(format!("${duration:02X}"));
                    last_duration = Some(*duration);
                }
                if line_items.len() >= 12 {
                    flush_line(&mut asm, &mut line_items);
                }
            }
            SmpsEvent::Rest { duration } => {
                if last_duration == Some(*duration) {
                    line_items.push("nRst".into());
                } else {
                    line_items.push("nRst".into());
                    line_items.push(format!("${duration:02X}"));
                    last_duration = Some(*duration);
                }
                if line_items.len() >= 12 {
                    flush_line(&mut asm, &mut line_items);
                }
            }
            SmpsEvent::Stop => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str("\tsmpsStop\n");
            }
        }
    }
    flush_line(&mut asm, &mut line_items);
    asm
}

fn flush_line(asm: &mut String, items: &mut Vec<String>) {
    if items.is_empty() { return; }
    asm.push_str(&format!("\tdc.b {}\n", items.join(", ")));
    items.clear();
}

/// Channel type label for assembly comments.
fn channel_type_label(ch: &ChannelAssignment) -> &'static str {
    match ch {
        ChannelAssignment::Fm(_) => "FM",
        ChannelAssignment::Psg(_) => "PSG",
        ChannelAssignment::PsgNoise => "PSG Noise",
        ChannelAssignment::Dac(_) => "DAC",
    }
}

/// Channel index for assembly labels (1-based).
fn channel_index(ch: &ChannelAssignment) -> u8 {
    match ch {
        ChannelAssignment::Fm(n) => n + 1,
        ChannelAssignment::Psg(n) => n + 1,
        ChannelAssignment::PsgNoise => 4,
        ChannelAssignment::Dac(n) => n + 1,
    }
}

/// Generate the complete music assembly file.
pub fn generate_music_asm(
    song: &Song,
    instruments: &InstrumentBank,
    voice_map: &HashMap<Uuid, u8>,
    params: &SmpsTempoParams,
) -> Result<String, Vec<ExportError>> {
    let label = sanitize_label(&song.metadata.name);
    let mut asm = String::new();

    // Header comment
    asm.push_str(&format!("; ============================================================\n"));
    asm.push_str(&format!("; Song: {}\n", song.metadata.name));
    asm.push_str(&format!("; Exported from MegaDAW\n"));
    asm.push_str(&format!("; ============================================================\n\n"));

    // Count active channels
    let active_tracks: Vec<&Track> = song.tracks.iter().filter(|t| !t.muted && t.regions.iter().any(|r| !r.notes.is_empty())).collect();
    let fm_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Fm(_))).count();
    let dac_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Dac(_))).count();
    let psg_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise)).count();
    let fm_dac_count = fm_count + dac_count;

    // Song header
    asm.push_str(&format!("Snd_{label}_Header:\n"));
    asm.push_str(&format!("\tsmpsHeaderStartSong 3\n"));
    asm.push_str(&format!("\tsmpsHeaderVoice\t\tSnd_{label}_Voices\n"));
    asm.push_str(&format!("\tsmpsHeaderChan\t\t${fm_dac_count:02X}, ${psg_count:02X}\n"));
    asm.push_str(&format!("\tsmpsHeaderTempo\t\t${:02X}, ${:02X}\n", params.divider, params.modifier));

    // Channel headers — DAC first, then FM, then PSG
    for track in &active_tracks {
        if !matches!(track.channel, ChannelAssignment::Dac(_)) { continue; }
        let ch_label = format!("Snd_{label}_DAC{}", channel_index(&track.channel));
        asm.push_str(&format!("\tsmpsHeaderDAC\t\t{ch_label}, $00, $00\n"));
    }
    for track in &active_tracks {
        if !matches!(track.channel, ChannelAssignment::Fm(_)) { continue; }
        let ch_label = format!("Snd_{label}_FM{}", channel_index(&track.channel));
        let vol = 0xFF - ((track.volume as u16 * 0x7F / 100) as u8);
        asm.push_str(&format!("\tsmpsHeaderFM\t\t{ch_label}, $00, ${vol:02X}\n"));
    }
    for track in &active_tracks {
        if !matches!(track.channel, ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise) { continue; }
        let ch_label = format!("Snd_{label}_PSG{}", channel_index(&track.channel));
        let vol = 0x0F - ((track.volume as u16 * 0x0F / 100) as u8);
        asm.push_str(&format!("\tsmpsHeaderPSG\t\t{ch_label}, $00, ${vol:02X}, $00, $00\n"));
    }

    asm.push('\n');

    // Channel data
    let mut all_errors = Vec::new();

    for track in &active_tracks {
        let type_label = channel_type_label(&track.channel);
        let ch_idx = channel_index(&track.channel);
        let ch_label = format!("Snd_{label}_{type_label}{ch_idx}");

        asm.push_str(&format!("; ------------------------------------------------------------\n"));
        asm.push_str(&format!("; {type_label} Channel {ch_idx} - \"{}\"\n", track.name));
        asm.push_str(&format!("; ------------------------------------------------------------\n"));
        asm.push_str(&format!("{ch_label}:\n"));

        // Build event list
        let mut events: Vec<SmpsEvent> = Vec::new();

        // Set voice/instrument
        match &track.channel {
            ChannelAssignment::Fm(_) => {
                if let Some(inst_id) = &track.instrument_id {
                    if let Some(&voice_idx) = voice_map.get(inst_id) {
                        events.push(SmpsEvent::SetVoice(voice_idx));
                    }
                }
            }
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => {
                events.push(SmpsEvent::SetPsgVoice(0));
            }
            _ => {}
        }

        // Set panning (FM only)
        if matches!(track.channel, ChannelAssignment::Fm(_)) {
            let pan_byte = match track.pan {
                Pan::Left => 0x80u8,
                Pan::Right => 0x40,
                Pan::Center => 0xC0,
            };
            events.push(SmpsEvent::SetPan(pan_byte));
        }

        // Flatten all regions into a single sorted note list
        let mut all_notes: Vec<(u64, u8, u64)> = Vec::new();
        for region in &track.regions {
            for note in &region.notes {
                let abs_tick = region.start_tick + note.tick;
                all_notes.push((abs_tick, note.pitch, note.duration_ticks));
            }
        }
        all_notes.sort_by_key(|&(tick, _, _)| tick);

        // Find total duration (end of last region)
        let total_duration = track.regions.iter()
            .map(|r| r.start_tick + r.duration_ticks)
            .max()
            .unwrap_or(0);

        // Encode notes
        match encode_channel_events(&all_notes, total_duration, params) {
            Ok(note_events) => events.extend(note_events),
            Err(e) => {
                let mut err = e;
                err.track_name = track.name.clone();
                all_errors.push(err);
                continue;
            }
        }

        events.push(SmpsEvent::Stop);

        asm.push_str(&format_channel_data(&events));
        asm.push('\n');
    }

    if !all_errors.is_empty() {
        return Err(all_errors);
    }

    Ok(asm)
}

/// Sanitize a song name into a valid assembly label.
fn sanitize_label(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}
```

- [ ] **Step 2: Add integration test**

```rust
    #[test]
    fn test_generate_music_asm_basic() {
        use crate::model::song::*;

        let inst_id = Uuid::new_v4();
        let song = Song {
            metadata: SongMetadata {
                name: "TestSong".into(), tempo: 120.0, time_signature: (4, 4),
                ticks_per_beat: 480, driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
                instrument_id: Some(inst_id), regions: vec![Region {
                    id: Uuid::new_v4(), start_tick: 0, duration_ticks: 960,
                    notes: vec![
                        Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 480 },
                        Note { tick: 480, pitch: 64, velocity: 100, duration_ticks: 480 },
                    ],
                }],
                muted: false, solo: false, volume: 100, pan: Pan::Center,
            }],
            instruments: InstrumentBank {
                fm: vec![FmInstrument {
                    id: inst_id, name: "TestPatch".into(), algorithm: 4, feedback: 7,
                    operators: [FmOperator::default(); 4],
                    metadata: InstrumentMetadata { category: String::new(), author: String::new(), tags: vec![] },
                }],
                psg: vec![], dac: vec![],
            },
        };

        let params = compute_tempo_params(120.0, 480);
        let (voice_map, _voices) = build_voice_index(&song.tracks, &song.instruments);
        let asm = generate_music_asm(&song, &song.instruments, &voice_map, &params).unwrap();

        assert!(asm.contains("Snd_TestSong_Header:"));
        assert!(asm.contains("smpsHeaderStartSong 3"));
        assert!(asm.contains("smpsSetvoice"));
        assert!(asm.contains("smpsPan"));
        assert!(asm.contains("nC4"));
        assert!(asm.contains("nE4"));
        assert!(asm.contains("smpsStop"));
    }
```

- [ ] **Step 3: Run tests**

Run: `cd src-tauri && cargo test --lib export::smps::tests`
Expected: All tests pass

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/export/smps.rs
git commit -m "feat(export): full SMPS music assembly file generation"
```

---

### Task 7: Flamedriver export_song Implementation

**Files:**
- Modify: `src-tauri/src/driver/flamedriver.rs`
- Modify: `src-tauri/src/export/smps.rs`

- [ ] **Step 1: Add file writing function to smps.rs**

Add to `smps.rs`:

```rust
use std::fs;

/// Write all export files to the output directory.
pub fn write_export(
    song: &Song,
    instruments: &InstrumentBank,
    driver: &crate::driver::flamedriver::FlamedriverProfile,
    output_dir: &Path,
) -> Result<ExportResult, Vec<ExportError>> {
    let params = compute_tempo_params(song.metadata.tempo, song.metadata.ticks_per_beat);

    // Validate
    let errors = validate_for_export(song, instruments, &params);
    if !errors.is_empty() {
        return Err(errors);
    }

    // Build voice bank
    let (voice_map, voices) = build_voice_index(&song.tracks, instruments);

    // Generate assembly
    let music_asm = generate_music_asm(song, instruments, &voice_map, &params)?;
    let voice_asm = generate_voice_bank_asm(&sanitize_label(&song.metadata.name), &voices, driver);

    // Write files
    let label = sanitize_label(&song.metadata.name);
    fs::create_dir_all(output_dir).map_err(|e| vec![ExportError {
        track_name: String::new(), region_index: None, note_index: None,
        message: format!("Failed to create output directory: {e}"),
    }])?;

    let music_path = output_dir.join(format!("Mus - {}.asm", song.metadata.name));
    let voice_path = output_dir.join(format!("Voices - {}.asm", song.metadata.name));

    fs::write(&music_path, &music_asm).map_err(|e| vec![ExportError {
        track_name: String::new(), region_index: None, note_index: None,
        message: format!("Failed to write music file: {e}"),
    }])?;
    fs::write(&voice_path, &voice_asm).map_err(|e| vec![ExportError {
        track_name: String::new(), region_index: None, note_index: None,
        message: format!("Failed to write voice bank file: {e}"),
    }])?;

    let mut files = vec![
        music_path.to_string_lossy().into_owned(),
        voice_path.to_string_lossy().into_owned(),
    ];

    // Copy DAC samples
    let dac_tracks: Vec<&Track> = song.tracks.iter()
        .filter(|t| !t.muted && matches!(t.channel, ChannelAssignment::Dac(_)))
        .collect();
    if !dac_tracks.is_empty() {
        let dac_dir = output_dir.join("dac");
        fs::create_dir_all(&dac_dir).map_err(|e| vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: format!("Failed to create dac directory: {e}"),
        }])?;

        for track in dac_tracks {
            if let Some(inst_id) = &track.instrument_id {
                if let Some(inst) = instruments.dac.iter().find(|i| &i.id == inst_id) {
                    let pcm_src = Path::new(&inst.pcm_file);
                    if pcm_src.exists() {
                        let dest = dac_dir.join(pcm_src.file_name().unwrap_or_default());
                        if let Err(e) = fs::copy(pcm_src, &dest) {
                            return Err(vec![ExportError {
                                track_name: track.name.clone(), region_index: None, note_index: None,
                                message: format!("Failed to copy DAC sample: {e}"),
                            }]);
                        }
                        files.push(dest.to_string_lossy().into_owned());
                    }
                }
            }
        }
    }

    Ok(ExportResult { files })
}
```

- [ ] **Step 2: Replace FlamedriverProfile stub with real implementation**

In `src-tauri/src/driver/flamedriver.rs`, replace the stub `export_song` with:

```rust
fn export_song(
    &self,
    song: &Song,
    instruments: &InstrumentBank,
    output_dir: &Path,
) -> Result<ExportResult, Vec<ExportError>> {
    crate::export::smps::write_export(song, instruments, self, output_dir)
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles (warnings OK)

- [ ] **Step 4: Run all tests**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/export/smps.rs src-tauri/src/driver/flamedriver.rs
git commit -m "feat(export): wire FlamedriverProfile::export_song() to SMPS file writer"
```

---

### Task 8: Export IPC Command

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/ipc/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add export_song IPC command**

In `src-tauri/src/ipc/commands.rs`, add the import:

```rust
use crate::export::{ExportResult, ExportError};
use crate::model::driver::DriverProfile;
```

Add the command:

```rust
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFailure {
    pub errors: Vec<ExportError>,
}

#[tauri::command]
pub fn export_song(
    state: State<'_, ProjectState>,
    output_dir: String,
) -> Result<ExportResult, ExportFailure> {
    let mgr = state.manager.lock().map_err(|e| ExportFailure {
        errors: vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: format!("mutex poisoned: {e}"),
        }],
    })?;

    let song = mgr.song().ok_or_else(|| ExportFailure {
        errors: vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: "No project open".into(),
        }],
    })?;

    let driver_id = &song.metadata.driver_id;
    let registry = mgr.driver_registry();
    let driver = registry.get(driver_id).ok_or_else(|| ExportFailure {
        errors: vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: format!("Driver '{driver_id}' not found"),
        }],
    })?;

    let path = std::path::PathBuf::from(&output_dir);
    driver.export_song(&song, &song.instruments, &path).map_err(|errors| ExportFailure { errors })
}
```

- [ ] **Step 2: Add re-export in mod.rs**

In `src-tauri/src/ipc/mod.rs`, add `export_song` to the re-export list (in a new `// Export` section).

- [ ] **Step 3: Register command in lib.rs**

In `src-tauri/src/lib.rs`, add `export_song` to both the `use` import and the `generate_handler!` macro.

- [ ] **Step 4: Verify it compiles**

Run: `cd src-tauri && cargo check`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/ipc/commands.rs src-tauri/src/ipc/mod.rs src-tauri/src/lib.rs
git commit -m "feat(export): add export_song IPC command"
```

---

### Task 9: Frontend Export UI

**Files:**
- Modify: `src/api/ipc.ts`
- Modify: `src/components/TopBar.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Add IPC wrapper**

In `src/api/ipc.ts`, add the types and function:

```typescript
export interface ExportResult {
  files: string[];
}

export interface ExportError {
  trackName: string;
  regionIndex: number | null;
  noteIndex: number | null;
  message: string;
}

export async function exportSong(outputDir: string): Promise<ExportResult> {
  return invoke<ExportResult>("export_song", { outputDir });
}
```

- [ ] **Step 2: Add Export button to TopBar**

In `src/components/TopBar.tsx`, add the `onExport` prop and button:

Add to `TopBarProps`:
```typescript
onExport?: () => void;
```

Add the Export button after the Save button:
```tsx
{props.onExport && (
  <button className={styles.btn} onClick={props.onExport}>Export</button>
)}
```

- [ ] **Step 3: Wire export handler in App.tsx**

In `src/App.tsx`, add the export handler and state:

```typescript
const [exportStatus, setExportStatus] = useState<{ type: "success"; files: string[] } | { type: "error"; errors: ipc.ExportError[] } | null>(null);

async function handleExport() {
  if (!projectMeta) return;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const selected = await open({ directory: true, title: "Export Song" });
  if (!selected) return;
  try {
    const result = await ipc.exportSong(selected as string);
    setExportStatus({ type: "success", files: result.files });
    setTimeout(() => setExportStatus(null), 4000);
  } catch (e: any) {
    if (e?.errors) {
      setExportStatus({ type: "error", errors: e.errors });
    } else {
      setExportStatus({ type: "error", errors: [{ trackName: "", regionIndex: null, noteIndex: null, message: String(e) }] });
    }
  }
}
```

Pass to TopBar:
```tsx
onExport={projectOpen ? handleExport : undefined}
```

Add status display after TopBar (inside the app div):
```tsx
{exportStatus?.type === "success" && (
  <div className={styles.exportSuccess}>
    Exported {exportStatus.files.length} files
  </div>
)}
{exportStatus?.type === "error" && (
  <div className={styles.exportError}>
    <div className={styles.exportErrorHeader}>
      <span>Export failed ({exportStatus.errors.length} errors)</span>
      <button onClick={() => setExportStatus(null)}>x</button>
    </div>
    <ul>
      {exportStatus.errors.map((e, i) => (
        <li key={i}>{e.trackName ? `${e.trackName}: ` : ""}{e.message}</li>
      ))}
    </ul>
  </div>
)}
```

- [ ] **Step 4: Add styles for export status**

In `src/App.module.css`, add:

```css
.exportSuccess {
  position: fixed;
  top: 48px;
  left: 50%;
  transform: translateX(-50%);
  background: #2a5a2a;
  color: #88ff88;
  padding: 8px 20px;
  border-radius: 4px;
  font-size: 13px;
  z-index: 100;
}

.exportError {
  position: fixed;
  top: 48px;
  right: 16px;
  background: #3a1a1a;
  border: 1px solid #ff4444;
  color: #ffaaaa;
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 13px;
  max-width: 400px;
  max-height: 300px;
  overflow-y: auto;
  z-index: 100;
}

.exportErrorHeader {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-weight: 600;
}

.exportErrorHeader button {
  background: none;
  border: none;
  color: #ffaaaa;
  cursor: pointer;
  font-size: 14px;
}

.exportError ul {
  margin: 0;
  padding-left: 16px;
}

.exportError li {
  margin: 4px 0;
}
```

- [ ] **Step 5: Verify TypeScript compiles**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 6: Commit**

```bash
git add src/api/ipc.ts src/components/TopBar.tsx src/App.tsx src/App.module.css
git commit -m "feat(export): add Export button, directory picker, and success/error display"
```

---

### Task 10: Integration Test & Final Verification

- [ ] **Step 1: Run full Rust test suite**

Run: `cd src-tauri && cargo test`
Expected: All tests pass

- [ ] **Step 2: Run TypeScript check**

Run: `npx tsc --noEmit`
Expected: No errors

- [ ] **Step 3: Build full app**

Run: `cd src-tauri && cargo build`
Expected: Builds successfully

- [ ] **Step 4: Commit any final fixes**

- [ ] **Step 5: Final commit**

```bash
git commit -m "feat(export): Phase 5 Flamedriver Export complete"
```
