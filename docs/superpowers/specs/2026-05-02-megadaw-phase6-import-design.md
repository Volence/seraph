# MegaDAW Phase 6: SMPS Import — Design Spec

## Goal

Import Flamedriver/SMPS assembly (`.asm`) song files into MegaDAW, producing a fully editable project with tracks, notes, FM voices, and PSG instruments. Enables the round-trip workflow: import existing S3K music → edit in DAW → re-export to game.

## Decisions

| Question | Decision | Rationale |
|----------|----------|-----------|
| Loop handling | Flatten all loops/calls into linear note streams | Faster to ship, guaranteed correct. Revisit pattern detection for smarter region mapping later. |
| PSG envelopes | Bundle Flamedriver envelope table as Rust const | No runtime dependency on driver source files |
| Voice resolution | Parse inline voices from song file; placeholder for external/UVB | Most songs have inline voices. Separate voice bank importer deferred. |
| Parser format | Assembly text (`.asm`) only | Binary `.bin` parser deferred to a later phase |
| Parser strategy | Line-by-line macro regex matching (Approach A) | SMPS macros are rigid and predictable. No need for a full assembler. |
| Import target | skdisasm + S.C.E. `.asm` files | 60+ songs available, all Flamedriver macro format |
| Project creation | Import creates a new project directory | Song IS the project; import → auto-open for editing |

## Architecture

Four new Rust files in `src-tauri/src/import/`, plus IPC and frontend wiring. The parser produces an intermediate `SmpsFile` struct, the mapper converts it to the DAW's `Song` + `InstrumentBank` model, and the IPC command orchestrates parse → map → create project → save.

## Components

### 1. SMPS Assembly Parser (`import/smps_parser.rs`)

Reads an `.asm` file line by line and produces an `SmpsFile` struct — a direct representation of the SMPS data before any DAW mapping.

**Line matching**: Each line is matched against known macro patterns via regex:

- **Header macros**: `smpsHeaderStartSong`, `smpsHeaderVoice`, `smpsHeaderChan`, `smpsHeaderTempo`, `smpsHeaderFM`, `smpsHeaderPSG`, `smpsHeaderDAC`
- **Channel data labels**: `Snd_XXX_FM1:` etc. (label followed by colon)
- **dc.b lines**: Note constants (`nC4`, `nRst`), coordination flags (`smpsFMvoice`, `smpsLoop`, `smpsStop`, etc.), raw hex (`$0F`)
- **Voice macros**: `smpsVcAlgorithm`, `smpsVcFeedback`, `smpsVcDetune`, `smpsVcTotalLevel`, etc. (OR `smpsVc` single-line 25-byte format)
- **Comments/whitespace/includes**: Ignored

**Output structs**:

```rust
struct SmpsFile {
    song_label: String,
    voice_pointer: VoiceRef,       // Inline, UVB, or label
    fm_count: u8,
    psg_count: u8,
    tempo_divider: u8,
    tempo_modifier: u8,
    channels: Vec<SmpsChannel>,    // DAC first, then FM, then PSG
    voices: Vec<[u8; 25]>,         // Parsed inline voices (if present)
}

enum VoiceRef {
    Inline(String),   // Label pointing to voices in same file
    Uvb,              // smpsHeaderVoiceUVB
    External(String), // Label not found in file
}

enum SmpsChannelKind { Dac, Fm, Psg }

struct SmpsChannel {
    kind: SmpsChannelKind,
    label: String,             // Original label name
    initial_pitch: i8,
    initial_volume: u8,
    psg_envelope: Option<u8>,  // PSG tone envelope index
    events: Vec<SmpsEvent>,    // Flat stream after loop/call flattening
}

enum SmpsEvent {
    Note { pitch: u8, duration: u8 },
    Rest { duration: u8 },
    SetVoice(u8),
    SetPan(u8),
    Transpose(i8),
    Tie,                        // smpsNoAttack — extend previous note
    Stop,
    Unsupported { flag: u8, name: String },
}
```

**Loop/call flattening**:
- `smpsLoop` ($F7): Record loop body, repeat inline `count` times
- `smpsCall` ($F8) / `smpsReturn` ($F9): Inline the subroutine body
- `smpsJump` ($F6): Single repetition of the target section, then end
- `smpsContinuousLoop` ($FC): Same as `smpsJump`

