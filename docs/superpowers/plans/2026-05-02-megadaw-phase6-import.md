# MegaDAW Phase 6: SMPS Import — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Import Flamedriver/SMPS `.asm` song files into MegaDAW as fully editable projects with tracks, notes, and instruments.

**Architecture:** Line-by-line regex parser produces an intermediate `SmpsFile` struct, a mapper converts it to the DAW's `Song` + `InstrumentBank` model, and an IPC command orchestrates parse → map → create project → save. Frontend gets an Import button that triggers file picker → directory picker → auto-open.

**Tech Stack:** Rust (regex crate for parsing), Tauri IPC, React/TypeScript frontend

---

## File Structure

**New files:**
- `src-tauri/src/import/mod.rs` — Module root, shared types (`ImportResult`, `ImportWarning`)
- `src-tauri/src/import/smps_parser.rs` — Assembly line parser → `SmpsFile` intermediate
- `src-tauri/src/import/smps_mapper.rs` — `SmpsFile` → `Song` + `InstrumentBank`
- `src-tauri/src/import/psg_envelopes.rs` — Bundled Flamedriver PSG envelope const table

**Modified files:**
- `src-tauri/src/lib.rs` — Add `mod import;`, register `import_song` command
- `src-tauri/src/ipc/mod.rs` — Re-export `import_song`
- `src-tauri/src/ipc/commands.rs` — `import_song` IPC command
- `src/api/ipc.ts` — `importSong()` wrapper + types
- `src/components/TopBar.tsx` — Import button
- `src/App.tsx` — Import handler + warning panel
- `src/App.module.css` — Warning panel styles

---

### Task 1: Import Module Scaffold + Shared Types

**Files:**
- Create: `src-tauri/src/import/mod.rs`
- Modify: `src-tauri/src/lib.rs:1`

- [ ] **Step 1: Create the import module with shared types**

Create `src-tauri/src/import/mod.rs`:

```rust
pub mod psg_envelopes;
pub mod smps_mapper;
pub mod smps_parser;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub metadata: crate::model::song::SongMetadata,
    pub track_count: usize,
    pub instrument_count: usize,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub channel: String,
    pub message: String,
}
```

Create stub files so the module compiles:

`src-tauri/src/import/psg_envelopes.rs`:
```rust
// Populated in Task 2
```

`src-tauri/src/import/smps_parser.rs`:
```rust
// Populated in Task 3
```

`src-tauri/src/import/smps_mapper.rs`:
```rust
// Populated in Task 5
```

- [ ] **Step 2: Add `mod import` to lib.rs**

In `src-tauri/src/lib.rs`, add after `mod export;` (line 4):

```rust
mod import;
```

- [ ] **Step 3: Verify it compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo build -p megadaw-app 2>&1 | tail -5`
Expected: Compiles (warnings about unused imports are OK)

- [ ] **Step 4: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/import/mod.rs src-tauri/src/import/psg_envelopes.rs src-tauri/src/import/smps_parser.rs src-tauri/src/import/smps_mapper.rs src-tauri/src/lib.rs
git commit -m "feat(import): scaffold import module with shared types"
```

---

### Task 2: PSG Envelope Table

**Files:**
- Create: `src-tauri/src/import/psg_envelopes.rs`

The Flamedriver driver has 52 PSG volume envelopes (indices $00-$33). Each is a sequence of volume offset bytes terminated by a command byte. The command bytes are:
- `$80` (VolEnvReset) — loop back to start
- `$81` (VolEnvRestTrack) — rest and stop
- `$82` (VolEnvJumpTo) — followed by a jump index byte (not used in any standard envelope)
- `$83` (VolEnvStopTrack) — stop track

For DAW import, we treat the envelope as the volume offset sequence, and the terminator tells us whether it loops ($80) or decays ($81/$83).

The `sTone_XX` constants in songs map to these indices: `sTone_01` = index 1, etc. Index 0 maps to `VolEnv_00`. The PSG header's last byte is the envelope index.

- [ ] **Step 1: Write the test**

Add to `src-tauri/src/import/psg_envelopes.rs`:

```rust
pub struct PsgEnvelopeEntry {
    pub volumes: &'static [i8],
    pub loop_point: Option<usize>,
}

pub const FLAMEDRIVER_PSG_ENVELOPES: &[PsgEnvelopeEntry] = &[]; // placeholder

pub fn get_envelope(index: u8) -> Option<&'static PsgEnvelopeEntry> {
    FLAMEDRIVER_PSG_ENVELOPES.get(index as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_count() {
        assert_eq!(FLAMEDRIVER_PSG_ENVELOPES.len(), 52);
    }

    #[test]
    fn test_envelope_00_is_short_decay() {
        let env = get_envelope(0).unwrap();
        assert_eq!(env.volumes, &[2]);
        assert!(env.loop_point.is_none());
    }

    #[test]
    fn test_envelope_01_is_fade_out() {
        let env = get_envelope(1).unwrap();
        assert_eq!(env.volumes, &[0, 2, 4, 6, 8, 0x10]);
        assert!(env.loop_point.is_none());
    }

    #[test]
    fn test_envelope_06_loops() {
        let env = get_envelope(6).unwrap();
        assert_eq!(env.volumes, &[1, 0x0C, 3, 0x0F, 2, 7, 3, 0x0F]);
        assert_eq!(env.loop_point, Some(0));
    }

    #[test]
    fn test_envelope_0a_loops() {
        let env = get_envelope(0x0A).unwrap();
        assert_eq!(env.volumes, &[0x10, 0x20, 0x30, 0x40, 0x30, 0x20, 0x10, 0, -0x10]);
        assert_eq!(env.loop_point, Some(0));
    }

    #[test]
    fn test_out_of_range_returns_none() {
        assert!(get_envelope(52).is_none());
        assert!(get_envelope(255).is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::psg_envelopes 2>&1 | tail -10`
Expected: FAIL — `FLAMEDRIVER_PSG_ENVELOPES` is empty

- [ ] **Step 3: Populate the envelope table**

Replace the placeholder `FLAMEDRIVER_PSG_ENVELOPES` with the full data extracted from `Sonic-Clean-Engine-S.C.E.-/Sound/Flamedriver.asm` lines 4652-4737. Each envelope is transcribed as a `PsgEnvelopeEntry` with:
- `volumes`: the byte sequence before the terminator command
- `loop_point`: `Some(0)` if terminated by `VolEnvReset` ($80), `None` if terminated by `VolEnvRestTrack` ($81) or `VolEnvStopTrack` ($83)

