# MegaDAW Phase 5: Flamedriver Export — Design Spec

## Overview

Phase 5 adds the ability to export MegaDAW compositions to Flamedriver's SMPS assembly format. The exported `.asm` files are assembled directly into the ROM by the AS assembler — no intermediate binary conversion step. The Z80 Flamedriver sound driver plays them back at runtime.

**Builds on:** Phase 1 (audio engine), Phase 2 (project model + IPC), Phase 3 (instrument editors), Phase 4 (sequencer + playback)

**Core guarantee:** What you hear in the DAW is what plays in-game. Export is a strict, lossless conversion — it succeeds completely or fails with actionable errors.

---

## 1. Architecture

### 1.1 Driver-Owned Export

Export is a method on the `DriverProfile` trait:

```rust
fn export_song(
    &self,
    song: &Song,
    instruments: &InstrumentBank,
    output_dir: &Path,
) -> Result<ExportResult, Vec<ExportError>>;
```

Each driver knows its own format, tick resolution, and constraints. Flamedriver's implementation converts the song model to SMPS assembly. A future driver (GEMS, custom) implements the same trait method with its own logic.

### 1.2 Export Pipeline

1. **Validate** — check all tracks, notes, instruments against SMPS constraints
2. **Map tempo** — compute best-fit SMPS tempo_divider + tempo_modifier for the song's BPM
3. **Convert tracks** — flatten regions into linear event streams, encode notes/rests/coordination flags
4. **Build voice bank** — deduplicate FM instruments, assign voice indices
5. **Write files** — emit assembly files + copy DAC PCM samples

Validation runs fully before any conversion begins. If validation fails, no files are written.

---

## 2. Tick Resolution Mapping

### 2.1 SMPS Timing Model

SMPS uses a frame-based timing system:
- Each VBlank (60Hz NTSC): `accumulator += tempo_modifier`
- On 8-bit overflow: advance all note durations by 1 tick
- Effective tick rate: `(tempo_modifier / 256) * 60` Hz
- Note durations in the data stream are pre-multiplied by `tempo_divider`

### 2.2 BPM to SMPS Conversion

Given the song's BPM and MegaDAW's 480 ticks/beat:

```
smps_ticks_per_second = (tempo_modifier / 256) * 60
smps_ticks_per_beat = smps_ticks_per_second / (BPM / 60)
daw_ticks_per_smps_tick = 480 / smps_ticks_per_beat
```

The exporter searches for the (tempo_divider, tempo_modifier) pair that minimizes quantization error for common note durations (quarter, eighth, sixteenth, triplet). This is a build-time computation — the exporter evaluates candidates and picks the best fit.

### 2.3 Duration Conversion

Each DAW note duration is converted:

```
smps_duration = round(daw_duration / daw_ticks_per_smps_tick)
```

If `smps_duration` rounds to 0, that's a validation error (note too short for the tempo resolution). If `smps_duration` exceeds 127 ($7F), the note is split using the no-attack flag (`$E7`) to tie multiple segments together.

### 2.4 Driver Profile Ownership

The tick mapping logic lives entirely within `FlamedriverProfile::export_song()`. A different driver implements its own resolution mapping. The `DriverProfile` trait does not prescribe a tick model.

---

## 3. Note & Event Encoding

### 3.1 Track Flattening

Each track's regions are merged into a single linear event stream sorted by absolute tick position. Gaps between regions become rests. Overlapping regions on the same track are a validation error.

### 3.2 Note Encoding

Each note becomes:
- **Pitch byte**: MIDI pitch mapped to SMPS note constant (`nC0`=$81 through `nBb7`=$CB). MIDI note 12 = nC0, MIDI note 95 = nBb7.
- **Duration byte**: 1–127 SMPS ticks. Omitted if it matches the previous note's duration (SMPS implicit duration optimization).

### 3.3 Long Notes (Ties)