**Label resolution**: The parser collects all labels and their positions in a first pass, then resolves jump/call/loop targets in a second pass. Labels that point to channel data vs. voice data are distinguished by whether they appear in a header pointer.

### 2. SMPS-to-DAW Mapper (`import/smps_mapper.rs`)

Takes the parser's `SmpsFile` and produces a `Song` + `InstrumentBank`.

**Tick conversion**: The SMPS `(divider, modifier)` pair defines timing. SMPS ticks per second = `(modifier / 256) * 60`. Convert to BPM: `bpm = smps_ticks_per_second / smps_ticks_per_beat * 60`. DAW uses 480 ticks/beat. Each SMPS note duration maps to: `smps_duration * daw_ticks_per_smps_tick`, rounded to nearest integer.

BPM derivation: `bpm = (modifier / 256) * 60 / divider`. This gives the effective frames-per-tick rate converted to beats-per-minute assuming 1 SMPS tick = 1 sixteenth note (the most common mapping in S3K music). The exact BPM is cosmetic — all note durations are converted individually from SMPS ticks to DAW ticks, so playback timing is preserved regardless of the BPM label.

**Track creation**: One `Track` per SMPS channel:
- DAC channels → `ChannelAssignment::Dac(0)`
- FM channels → `ChannelAssignment::Fm(0..5)`
- PSG channels → `ChannelAssignment::Psg(0..2)`
- Track name derived from channel label (e.g., `Snd_DEZ1_FM1` → "FM1")

**Region strategy**: Each track gets one region spanning the full duration. Simple, correct, ideal for the edit-and-re-export workflow.

**Note mapping**:
- `nC0` ($81) = MIDI 12, incrementing chromatically. Reverse of `midi_to_smps_note` from the exporter.
- Rest events → gaps (advance tick position, no Note created)
- `smpsNoAttack` (tie) → extend previous note's `duration_ticks`
- Velocity: Default 127 (SMPS has no per-note velocity)
- Channel `initial_volume` → track `volume` field
- Transposition flag → applied as pitch offset to subsequent notes

**Instrument creation**:
- **Inline FM voices**: 25-byte voice → `fm_from_bytes()` → `FmInstrument`. Named "SongLabel Voice N". Deduplicated by byte equality.
- **External/UVB voices**: Placeholder `FmInstrument` with default operators, named "Voice N (unresolved)".
- **PSG envelopes**: Index into bundled Flamedriver table → `PsgInstrument` with volume sequence and loop point.
- **DAC samples**: Placeholder `DacInstrument` named by sample ID (e.g., "DAC $81"). User replaces with real WAV imports later.

**Track-instrument linking**: Each track's `instrument_id` points to the first voice used on that channel. Mid-stream `smpsFMvoice` changes are logged as import warnings.

**Import result**:
```rust
struct ImportResult {
    metadata: SongMetadata,
    track_count: usize,
    instrument_count: usize,
    warnings: Vec<ImportWarning>,
}

struct ImportWarning {
    channel: String,
    message: String,
}
```

### 3. Coordination Flag Handling (in `smps_parser.rs`)

Flags are dispatched via a lookup table that maps each byte to its argument count and handler.

**Processed** (affect note/timing data):
- `smpsFMvoice` ($EF) → `SmpsEvent::SetVoice`
- `smpsPSGvoice` ($F5) → updates channel PSG envelope
- `smpsPan` ($E0) → `SmpsEvent::SetPan`
- `smpsChangeTransposition` ($FB) → `SmpsEvent::Transpose`
- `smpsNoAttack` ($E7) → `SmpsEvent::Tie`
- `smpsStop` ($F2) → `SmpsEvent::Stop`
- `smpsLoop` ($F7), `smpsCall` ($F8), `smpsReturn` ($F9), `smpsJump` ($F6), `smpsContinuousLoop` ($FC) → flattened during parsing