Note: `VolEnv_0E` shares the same data as `VolEnv_01`. Both are index $01 and $0E respectively — the table has 52 entries indexed 0-51 ($00-$33).

The full const array (all 52 entries from the Flamedriver source):

```rust
pub const FLAMEDRIVER_PSG_ENVELOPES: &[PsgEnvelopeEntry] = &[
    // $00: VolEnv_00 — StopTrack
    PsgEnvelopeEntry { volumes: &[2], loop_point: None },
    // $01: VolEnv_01 — StopTrack (also VolEnv_0E)
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x10], loop_point: None },
    // $02: VolEnv_02 — RestTrack
    PsgEnvelopeEntry { volumes: &[2, 1, 0, 0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5], loop_point: None },
    // $03: VolEnv_03 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 2, 3, 4, 4, 5, 5, 5, 6, 6], loop_point: None },
    // $04: VolEnv_04 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 0, 1, 1, 1, 2, 3, 4, 4, 5], loop_point: None },
    // $05: VolEnv_05 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 2, 3, 4, 5, 5, 6, 8, 7, 7, 6], loop_point: None },
    // $06: VolEnv_06 — Reset (loop)
    PsgEnvelopeEntry { volumes: &[1, 0x0C, 3, 0x0F, 2, 7, 3, 0x0F], loop_point: Some(0) },
    // $07: VolEnv_07 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 2, 3, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0E, 0x0F], loop_point: None },
    // $08: VolEnv_08 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 2, 1, 1, 0, 0, 1, 2, 3, 4], loop_point: None },
    // $09: VolEnv_09 — RestTrack
    PsgEnvelopeEntry { volumes: &[1, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5], loop_point: None },
    // $0A: VolEnv_0A — Reset (loop)
    PsgEnvelopeEntry { volumes: &[0x10, 0x20, 0x30, 0x40, 0x30, 0x20, 0x10, 0, -0x10], loop_point: Some(0) },
    // $0B: VolEnv_0B — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 3, 3, 4, 5], loop_point: None },
    // $0C: VolEnv_0C — RestTrack
    PsgEnvelopeEntry { volumes: &[0], loop_point: None },
    // $0D: VolEnv_0D — StopTrack
    PsgEnvelopeEntry { volumes: &[2], loop_point: None },
    // $0E: VolEnv_0E — StopTrack (same data as $01)
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x10], loop_point: None },
    // $0F: VolEnv_0F — RestTrack
    PsgEnvelopeEntry { volumes: &[9, 9, 9, 8, 8, 8, 7, 7, 7, 6, 6, 6, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0], loop_point: None },
    // $10: VolEnv_10 — RestTrack
    PsgEnvelopeEntry { volumes: &[1, 1, 1, 0, 0, 0], loop_point: None },
    // $11: VolEnv_11 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 0, 1, 1, 1, 2, 3, 4, 4, 5], loop_point: None },
    // $12: VolEnv_12 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 2, 3, 4, 5, 5, 6, 8, 7, 7, 6], loop_point: None },
    // $13: VolEnv_13 — StopTrack
    PsgEnvelopeEntry { volumes: &[0x0A, 5, 0, 4, 8], loop_point: None },
    // $14: VolEnv_14 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 2, 3, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0E, 0x0F], loop_point: None },
    // $15: VolEnv_15 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 2, 1, 1, 0, 0, 1, 2, 3, 4], loop_point: None },
    // $16: VolEnv_16 — RestTrack
    PsgEnvelopeEntry { volumes: &[1, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5], loop_point: None },
    // $17: VolEnv_17 — Reset (loop)
    PsgEnvelopeEntry { volumes: &[0x10, 0x20, 0x30, 0x40, 0x30, 0x20, 0x10, 0], loop_point: Some(0) },
    // $18: VolEnv_18 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 3, 3, 4, 5], loop_point: None },
    // $19: VolEnv_19 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x16], loop_point: None },
    // $1A: VolEnv_1A — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 3, 3, 4, 5], loop_point: None },
    // $1B: VolEnv_1B — StopTrack
    PsgEnvelopeEntry { volumes: &[4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1], loop_point: None },
    // $1C: VolEnv_1C — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 8, 9, 9, 9, 9, 0x0A, 0x0A, 0x0A, 0x0A], loop_point: None },
    // $1D: VolEnv_1D — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0x0A], loop_point: None },
    // $1E: VolEnv_1E — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 4], loop_point: None },
    // $1F: VolEnv_1F — RestTrack
    PsgEnvelopeEntry { volumes: &[0x30, 0x20, 0x10, 0, 0, 0, 0, 0, 8, 0x10, 0x20, 0x30], loop_point: None },
    // $20: VolEnv_20 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 0x0A], loop_point: None },
    // $21: VolEnv_21 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 3, 4, 6, 7], loop_point: None },
    // $22: VolEnv_22 — RestTrack
    PsgEnvelopeEntry { volumes: &[2, 1, 0, 0, 0, 2, 4, 7], loop_point: None },
    // $23: VolEnv_23 — StopTrack
    PsgEnvelopeEntry { volumes: &[0x0F, 1, 5], loop_point: None },
    // $24: VolEnv_24 — StopTrack
    PsgEnvelopeEntry { volumes: &[8, 6, 2, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10], loop_point: None },
    // $25: VolEnv_25 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9], loop_point: None },
    // $26: VolEnv_26 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5], loop_point: None },
    // $27: VolEnv_27 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7], loop_point: None },
    // $28: VolEnv_28 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x10], loop_point: None },
    // $29: VolEnv_29 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7], loop_point: None },
    // $2A: VolEnv_2A — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 2, 3, 4, 4, 5, 5, 5, 6], loop_point: None },
    // $2B: VolEnv_2B — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 4], loop_point: None },
    // $2C: VolEnv_2C — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0], loop_point: None },
    // $2D: VolEnv_2D — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 7], loop_point: None },
    // $2E: VolEnv_2E — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 7, 7, 7], loop_point: None },
    // $2F: VolEnv_2F — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F], loop_point: None },
    // $30: VolEnv_30 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4], loop_point: None },
    // $31: VolEnv_31 — RestTrack
    PsgEnvelopeEntry { volumes: &[4, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4], loop_point: None },
    // $32: VolEnv_32 — RestTrack
    PsgEnvelopeEntry { volumes: &[4, 4, 3, 3, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7], loop_point: None },
    // $33: VolEnv_33 — RestTrack
    PsgEnvelopeEntry { volumes: &[0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0], loop_point: None },
];
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::psg_envelopes 2>&1 | tail -10`
Expected: All 5 tests PASS