Notes with SMPS duration > 127 are split:
```asm
dc.b nC4, $7F          ; First 127 ticks
smpsNoAttack
dc.b nC4, $XX          ; Remaining ticks (repeat if still > 127)
```

The `$E7` (smpsNoAttack) flag tells the driver to continue the current note without re-triggering the envelope.

### 3.4 Rests

Gaps between notes emit `nRst` ($80) + duration. Long rests are split the same way as long notes (without no-attack, just repeated rests).

### 3.5 Duration Optimization

SMPS allows omitting the duration byte when it matches the previous duration. The exporter tracks the "current duration" state and only emits a duration byte when it changes. This produces smaller, more idiomatic SMPS output.

### 3.6 Coordination Flags

The exporter emits these coordination flags per channel:
- `smpsSetvoice $XX` — set FM voice index (at channel start, and whenever the instrument changes within regions)
- `smpsPan panXX, $00` — panning from the track's pan setting (panLeft=$80, panRight=$40, panCenter=$C0)
- `smpsStop` ($F2) — end of channel data

Phase 5 does not emit: modulation, loops, subroutines, detune, note fill, or volume changes mid-track. These are future additions.

### 3.7 DAC Channels

DAC tracks encode each note as a DAC sample reference. The pitch byte maps to the DAC sample ID. Rest handling is the same as FM/PSG channels.

---

## 4. Voice Bank

### 4.1 FM Voice Bank

One voice bank per exported song. Each voice is 25 bytes in the format `fm_to_bytes()` already produces, emitted using `smpsVc` macros for readability.

### 4.2 Voice Indexing

The exporter builds a voice index by scanning all non-muted FM tracks:
1. Collect unique instrument IDs referenced by FM tracks
2. Assign indices 0, 1, 2... in order of first appearance
3. If two tracks share the same FM instrument, they share the same voice index

### 4.3 PSG Instruments

PSG volume envelopes are emitted as PSG voice data tables alongside the voice bank. Each PSG instrument becomes a byte sequence (volume levels 0–15) with a loop point marker. Referenced by `$F5 xx` (smpsSetPSGVoice) in the channel data.

---

## 5. Exported File Structure

### 5.1 Output Directory

```
export/
├── Mus - SongName.asm        # Song header + all channel data streams
├── Voices - SongName.asm     # FM voice bank (smpsVc macros)
└── dac/                      # Only if DAC tracks exist
    ├── sample_name.pcm
    └── ...
```

The export directory is self-contained — it can be copied into any project without path dependencies.

### 5.2 Music File Layout

```asm
; ============================================================
; Song: SongName
; Exported from MegaDAW
; ============================================================

Snd_SongName_Header:
    smpsHeaderStartSong 3
    smpsHeaderVoice     Snd_SongName_Voices
    smpsHeaderChan      $XX, $XX
    smpsHeaderTempo     $XX, $XX

    smpsHeaderDAC       Snd_SongName_DAC, $00, $00
    smpsHeaderFM        Snd_SongName_FM1, $00, $XX
    smpsHeaderFM        Snd_SongName_FM2, $00, $XX
    ; ... (one per active FM track)
    smpsHeaderPSG       Snd_SongName_PSG1, $XX, $XX, $00, $00
    ; ... (one per active PSG track)

; ------------------------------------------------------------
; DAC Channel
; ------------------------------------------------------------
Snd_SongName_DAC:
    dc.b nRst, $30
    smpsStop

; ------------------------------------------------------------
; FM Channel 1
; ------------------------------------------------------------
Snd_SongName_FM1:
    smpsSetvoice    $00
    smpsPan         panCenter, $00
    dc.b nC4, $0F, nE4, nG4, nC5, $1E
    ; ...
    smpsStop

; ... (remaining channels)
```

### 5.3 Voice Bank File Layout