**Logged as warnings** (musical meaning but no DAW model equivalent yet):
- `smpsModSet` ($F0), `smpsModOn`/`smpsModOff` — modulation
- `smpsDetune` ($E1) — fine detune
- `smpsAlterVol` ($E6), `smpsPSGAlterVol` ($EC) — volume changes
- `smpsNoteFill` ($E8) — gate time
- `smpsSetTempoMod` ($FF $00), `smpsSetTempoDiv` ($FF $04) — mid-song tempo
- `smpsSSGEG` ($FF $05), `smpsChanTempoDiv` ($FF $08), `smpsPitchSlide` ($FF $0B), `smpsSetLFO` ($FF $0C)

**Skipped silently** (driver-internal, no musical meaning):
- `smpsNop`/$E2, `smpsStopFM`/$E3, `smpsSpindashRev`/$E9, `smpsPlayDACSample`/$EA, `smpsConditionalJump`/$EB, `smpsFMICommand`/$EE, `smpsPSGform`/$F3, `smpsAlternateSMPS`/$FD, `smpsFM3SpecialMode`/$FE
- `$FF` subcodes: `smpsPlaySound`/$01, `smpsHaltMusic`/$02, `smpsResetSpindashRev`/$07, `smpsFMVolEnv`/$06, `smpsChanFMCommand`/$09, `smpsPlayMusic`/$0D

Every flag's argument byte count is known, so skipped flags are consumed correctly without misaligning the stream.

### 4. PSG Envelope Table (`import/psg_envelopes.rs`)

Const array of Flamedriver PSG envelopes extracted from S.C.E.'s `Flamedriver.asm` at development time.

```rust
struct PsgEnvelopeEntry {
    volumes: &'static [u8],
    loop_point: Option<usize>,
}

const FLAMEDRIVER_PSG_ENVELOPES: &[PsgEnvelopeEntry] = &[ /* ... */ ];
```

Each entry becomes a `PsgInstrument` with `volume_sequence` and `loop_point` when referenced by a channel header.

### 5. Import IPC + Frontend

**IPC command** (`import_song` in `commands.rs`):
1. Parse the `.asm` file → `SmpsFile`
2. Map to `Song` + `InstrumentBank`
3. Create project directory at user-specified path
4. Save `project.json` + instrument files
5. Return `ImportResult`

**Frontend flow**:
1. User clicks "Import SMPS" button in TopBar (visible always, next to New/Open)
2. File picker: select `.asm` file
3. Directory picker: choose location for new project
4. Backend: parse → map → create project → save
5. Frontend: auto-opens the new project via `open_project`
6. Import warnings shown in a dismissable yellow info panel

**Files modified**:
- `src-tauri/src/ipc/commands.rs` — `import_song` command
- `src-tauri/src/ipc/mod.rs` — re-export
- `src-tauri/src/lib.rs` — `mod import`, register command
- `src/api/ipc.ts` — `importSong()` wrapper + `ImportResult`/`ImportWarning` types
- `src/components/TopBar.tsx` — Import button prop + UI
- `src/App.tsx` — `handleImport()`, warning state, warning panel rendering
- `src/App.module.css` — `.importWarning` panel styles (yellow/info theme)

## Deferred Work

- **Binary `.bin` parser**: Same mapper, different parser front-end. Separate file, separate phase.
- **Loop pattern detection**: Detect repeated sections and create separate linked regions instead of flattening. Would produce more "musical" arrangement view.
- **Mid-stream voice changes**: Currently logged as warnings. Could split into multiple tracks or create voice-change automation.
- **DAC sample import**: Requires actual PCM files. Current import creates placeholders.
- **UVB / external voice bank importer**: Separate tool to import voices from `UniBank.asm` or other voice files and resolve placeholder instruments.
- **Modulation, detune, volume automation**: Logged as warnings now. Would require DAW model extensions for automation lanes.

## Test Strategy

- **Parser unit tests**: Parse known S.C.E. song snippets, verify `SmpsFile` field values
- **Mapper unit tests**: Feed constructed `SmpsFile` structs, verify `Song`/`InstrumentBank` output
- **Round-trip test**: Import a song → export it → compare output `.asm` against input (should be structurally equivalent modulo loop flattening and formatting)
- **Integration test**: Full IPC `import_song` → `open_project` → verify track/note/instrument counts