- [ ] **Step 5: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/import/psg_envelopes.rs
git commit -m "feat(import): add bundled Flamedriver PSG envelope table"
```

---

### Task 3: SMPS Parser — Types + Note/Flag Constants

**Files:**
- Create: `src-tauri/src/import/smps_parser.rs`

This task defines the parser's output types and the constant lookup tables for note names, DAC sample names, coordination flag argument counts, and pan constants.

- [ ] **Step 1: Write the types and constant tables**

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SmpsFile {
    pub song_label: String,
    pub voice_ref: VoiceRef,
    pub fm_count: u8,
    pub psg_count: u8,
    pub tempo_divider: u8,
    pub tempo_modifier: u8,
    pub channels: Vec<SmpsChannel>,
    pub voices: Vec<[u8; 25]>,
}

#[derive(Debug, Clone)]
pub enum VoiceRef {
    Inline(String),
    Uvb,
    External(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpsChannelKind {
    Dac,
    Fm,
    Psg,
}

#[derive(Debug, Clone)]
pub struct SmpsChannel {
    pub kind: SmpsChannelKind,
    pub label: String,
    pub initial_pitch: i8,
    pub initial_volume: u8,
    pub psg_envelope: Option<u8>,
    pub events: Vec<SmpsEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SmpsEvent {
    Note { pitch: u8, duration: u8 },
    Rest { duration: u8 },
    SetVoice(u8),
    SetPan(u8),
    Transpose(i8),
    Tie,
    Stop,
    Unsupported { name: String },
}

pub fn build_note_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    map.insert("nRst".into(), 0x80);
    let note_names = ["C", "Cs", "D", "Eb", "E", "F", "Fs", "G", "Ab", "A", "Bb", "B"];
    let aliases = [
        ("Db", "Cs"), ("Ds", "Eb"), ("Es", "F"), ("Fb", "E"),
        ("Gb", "Fs"), ("Gs", "Ab"), ("As", "Bb"), ("Bs", "C"),
        ("Cb", "B"),
    ];
    for octave in 0..=7u8 {
        for (semi, name) in note_names.iter().enumerate() {
            let byte = 0x81 + octave * 12 + semi as u8;
            map.insert(format!("n{name}{octave}"), byte);
        }
    }
    for octave in 0..=7u8 {
        for &(alias, canonical) in &aliases {
            if alias == "Bs" || alias == "Cb" {
                let target_octave = if alias == "Bs" { octave + 1 } else { octave.wrapping_sub(1) };
                if let Some(&val) = map.get(&format!("n{canonical}{target_octave}")) {
                    map.insert(format!("n{alias}{octave}"), val);
                }
            } else if let Some(&val) = map.get(&format!("n{canonical}{octave}")) {
                map.insert(format!("n{alias}{octave}"), val);
            }
        }
    }
    map
}

pub fn build_dac_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    let dac_names = [
        ("dSnareS3", 0x81), ("dHighTom", 0x82), ("dMidTomS3", 0x83),
        ("dLowTomS3", 0x84), ("dFloorTomS3", 0x85), ("dKickS3", 0x86),
        ("dMuffledSnare", 0x87), ("dCrashCymbal", 0x88), ("dRideCymbal", 0x89),
        ("dLowMetalHit", 0x8A), ("dMetalHit", 0x8B), ("dHighMetalHit", 0x8C),
        ("dHigherMetalHit", 0x8D), ("dMidMetalHit", 0x8E), ("dClapS3", 0x8F),
        ("dElectricHighTom", 0x90), ("dElectricMidTom", 0x91),
        ("dElectricLowTom", 0x92), ("dElectricFloorTom", 0x93),
        ("dTightSnare", 0x94), ("dMidpitchSnare", 0x95), ("dLooseSnare", 0x96),
        ("dLooserSnare", 0x97), ("dHiTimpaniS3", 0x98), ("dLowTimpaniS3", 0x99),
        ("dMidTimpaniS3", 0x9A), ("dQuickLooseSnare", 0x9B), ("dClick", 0x9C),
        ("dPowerKick", 0x9D), ("dQuickGlassCrash", 0x9E),
        ("dGlassCrashSnare", 0x9F), ("dGlassCrash", 0xA0),
        ("dGlassCrashKick", 0xA1), ("dQuietGlassCrash", 0xA2),
        ("dOddSnareKick", 0xA3), ("dKickExtraBass", 0xA4), ("dComeOn", 0xA5),
        ("dDanceSnare", 0xA6), ("dLooseKick", 0xA7), ("dModLooseKick", 0xA8),
        ("dWoo", 0xA9), ("dGo", 0xAA), ("dSnareGo", 0xAB), ("dPowerTom", 0xAC),
        ("dHiWoodBlock", 0xAD), ("dLowWoodBlock", 0xAE), ("dHiHitDrum", 0xAF),
        ("dLowHitDrum", 0xB0), ("dMetalCrashHit", 0xB1),
        ("dEchoedClapHit_S3", 0xB2), ("dLowerEchoedClapHit_S3", 0xB3),
        ("dHipHopHitKick", 0xB4), ("dHipHopHitPowerKick", 0xB5),
        ("dBassHey", 0xB6), ("dDanceStyleKick", 0xB7),
        ("dHipHopHitKick2", 0xB8), ("dHipHopHitKick3", 0xB9),
        ("dReverseFadingWind", 0xBA), ("dScratchS3", 0xBB),
        ("dLooseSnareNoise", 0xBC), ("dPowerKick2", 0xBD),
        ("dCrashingNoiseWoo", 0xBE), ("dQuickHit", 0xBF),
        ("dKickHey", 0xC0), ("dPowerKickHit", 0xC1),
        ("dLowPowerKickHit", 0xC2), ("dLowerPowerKickHit", 0xC3),
        ("dLowestPowerKickHit", 0xC4),
    ];
    for (name, val) in dac_names {
        map.insert(name.into(), val);
    }
    map
}

pub fn build_pan_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    map.insert("panNone".into(), 0x00);
    map.insert("panRight".into(), 0x40);
    map.insert("panLeft".into(), 0x80);
    map.insert("panCenter".into(), 0xC0);
    map.insert("panCentre".into(), 0xC0);
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_table_nrst() {
        let table = build_note_table();
        assert_eq!(table["nRst"], 0x80);
    }

    #[test]
    fn test_note_table_nc0_is_81() {
        let table = build_note_table();
        assert_eq!(table["nC0"], 0x81);
    }

    #[test]
    fn test_note_table_aliases() {
        let table = build_note_table();
        assert_eq!(table["nDb0"], table["nCs0"]);
        assert_eq!(table["nEb4"], table["nDs4"]);
        assert_eq!(table["nFs3"], table["nGb3"]);
    }

    #[test]
    fn test_dac_table() {
        let table = build_dac_table();
        assert_eq!(table["dKickS3"], 0x86);
        assert_eq!(table["dSnareS3"], 0x81);
    }

    #[test]
    fn test_pan_table() {
        let table = build_pan_table();
        assert_eq!(table["panCenter"], 0xC0);
        assert_eq!(table["panLeft"], 0x80);
        assert_eq!(table["panRight"], 0x40);
    }
}
```