```asm
; ============================================================
; Voice Bank: SongName
; Exported from MegaDAW
; ============================================================

Snd_SongName_Voices:

; Voice 0 - "Bright Piano"
    smpsVcAlgorithm     $04
    smpsVcFeedback      $07
    smpsVcUnusedBits    $00
    smpsVcDetune        $00, $03, $07, $03
    smpsVcCoarseFreq    $01, $01, $01, $01
    smpsVcRateScale     $02, $02, $02, $01
    smpsVcAttackRate    $1F, $1F, $1F, $14
    smpsVcAmpMod        $00, $00, $00, $00
    smpsVcDecayRate1    $05, $05, $05, $07
    smpsVcDecayRate2    $02, $02, $02, $02
    smpsVcDecayLevel    $01, $01, $01, $01
    smpsVcReleaseRate   $01, $01, $01, $06
    smpsVcTotalLevel    $27, $27, $27, $1A

; Voice 1 - "Bass"
    ; ...
```

---

## 6. Validation

### 6.1 Strict Export Policy

Export succeeds completely or fails with a list of errors. No partial output, no best-effort approximation. If it exports, it will play correctly.

### 6.2 Validation Checks

| Check | Error condition |
|-------|----------------|
| Missing instrument | Non-muted track has no instrument assigned |
| Pitch range | Note pitch outside MIDI 12–95 (C0–Bb7) |
| Channel overlap | Two tracks on the same hardware channel with overlapping note times |
| Zero duration | Note duration rounds to 0 SMPS ticks after conversion |
| FM parameters | Any FM operator parameter out of range (via `validate_fm`) |
| PSG envelope | Missing volume sequence or invalid loop point (via `validate_psg`) |
| DAC data | Missing PCM data or invalid sample rate (via `validate_dac`) |
| Empty track | Non-muted track has no notes in any region |
| Overlapping regions | Two regions on the same track overlap in time |

### 6.3 Error Format

Each error identifies the exact location:

```rust
pub struct ExportError {
    pub track_name: String,
    pub region_index: Option<usize>,
    pub note_index: Option<usize>,
    pub message: String,
}
```

### 6.4 DAW-Side Prevention

Most export errors should be impossible during normal composition:
- Piano roll pitch ranges are already channel-aware (FM: 24–95, PSG: 33–95)
- Channel overlap validation already exists via `get_channel_overlaps`
- Instruments are validated on save

Export validation is a safety net, not the primary line of defense. If an error appears at export time, it indicates a gap in the DAW's real-time validation that should be fixed.

---

## 7. IPC & Frontend

### 7.1 Backend

One new IPC command:

```rust
#[tauri::command]
fn export_song(
    project_state: State<'_, ProjectState>,
    output_dir: String,
) -> Result<ExportResult, ExportFailure>;
```

Where:

```rust
pub struct ExportResult {
    pub files: Vec<String>,   // Paths of written files
}

pub struct ExportFailure {
    pub errors: Vec<ExportError>,
}
```

The command reads the current project state, resolves the driver profile, and calls `driver.export_song()`.

### 7.2 Frontend

- **Export button** in the TopBar, next to Save
- Clicking opens a **directory picker** dialog (via `@tauri-apps/plugin-dialog`)
- On **success**: brief toast/banner showing "Exported N files to /path/"
- On **failure**: error list panel showing each error with track/region/note location

No export settings dialog in Phase 5. The driver determines all parameters.

---

## 8. Scope

### 8.1 In Phase 5

- `DriverProfile::export_song()` trait method
- `FlamedriverProfile` implementation: tick mapping, note encoding, voice bank, file output
- Pre-export validation with actionable error messages
- Export IPC command
- Export button + directory picker + success/error UI
- Self-contained export directory (music asm + voice bank asm + DAC samples)

### 8.2 Deferred

- Full sound system export (sound ID table, master includes, DAC sample table)
- Loop point export (smpsLoop / smpsJump for song looping)
- Modulation export (smpsModSet from track/note metadata)
- Volume/velocity mapping to SMPS volume changes mid-track
- Import from SMPS (reverse direction — load existing songs into the DAW)
- Multi-song batch export
- Export preview (diff what changed since last export)