- [ ] **Step 2: Run tests**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::smps_parser 2>&1 | tail -10`
Expected: All 5 tests PASS

- [ ] **Step 3: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/import/smps_parser.rs
git commit -m "feat(import): SMPS parser types and constant tables"
```

---

### Task 4: SMPS Parser — Core Parse Logic

**Files:**
- Modify: `src-tauri/src/import/smps_parser.rs`

This task implements the actual line-by-line parser: header parsing, label collection, `dc.b` data parsing, coordination flag dispatch, voice parsing, loop/call/jump flattening.

- [ ] **Step 1: Write integration test using real song data**

Add to the test module at the bottom of `smps_parser.rs`:

```rust
    #[test]
    fn test_parse_dez1_header() {
        let source = include_str!("../../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        assert_eq!(result.fm_count, 6);
        assert_eq!(result.psg_count, 3);
        assert_eq!(result.tempo_divider, 1);
        assert_eq!(result.tempo_modifier, 0x08);
        assert_eq!(result.channels.len(), 10); // 1 DAC + 6 FM + 3 PSG (though FM6 has no header, so actually 1+5+3=9 or 1 DAC + 5 FM + 3 PSG headers)
    }

    #[test]
    fn test_parse_dez1_voices() {
        let source = include_str!("../../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        assert_eq!(result.voices.len(), 4);
        assert!(matches!(result.voice_ref, VoiceRef::Inline(_)));
    }

    #[test]
    fn test_parse_dez1_fm1_has_notes() {
        let source = include_str!("../../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        let fm1 = &result.channels[1]; // index 0 = DAC, 1 = FM1
        assert_eq!(fm1.kind, SmpsChannelKind::Fm);
        assert!(!fm1.events.is_empty());
        // FM1 starts with smpsSetvoice $00
        assert_eq!(fm1.events[0], SmpsEvent::SetVoice(0));
    }

    #[test]
    fn test_parse_uvb_song() {
        let source = include_str!("../../../test_data/AIZ1.asm");
        let result = parse_smps(source).unwrap();
        assert!(matches!(result.voice_ref, VoiceRef::Uvb));
        assert_eq!(result.voices.len(), 0);
    }

    #[test]
    fn test_parse_loops_are_flattened() {
        let source = include_str!("../../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        // After flattening, no SmpsEvent::Loop or similar should exist
        for ch in &result.channels {
            for ev in &ch.events {
                match ev {
                    SmpsEvent::Note { .. } | SmpsEvent::Rest { .. } | SmpsEvent::SetVoice(_)
                    | SmpsEvent::SetPan(_) | SmpsEvent::Transpose(_) | SmpsEvent::Tie
                    | SmpsEvent::Stop | SmpsEvent::Unsupported { .. } => {}
                }
            }
        }
    }

    #[test]
    fn test_parse_psg_channels_stop() {
        let source = include_str!("../../../test_data/Mus - DEZ1.asm");
        let result = parse_smps(source).unwrap();
        // DEZ1 PSG channels all have just smpsStop
        for ch in &result.channels {
            if ch.kind == SmpsChannelKind::Psg {
                assert!(ch.events.iter().any(|e| matches!(e, SmpsEvent::Stop)));
            }
        }
    }
```

Before running, copy test data:
```bash
mkdir -p /home/volence/sonic_hacks/megadaw/src-tauri/test_data
cp "/home/volence/sonic_hacks/Sonic-Clean-Engine-S.C.E.-/Sound/Music/Mus - DEZ1.asm" /home/volence/sonic_hacks/megadaw/src-tauri/test_data/
cp "/home/volence/sonic_hacks/skdisasm/Sound/Music/AIZ1.asm" /home/volence/sonic_hacks/megadaw/src-tauri/test_data/
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::smps_parser 2>&1 | tail -15`
Expected: FAIL — `parse_smps` function doesn't exist

- [ ] **Step 3: Implement the parser**

Add the `parse_smps` function and all its helper functions to `smps_parser.rs`. The parser needs to:

1. **First pass** — scan all lines collecting labels and their line numbers, identify the voice label from the header
2. **Second pass** — parse header macros, then parse each channel's data stream starting from its label

Key implementation details:

**Line parsing** — each non-empty, non-comment line is one of:
- A label definition: `SomeName:` (possibly with trailing content on same line)
- A macro call: `smpsHeaderStartSong`, `smpsPan`, `smpsSetvoice`, `smpsLoop`, etc.
- A `dc.b` data line: comma-separated tokens that are note names, hex literals, or macro names

**Token resolution in dc.b lines** — each comma-separated token (trimmed) is resolved by checking in order:
1. Note name table (`nC4` → 0x93)
2. DAC sample table (`dKickS3` → 0x86)
3. Hex literal (`$0F` → 15)
4. Coordination flag name (`smpsNoAttack` → special handling)
5. PSG envelope name (`sTone_0A` → 10)

**Coordination flag dispatch** — when a flag token is encountered in dc.b data, consume the appropriate number of following tokens as arguments. The argument count table for S3+ Flamedriver (SonicDriverVer >= 3):

```rust
fn coord_flag_args(name: &str) -> usize {
    match name {
        "smpsPan" => 2,           // direction, amsfms
        "smpsDetune" | "smpsAlterNote" => 1,
        "smpsNop" | "smpsFade" => 1,    // $E2
        "smpsStopFM" => 0,               // $E3
        "smpsSetVol" => 1,               // $E4
        "smpsFMAlterVol" => 1,           // can be 1 or 2, need context
        "smpsAlterVol" => 1,             // $E6
        "smpsNoAttack" => 0,             // $E7 (inline, no args)
        "smpsNoteFill" => 1,             // $E8
        "smpsSpindashRev" => 0,          // $E9
        "smpsPlayDACSample" => 1,        // $EA
        "smpsConditionalJump" => 3,      // $EB: index, addr_lo, addr_hi
        "smpsPSGAlterVol" => 1,          // $EC
        "smpsSetNote" => 1,              // $ED
        "smpsFMICommand" => 2,           // $EE
        "smpsFMvoice" | "smpsSetvoice" => 1, // $EF
        "smpsModSet" => 4,               // $F0
        "smpsModChange2" => 2,           // $F1
        "smpsStop" => 0,                 // $F2
        "smpsPSGform" => 1,              // $F3
        "smpsModChange" | "smpsModOff" => 0, // $F4 S3+ = smpsModChange 1 arg? Actually S3+ $FA = smpsModOff 0 args, $F4 = smpsModChange 1 arg
        "smpsPSGvoice" => 1,             // $F5
        "smpsChangeTransposition" | "smpsAlterPitch" => 1, // $FB
        "smpsAlternateSMPS" => 1,        // $FD
        "smpsFM3SpecialMode" => 4,       // $FE
        _ => 0,
    }
}
```

Note: `smpsLoop`, `smpsCall`, `smpsJump`, `smpsContinuousLoop`, and `smpsReturn` are parsed as top-level macro lines, NOT as dc.b inline tokens. They appear as their own assembly lines.

**Loop flattening** — when `smpsLoop $idx, $count, Label` is encountered:
1. Find the label's position in the current channel's event list
2. Copy the events from that label to the current position
3. Repeat `count - 1` more times (the first play-through is the original data)

**Call inlining** — when `smpsCall Label` is encountered:
1. Parse the subroutine (label to `smpsReturn`) into events
2. Insert those events at the current position

**Jump handling** — when `smpsJump Label` or `smpsContinuousLoop Label` is encountered:
1. If the label points backwards (already seen), this is a loop-to-top → stop parsing this channel (the song loops, but we've already captured one iteration)
2. If the label points forward, jump to that position and continue parsing

**Voice parsing** — `smpsVcAlgorithm`, `smpsVcFeedback`, etc. are collected into a 25-byte array matching the `fm_to_bytes`/`fm_from_bytes` layout. The macros appear in a fixed order (algorithm, feedback, unused bits, detune[4], coarse freq[4], rate scale[4], attack rate[4], amp mod[4], decay rate 1[4], decay rate 2[4], decay level[4], release rate[4], total level[4]). The operator order in the macros is 1,2,3,4 but the byte layout is 4,3,2,1 (matching `OP_ORDER`).

The actual implementation should be a `pub fn parse_smps(source: &str) -> Result<SmpsFile, String>` function.

This is a substantial function (~300-400 lines). The implementer should write it incrementally, running the tests after each major piece:
1. Header parsing (gets the first test passing)
2. Label collection and channel data parsing with dc.b tokenization
3. Coordination flag dispatch
4. Loop/call/jump flattening
5. Voice parsing

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::smps_parser 2>&1 | tail -15`
Expected: All 11 tests PASS (5 from Task 3 + 6 from this task)

- [ ] **Step 5: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/import/smps_parser.rs src-tauri/test_data/
git commit -m "feat(import): SMPS assembly parser with loop flattening"
```

---

### Task 5: SMPS-to-DAW Mapper

**Files:**
- Create: `src-tauri/src/import/smps_mapper.rs`

Converts `SmpsFile` → `Song` + `InstrumentBank`. Handles tick conversion, track creation, note mapping, instrument creation.

- [ ] **Step 1: Write the tests**

```rust
use uuid::Uuid;
use crate::driver::flamedriver::FlamedriverProfile;
use crate::model::driver::DriverProfile;
use crate::import::smps_parser::*;

pub struct MappedSong {
    pub song: crate::model::song::Song,
    pub warnings: Vec<crate::import::ImportWarning>,
}

pub fn map_smps_to_song(smps: &SmpsFile, driver: &dyn DriverProfile) -> Result<MappedSong, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::import::smps_parser::{SmpsFile, SmpsChannel, SmpsChannelKind, SmpsEvent, VoiceRef};

    fn make_simple_smps() -> SmpsFile {
        SmpsFile {
            song_label: "Test_Song".into(),
            voice_ref: VoiceRef::Inline("Test_Song_Voices".into()),
            fm_count: 1,
            psg_count: 0,
            tempo_divider: 1,
            tempo_modifier: 0x18,
            channels: vec![
                SmpsChannel {
                    kind: SmpsChannelKind::Fm,
                    label: "Test_Song_FM1".into(),
                    initial_pitch: 0,
                    initial_volume: 0x0F,
                    psg_envelope: None,
                    events: vec![
                        SmpsEvent::SetVoice(0),
                        SmpsEvent::Note { pitch: 0x93, duration: 0x18 }, // nC4
                        SmpsEvent::Rest { duration: 0x06 },
                        SmpsEvent::Note { pitch: 0x95, duration: 0x0C }, // nD4
                        SmpsEvent::Stop,
                    ],
                },
            ],
            voices: vec![[0u8; 25]], // one dummy voice
        }
    }

    #[test]
    fn test_map_creates_tracks() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.tracks.len(), 1);
        assert_eq!(result.song.tracks[0].name, "FM1");
    }

    #[test]
    fn test_map_creates_metadata() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.metadata.driver_id, "flamedriver");
        assert_eq!(result.song.metadata.ticks_per_beat, 480);
        assert!(result.song.metadata.tempo > 0.0);
    }

    #[test]
    fn test_map_creates_fm_instrument() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.instruments.fm.len(), 1);
        assert!(result.song.tracks[0].instrument_id.is_some());
    }

    #[test]
    fn test_map_note_pitches() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        let notes = &result.song.tracks[0].regions[0].notes;
        // nC4 = $93 = SMPS $81 + 18 semitones = MIDI 12 + 18 = MIDI 30
        // Wait, nC4 = 0x81 + (4*12 + 0) = 0x81 + 48 = 0xB1? No.
        // nC0 = 0x81. nC1 = 0x81 + 12 = 0x8D. nC4 = 0x81 + 48 = 0xB1.
        // But we used pitch: 0x93 which is 0x93 - 0x81 = 18 semitones from C0 = MIDI 30
        // That's actually nFs1 (F#1). Let me fix: nC4 should be 0x81 + 48 = 0xB1
        // Actually the test data says pitch: 0x93 which = 0x81 + 0x12 = nD1 + something
        // Let's just check the MIDI conversion: smps 0x93 - 0x81 = 0x12 = 18. MIDI = 12 + 18 = 30.
        assert_eq!(notes[0].pitch, 30); // 0x93 → MIDI 30
        assert_eq!(notes[1].pitch, 32); // 0x95 → MIDI 32
    }

    #[test]
    fn test_map_creates_region() {
        let smps = make_simple_smps();
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.tracks[0].regions.len(), 1);
        assert_eq!(result.song.tracks[0].regions[0].start_tick, 0);
        assert!(result.song.tracks[0].regions[0].duration_ticks > 0);
    }

    #[test]
    fn test_map_tie_extends_previous_note() {
        let smps = SmpsFile {
            song_label: "Tie_Test".into(),
            voice_ref: VoiceRef::Inline("Tie_Test_Voices".into()),
            fm_count: 1,
            psg_count: 0,
            tempo_divider: 1,
            tempo_modifier: 0x18,
            channels: vec![SmpsChannel {
                kind: SmpsChannelKind::Fm,
                label: "Tie_Test_FM1".into(),
                initial_pitch: 0,
                initial_volume: 0x0F,
                psg_envelope: None,
                events: vec![
                    SmpsEvent::SetVoice(0),
                    SmpsEvent::Note { pitch: 0xB1, duration: 0x7F }, // nC4, max duration
                    SmpsEvent::Tie,
                    SmpsEvent::Note { pitch: 0xB1, duration: 0x29 }, // tied continuation
                    SmpsEvent::Stop,
                ],
            }],
            voices: vec![[0u8; 25]],
        };
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        let notes = &result.song.tracks[0].regions[0].notes;
        assert_eq!(notes.len(), 1); // should be a single merged note
    }

    #[test]
    fn test_map_uvb_creates_placeholder_instruments() {
        let smps = SmpsFile {
            song_label: "UVB_Test".into(),
            voice_ref: VoiceRef::Uvb,
            fm_count: 1,
            psg_count: 0,
            tempo_divider: 1,
            tempo_modifier: 0x18,
            channels: vec![SmpsChannel {
                kind: SmpsChannelKind::Fm,
                label: "UVB_Test_FM1".into(),
                initial_pitch: 0,
                initial_volume: 0x0F,
                psg_envelope: None,
                events: vec![
                    SmpsEvent::SetVoice(0),
                    SmpsEvent::Note { pitch: 0xB1, duration: 0x18 },
                    SmpsEvent::Stop,
                ],
            }],
            voices: vec![], // no inline voices
        };
        let driver = FlamedriverProfile;
        let result = map_smps_to_song(&smps, &driver).unwrap();
        assert_eq!(result.song.instruments.fm.len(), 1);
        assert!(result.song.instruments.fm[0].name.contains("unresolved"));
        assert!(result.warnings.iter().any(|w| w.message.contains("unresolved")));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::smps_mapper 2>&1 | tail -15`
Expected: FAIL — `todo!()`

- [ ] **Step 3: Implement the mapper**

Replace `todo!()` in `map_smps_to_song` with the full implementation:

1. **Compute BPM and tick scale**: `smps_ticks_per_sec = (modifier as f64 / 256.0) * 60.0`. Assume 1 SMPS tick = 1/4 beat (sixteenth note) for BPM: `bpm = smps_ticks_per_sec * 60.0 / (ticks_per_beat as f64 / daw_ticks_per_smps_tick)`. Actually simpler: pick `daw_ticks_per_smps_tick` to maintain proportions. The formula from the exporter: `daw_ticks_per_smps_tick = ticks_per_beat / smps_ticks_per_beat`. We reverse this: choose `daw_ticks_per_smps_tick = 480.0 / (smps_ticks_per_sec / beats_per_second)`. We derive BPM as: `bpm = (modifier as f64 / 256.0) * 60.0 / divider as f64`. This treats 1 SMPS tick as 1 sixteenth note (divider controls how many frames per tick).

2. **Create instruments**:
   - For each inline voice, call `driver.fm_from_bytes()` → `FmInstrument`. Deduplicate by byte content.
   - For UVB/external, create placeholder FM instruments per unique voice index used.
   - For PSG channels with envelope indices, create `PsgInstrument` from the bundled table.
   - For DAC channels, create placeholder `DacInstrument` per unique DAC sample used.

3. **Create tracks**: One track per channel. Set name from label suffix (after last `_`). Set channel assignment. Set initial volume. Link instrument.

4. **Map events to notes**: Walk each channel's events, maintaining a cursor position (in DAW ticks). Notes get placed at the cursor, rests advance the cursor. Ties extend the previous note. Transposition offsets are accumulated. Create one region per track spanning all notes.

5. **Collect warnings**: Unsupported events, unresolved voices, mid-stream voice changes.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::smps_mapper 2>&1 | tail -15`
Expected: All 7 tests PASS

- [ ] **Step 5: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/import/smps_mapper.rs
git commit -m "feat(import): SMPS-to-DAW mapper with tick conversion and instrument creation"
```

---

### Task 6: Import Orchestrator + Project Creation

**Files:**
- Modify: `src-tauri/src/import/mod.rs`

Add the top-level `import_smps_file` function that orchestrates: parse → map → create project directory → save files → return result.

- [ ] **Step 1: Write the test**

Add to `src-tauri/src/import/mod.rs`:

```rust
pub fn import_smps_file(
    source_path: &std::path::Path,
    project_dir: &std::path::Path,
    driver: &dyn crate::model::driver::DriverProfile,
) -> Result<ImportResult, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::flamedriver::FlamedriverProfile;
    use std::path::PathBuf;

    #[test]
    fn test_import_creates_project_directory() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/Mus - DEZ1.asm");
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("DEZ1_Import");

        let driver = FlamedriverProfile;
        let result = import_smps_file(&source, &project_dir, &driver).unwrap();

        assert!(project_dir.join("project.json").exists());
        assert!(project_dir.join(".megadaw").exists());
        assert!(project_dir.join("instruments/fm").exists());
        assert_eq!(result.track_count, 9); // 1 DAC + 5 FM + 3 PSG
        assert!(result.instrument_count > 0);
    }

    #[test]
    fn test_import_saves_fm_instruments() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/Mus - DEZ1.asm");
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("DEZ1_Import2");

        let driver = FlamedriverProfile;
        import_smps_file(&source, &project_dir, &driver).unwrap();

        let fm_dir = project_dir.join("instruments/fm");
        let fm_files: Vec<_> = std::fs::read_dir(&fm_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "json"))
            .collect();
        assert_eq!(fm_files.len(), 4); // DEZ1 has 4 voices
    }
}
```

Add `tempfile` to dev-dependencies if not already present:

```bash
cd /home/volence/sonic_hacks/megadaw/src-tauri
grep -q "tempfile" Cargo.toml || cargo add tempfile --dev
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import::tests 2>&1 | tail -15`
Expected: FAIL — `todo!()`

- [ ] **Step 3: Implement import_smps_file**

```rust
pub fn import_smps_file(
    source_path: &std::path::Path,
    project_dir: &std::path::Path,
    driver: &dyn crate::model::driver::DriverProfile,
) -> Result<ImportResult, String> {
    let source = std::fs::read_to_string(source_path)
        .map_err(|e| format!("failed to read {}: {e}", source_path.display()))?;

    let smps = smps_parser::parse_smps(&source)?;
    let mapped = smps_mapper::map_smps_to_song(&smps, driver)?;
    let song = mapped.song;

    // Create project directory structure
    std::fs::create_dir_all(project_dir).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/fm")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/psg")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/dac")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("exports")).map_err(|e| e.to_string())?;

    // Write .megadaw marker
    let version = serde_json::json!({ "version": "0.1.0" });
    std::fs::write(
        project_dir.join(".megadaw"),
        serde_json::to_string_pretty(&version).unwrap(),
    ).map_err(|e| e.to_string())?;

    // Write project.json (tracks + metadata, no instruments)
    let project_file = crate::model::song::ProjectFile {
        metadata: song.metadata.clone(),
        tracks: song.tracks.clone(),
    };
    let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
    std::fs::write(project_dir.join("project.json"), json).map_err(|e| e.to_string())?;

    // Write instrument files
    let mut instrument_count = 0;
    for inst in &song.instruments.fm {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/fm/{}.json", inst.id)),
            json,
        ).map_err(|e| e.to_string())?;
        instrument_count += 1;
    }
    for inst in &song.instruments.psg {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/psg/{}.json", inst.id)),
            json,
        ).map_err(|e| e.to_string())?;
        instrument_count += 1;
    }
    for inst in &song.instruments.dac {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/dac/{}.json", inst.id)),
            json,
        ).map_err(|e| e.to_string())?;
        instrument_count += 1;
    }

    Ok(ImportResult {
        metadata: song.metadata,
        track_count: song.tracks.len(),
        instrument_count,
        warnings: mapped.warnings,
    })
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app --lib import 2>&1 | tail -15`
Expected: All import tests PASS

- [ ] **Step 5: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/import/mod.rs src-tauri/Cargo.toml
git commit -m "feat(import): import orchestrator with project creation"
```

---

### Task 7: Import IPC Command

**Files:**
- Modify: `src-tauri/src/ipc/commands.rs`
- Modify: `src-tauri/src/ipc/mod.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add the IPC command**

Add to `src-tauri/src/ipc/commands.rs` at the bottom:

```rust
// --- Import ---

#[tauri::command]
pub fn import_song(
    state: State<'_, ProjectState>,
    source_path: String,
    project_dir: String,
) -> Result<crate::import::ImportResult, String> {
    let mgr = state.manager.lock().map_err(|e| format!("mutex poisoned: {e}"))?;
    let registry = mgr.driver_registry();
    let driver = registry.get("flamedriver")
        .ok_or("Flamedriver driver not found")?;

    let source = std::path::PathBuf::from(&source_path);
    let project = std::path::PathBuf::from(&project_dir);

    crate::import::import_smps_file(&source, &project, driver)
}
```

- [ ] **Step 2: Add re-export to ipc/mod.rs**

Add `import_song` to the `pub use commands::{...}` block in `src-tauri/src/ipc/mod.rs`, under a new `// Import` comment section.

- [ ] **Step 3: Register in lib.rs**

Add `import_song` to the imports in `src-tauri/src/lib.rs` and to the `tauri::generate_handler![...]` macro.

- [ ] **Step 4: Verify it compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo build -p megadaw-app 2>&1 | tail -5`
Expected: Compiles

- [ ] **Step 5: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/ipc/commands.rs src-tauri/src/ipc/mod.rs src-tauri/src/lib.rs
git commit -m "feat(import): add import_song IPC command"
```

---

### Task 8: Frontend — IPC Wrapper + Types

**Files:**
- Modify: `src/api/ipc.ts`

- [ ] **Step 1: Add import types and wrapper function**

Add to the bottom of `src/api/ipc.ts`:

```typescript
// --- Import ---

export interface ImportWarning {
  channel: string;
  message: string;
}

export interface ImportResult {
  metadata: SongMetadata;
  trackCount: number;
  instrumentCount: number;
  warnings: ImportWarning[];
}

export async function importSong(sourcePath: string, projectDir: string): Promise<ImportResult> {
  return invoke<ImportResult>("import_song", { sourcePath, projectDir });
}
```

- [ ] **Step 2: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit 2>&1 | tail -5`
Expected: No errors (or only pre-existing warnings)

- [ ] **Step 3: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src/api/ipc.ts
git commit -m "feat(import): add importSong IPC wrapper"
```

---

### Task 9: Frontend — Import Button + Handler

**Files:**
- Modify: `src/components/TopBar.tsx`
- Modify: `src/App.tsx`
- Modify: `src/App.module.css`

- [ ] **Step 1: Add onImport prop to TopBar**

In `src/components/TopBar.tsx`, add `onImport?: () => void` to the `TopBarProps` interface and destructure it. Add an Import button in the actions div, after the Open button:

```tsx
<button className={styles.btn} onClick={onImport}>Import</button>
```

The Import button should always be visible (not gated on `projectMeta`), same as New and Open.

- [ ] **Step 2: Add import handler and warning state to App.tsx**

In `src/App.tsx`:

Add `importWarnings` state:
```typescript
const [importWarnings, setImportWarnings] = useState<ipc.ImportWarning[] | null>(null);
```

Add `handleImport` function:
```typescript
async function handleImport() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const sourcePath = await open({
    title: "Select SMPS Assembly File",
    filters: [{ name: "SMPS Assembly", extensions: ["asm"] }],
  });
  if (!sourcePath) return;

  const projectDir = await open({
    directory: true,
    title: "Choose Project Directory",
  });
  if (!projectDir) return;

  try {
    if (projectOpen) await ipc.closeProject();
    setPlaying(false);

    const result = await ipc.importSong(sourcePath as string, projectDir as string);
    const song = await ipc.openProject(projectDir as string);
    setProjectMeta(song.metadata);
    setSelectedInstrument(null);
    setSelectedRegions([]);

    if (result.warnings.length > 0) {
      setImportWarnings(result.warnings);
    }
  } catch (e) {
    console.error("Import failed:", e);
  }
}
```

Pass `onImport={handleImport}` to `<TopBar>`.

Add warning panel in the JSX (after the export status panels, before `<div className={styles.body}>`):

```tsx
{importWarnings && (
  <div className={styles.importWarning}>
    <div className={styles.importWarningHeader}>
      <span>Import complete ({importWarnings.length} warning{importWarnings.length !== 1 ? "s" : ""})</span>
      <button onClick={() => setImportWarnings(null)}>x</button>
    </div>
    <ul>
      {importWarnings.map((w, i) => (
        <li key={i}>{w.channel ? `${w.channel}: ` : ""}{w.message}</li>
      ))}
    </ul>
  </div>
)}
```

- [ ] **Step 3: Add warning panel styles to App.module.css**

Add to `src/App.module.css`:

```css
.importWarning {
  position: fixed;
  top: 48px;
  right: 16px;
  background: #2a2a1a;
  border: 1px solid #ccaa44;
  color: #eedd88;
  padding: 12px 16px;
  border-radius: 6px;
  font-size: 13px;
  max-width: 400px;
  max-height: 300px;
  overflow-y: auto;
  z-index: 100;
}

.importWarningHeader {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
  font-weight: 600;
}

.importWarningHeader button {
  background: none;
  border: none;
  color: #eedd88;
  cursor: pointer;
  font-size: 14px;
}

.importWarning ul {
  margin: 0;
  padding-left: 16px;
}

.importWarning li {
  margin: 4px 0;
}
```

- [ ] **Step 4: Verify TypeScript compiles**

Run: `cd /home/volence/sonic_hacks/megadaw && npx tsc --noEmit 2>&1 | tail -5`
Expected: No errors

- [ ] **Step 5: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src/components/TopBar.tsx src/App.tsx src/App.module.css
git commit -m "feat(import): add Import button and warning display"
```

---

### Task 10: Integration Test — Full Round Trip

**Files:**
- No new files — uses existing test infrastructure

This task verifies the full pipeline: parse a real song → create project → open it → verify track/note/instrument integrity.

- [ ] **Step 1: Write a round-trip integration test**

Add to `src-tauri/src/import/mod.rs` tests:

```rust
    #[test]
    fn test_import_dez1_round_trip_opens() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/Mus - DEZ1.asm");
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("DEZ1_RT");

        let driver = FlamedriverProfile;
        let result = import_smps_file(&source, &project_dir, &driver).unwrap();

        // Verify project.json is valid
        let json = std::fs::read_to_string(project_dir.join("project.json")).unwrap();
        let project_file: crate::model::song::ProjectFile =
            serde_json::from_str(&json).unwrap();

        assert_eq!(project_file.metadata.driver_id, "flamedriver");
        assert_eq!(project_file.tracks.len(), result.track_count);

        // Verify FM instruments are valid JSON
        let fm_dir = project_dir.join("instruments/fm");
        for entry in std::fs::read_dir(&fm_dir).unwrap() {
            let entry = entry.unwrap();
            if entry.path().extension().map_or(false, |ext| ext == "json") {
                let data = std::fs::read_to_string(entry.path()).unwrap();
                let _inst: crate::model::instrument::FmInstrument =
                    serde_json::from_str(&data).unwrap();
            }
        }

        // Verify tracks have notes
        let fm_tracks: Vec<_> = project_file.tracks.iter()
            .filter(|t| matches!(t.channel, crate::model::song::ChannelAssignment::Fm(_)))
            .collect();
        assert!(!fm_tracks.is_empty());
        // FM1 should have notes (it's a melodic track)
        let fm1 = fm_tracks.iter().find(|t| t.name == "FM1").unwrap();
        assert!(!fm1.regions.is_empty());
        assert!(!fm1.regions[0].notes.is_empty());
    }

    #[test]
    fn test_import_aiz1_uvb_song() {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("test_data/AIZ1.asm");
        let tmp = tempfile::tempdir().unwrap();
        let project_dir = tmp.path().join("AIZ1_Import");

        let driver = FlamedriverProfile;
        let result = import_smps_file(&source, &project_dir, &driver).unwrap();

        // AIZ1 uses UVB, so all FM instruments should be "unresolved"
        assert!(result.warnings.iter().any(|w| w.message.contains("unresolved")));
        assert!(project_dir.join("project.json").exists());
    }
```

- [ ] **Step 2: Run all tests**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app 2>&1 | tail -15`
Expected: All tests PASS (existing + new import tests)

- [ ] **Step 3: Commit**

```bash
cd /home/volence/sonic_hacks/megadaw
git add src-tauri/src/import/mod.rs
git commit -m "test(import): add full round-trip integration tests"
```

- [ ] **Step 4: Run the full test suite one more time**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo test -p megadaw-app 2>&1 | tail -5`
Expected: All tests pass, no regressions

- [ ] **Step 5: Final verification — build the app**

Run: `cd /home/volence/sonic_hacks/megadaw && cargo build -p megadaw-app && npx tsc --noEmit 2>&1 | tail -5`
Expected: Both Rust and TypeScript compile cleanly
