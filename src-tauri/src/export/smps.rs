use std::collections::HashMap;
use uuid::Uuid;
use crate::export::ExportError;
use crate::model::driver::DriverProfile;
use crate::model::instrument::{FmInstrument, InstrumentBank};
use crate::model::song::{ChannelAssignment, NoteModulation, Pan, Track};

/// SMPS tempo parameters chosen to best represent the song's BPM.
#[derive(Debug, Clone, Copy)]
pub struct SmpsTempoParams {
    pub divider: u8,
    pub modifier: u8,
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
            let smps_ticks_per_sec = (60.0 / divider as f64) * (256.0 - modifier as f64) / 256.0;
            let smps_ticks_per_beat = smps_ticks_per_sec / beats_per_second;
            let daw_per_smps = tpb / smps_ticks_per_beat;

            let test_durations = [
                tpb,
                tpb / 2.0,
                tpb / 4.0,
                tpb / 3.0,
                tpb * 2.0,
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

    let smps_ticks_per_sec = (60.0 / best_divider as f64) * (256.0 - best_modifier as f64) / 256.0;
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

// --- Note Encoding ---

const SMPS_MAX_DURATION: u64 = 127;

/// MIDI pitch → SMPS note byte for FM. Valid MIDI range: 12 (C0) through 106 (Bb7).
/// Max note byte is $DF (z80_index 94); $E0+ are coordination flags.
pub fn midi_to_smps_note(midi_pitch: u8) -> Option<u8> {
    if midi_pitch < 12 || midi_pitch > 106 {
        return None;
    }
    Some(0x81 + (midi_pitch - 12))
}

/// SMPS note name for FM assembly output.
pub fn smps_note_name(midi_pitch: u8) -> String {
    let semitone = (midi_pitch - 12) % 12;
    let octave = (midi_pitch - 12) / 12;
    let name = match semitone {
        0 => "nC", 1 => "nCs", 2 => "nD", 3 => "nEb",
        4 => "nE", 5 => "nF", 6 => "nFs", 7 => "nG",
        8 => "nAb", 9 => "nA", 10 => "nBb", 11 => "nB",
        _ => unreachable!(),
    };
    format!("{name}{octave}")
}

/// SMPS note name for PSG. PSG import uses midi = z80_index + 36, so we
/// reverse with z80_index = midi - 36, then octave = z80_index / 12.
pub fn smps_note_name_psg(midi_pitch: u8) -> String {
    let z80_index = (midi_pitch as i16 - 36).max(0) as u8;
    let semitone = z80_index % 12;
    let octave = z80_index / 12;
    let name = match semitone {
        0 => "nC", 1 => "nCs", 2 => "nD", 3 => "nEb",
        4 => "nE", 5 => "nF", 6 => "nFs", 7 => "nG",
        8 => "nAb", 9 => "nA", 10 => "nBb", 11 => "nB",
        _ => unreachable!(),
    };
    format!("{name}{octave}")
}

/// DAW pitch → SMPS DAC sample name for assembly output.
/// DAW pitch 36 maps to SMPS byte 0x81, pitch 37→0x82, etc.
pub fn dac_sample_name(daw_pitch: u8) -> String {
    let smps_byte = 0x81u8.wrapping_add(daw_pitch.saturating_sub(36));
    use crate::import::smps_parser::build_dac_table;
    let table = build_dac_table();
    for (name, byte) in &table {
        if *byte == smps_byte {
            return name.clone();
        }
    }
    format!("${:02X}", smps_byte)
}

/// A single event in the SMPS output stream.
#[derive(Debug, Clone)]
pub enum SmpsEvent {
    Note { pitch_name: String, duration: u64 },
    Rest { duration: u64 },
    Tie,
    SetVoice(u8),
    SetPan(u8),
    SetPsgVoice(u8),
    VolumeChange(i8),
    SetModulation { wait: u8, speed: u8, delta: u8, steps: u8 },
    AlterNote(i8),
    PsgForm(u8),
    Stop,
}

/// Encode notes into SmpsEvents. Notes must be sorted by tick.
/// Gaps between notes become rests. Long durations split with ties.
// Kept: the no-voice-map entry point, referenced only from `#[cfg(test)]`
// tests in this module; production goes through `_with_voices` directly.
#[allow(dead_code)]
pub fn encode_channel_events(
    notes: &[(u64, u8, u64)],
    region_duration: u64,
    params: &SmpsTempoParams,
    channel: &ChannelAssignment,
) -> Result<Vec<SmpsEvent>, ExportError> {
    let notes_with_inst: Vec<(u64, u8, u64, u8, Option<Uuid>, i8, Option<u8>, Option<NoteModulation>)> =
        notes.iter().map(|&(t, p, d)| (t, p, d, 127, None, 0, None, None)).collect();
    encode_channel_events_with_voices(&notes_with_inst, region_duration, params, channel, &HashMap::new(), &HashMap::new())
}

fn encode_channel_events_with_voices(
    notes: &[(u64, u8, u64, u8, Option<Uuid>, i8, Option<u8>, Option<NoteModulation>)],
    region_duration: u64,
    params: &SmpsTempoParams,
    channel: &ChannelAssignment,
    voice_map: &HashMap<Uuid, u8>,
    psg_env_map: &HashMap<Uuid, u8>,
) -> Result<Vec<SmpsEvent>, ExportError> {
    let mut out = Vec::new();
    let mut cursor: u64 = 0;
    let mut current_voice: Option<u8> = None;
    let mut current_psg_env: Option<u8> = None;
    let mut current_vol_delta: i16 = 0;
    let mut current_detune: i8 = 0;
    let mut current_pan: Option<u8> = None;
    let mut current_mod: Option<(u8, u8, u8, u8)> = None;

    for (tick, pitch, dur_ticks, velocity, inst_id, detune, pan_override, note_mod) in notes {
        let (tick, pitch, dur_ticks, velocity, detune) = (*tick, *pitch, *dur_ticks, *velocity, *detune);
        if tick > cursor {
            let gap = tick - cursor;
            if let Some(smps_gap) = daw_to_smps_duration(gap, params) {
                emit_duration_events(&mut out, None, smps_gap);
            }
        }

        if matches!(channel, ChannelAssignment::Fm(_)) {
            if let Some(id) = inst_id {
                if let Some(&voice_idx) = voice_map.get(id) {
                    if current_voice != Some(voice_idx) {
                        out.push(SmpsEvent::SetVoice(voice_idx));
                        current_voice = Some(voice_idx);
                    }
                }
            }

            let new_mod = note_mod.as_ref().map(|m| (m.wait, m.speed, m.delta, m.steps));
            if new_mod != current_mod {
                if let Some((w, s, d, st)) = new_mod {
                    out.push(SmpsEvent::SetModulation { wait: w, speed: s, delta: d, steps: st });
                }
                current_mod = new_mod;
            }

            if detune != current_detune {
                out.push(SmpsEvent::AlterNote(detune));
                current_detune = detune;
            }
        }

        if matches!(channel, ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise) {
            if let Some(id) = inst_id {
                if let Some(&env_idx) = psg_env_map.get(id) {
                    if current_psg_env != Some(env_idx) {
                        out.push(SmpsEvent::SetPsgVoice(env_idx));
                        current_psg_env = Some(env_idx);
                    }
                }
            }

            let new_mod = note_mod.as_ref().map(|m| (m.wait, m.speed, m.delta, m.steps));
            if new_mod != current_mod {
                if let Some((w, s, d, st)) = new_mod {
                    out.push(SmpsEvent::SetModulation { wait: w, speed: s, delta: d, steps: st });
                }
                current_mod = new_mod;
            }

            if detune != current_detune {
                out.push(SmpsEvent::AlterNote(detune));
                current_detune = detune;
            }
        }

        if let Some(pan_byte) = pan_override {
            if current_pan != Some(*pan_byte) && matches!(channel, ChannelAssignment::Fm(_) | ChannelAssignment::Dac(_)) {
                out.push(SmpsEvent::SetPan(*pan_byte));
                current_pan = Some(*pan_byte);
            }
        }

        let note_vol_delta = (127i16 - velocity as i16).clamp(0, 127);
        if note_vol_delta != current_vol_delta && matches!(channel, ChannelAssignment::Fm(_)) {
            let change = (note_vol_delta - current_vol_delta) as i8;
            out.push(SmpsEvent::VolumeChange(change));
            current_vol_delta = note_vol_delta;
        }

        let smps_dur = daw_to_smps_duration(dur_ticks, params)
            .ok_or_else(|| ExportError {
                track_name: String::new(),
                region_index: None,
                note_index: None,
                message: format!("Note duration {dur_ticks} DAW ticks rounds to 0 SMPS ticks"),
            })?;

        let pitch_name = match channel {
            ChannelAssignment::Dac(_) => dac_sample_name(pitch),
            ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => smps_note_name_psg(pitch),
            _ => smps_note_name(pitch),
        };
        emit_duration_events(&mut out, Some(pitch_name), smps_dur);

        cursor = tick + dur_ticks;
    }

    if cursor < region_duration {
        let gap = region_duration - cursor;
        if let Some(smps_gap) = daw_to_smps_duration(gap, params) {
            emit_duration_events(&mut out, None, smps_gap);
        }
    }

    Ok(out)
}

fn emit_duration_events(out: &mut Vec<SmpsEvent>, pitch_name: Option<String>, total: u64) {
    let mut remaining = total;
    let mut first = true;

    while remaining > 0 {
        let chunk = remaining.min(SMPS_MAX_DURATION);

        if !first && pitch_name.is_some() {
            out.push(SmpsEvent::Tie);
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

// --- Voice Bank ---

/// Build a deduplicated voice index from active FM tracks, including per-note voices.
pub fn build_voice_index<'a>(tracks: &[Track], instruments: &'a InstrumentBank) -> (HashMap<Uuid, u8>, Vec<&'a FmInstrument>) {
    let mut map = HashMap::new();
    let mut voices: Vec<&FmInstrument> = Vec::new();

    let add_voice = |id: &Uuid, map: &mut HashMap<Uuid, u8>, voices: &mut Vec<&'a FmInstrument>| {
        if map.contains_key(id) { return; }
        if let Some(inst) = instruments.fm.iter().find(|i| &i.id == id) {
            let idx = voices.len() as u8;
            map.insert(*id, idx);
            voices.push(inst);
        }
    };

    for track in tracks {
        if track.muted { continue; }
        if !matches!(track.channel, ChannelAssignment::Fm(_)) { continue; }
        if let Some(inst_id) = &track.instrument_id {
            add_voice(inst_id, &mut map, &mut voices);
        }
        for region in &track.regions {
            for note in &region.notes {
                if let Some(inst_id) = &note.instrument_id {
                    add_voice(inst_id, &mut map, &mut voices);
                }
            }
        }
    }

    (map, voices)
}

/// Generate the voice bank assembly text.
pub fn generate_voice_bank_asm(song_label: &str, voices: &[&FmInstrument], driver: &dyn DriverProfile) -> String {
    let mut asm = String::new();
    asm.push_str("; ============================================================\n");
    asm.push_str(&format!("; Voice Bank: {song_label}\n"));
    asm.push_str("; Exported from Seraph\n");
    asm.push_str("; ============================================================\n\n");
    asm.push_str(&format!("Snd_{song_label}_Voices:\n"));

    for (i, inst) in voices.iter().enumerate() {
        let bytes = driver.fm_to_bytes(inst);

        let alg = bytes[24] & 0x07;
        let fb = (bytes[24] >> 3) & 0x07;

        asm.push_str(&format!("\n; Voice {i} - \"{}\"\n", inst.name));
        asm.push_str(&format!("\tsmpsVcAlgorithm\t\t${alg:02X}\n"));
        asm.push_str(&format!("\tsmpsVcFeedback\t\t${fb:02X}\n"));
        asm.push_str("\tsmpsVcUnusedBits\t$00\n");

        let dt: Vec<String> = (0..4).map(|j| format!("${:02X}", (bytes[j] >> 4) & 0x07)).collect();
        let mul: Vec<String> = (0..4).map(|j| format!("${:02X}", bytes[j] & 0x0F)).collect();
        asm.push_str(&format!("\tsmpsVcDetune\t\t{}\n", dt.join(", ")));
        asm.push_str(&format!("\tsmpsVcCoarseFreq\t{}\n", mul.join(", ")));

        let rs: Vec<String> = (4..8).map(|j| format!("${:02X}", (bytes[j] >> 6) & 0x03)).collect();
        let ar: Vec<String> = (4..8).map(|j| format!("${:02X}", bytes[j] & 0x1F)).collect();
        asm.push_str(&format!("\tsmpsVcRateScale\t\t{}\n", rs.join(", ")));
        asm.push_str(&format!("\tsmpsVcAttackRate\t{}\n", ar.join(", ")));

        let am: Vec<String> = (8..12).map(|j| format!("${:02X}", (bytes[j] >> 7) & 0x01)).collect();
        let d1r: Vec<String> = (8..12).map(|j| format!("${:02X}", bytes[j] & 0x1F)).collect();
        asm.push_str(&format!("\tsmpsVcAmpMod\t\t{}\n", am.join(", ")));
        asm.push_str(&format!("\tsmpsVcDecayRate1\t{}\n", d1r.join(", ")));

        let d2r: Vec<String> = (12..16).map(|j| format!("${:02X}", bytes[j] & 0x1F)).collect();
        asm.push_str(&format!("\tsmpsVcDecayRate2\t{}\n", d2r.join(", ")));

        let sl: Vec<String> = (16..20).map(|j| format!("${:02X}", (bytes[j] >> 4) & 0x0F)).collect();
        let rr: Vec<String> = (16..20).map(|j| format!("${:02X}", bytes[j] & 0x0F)).collect();
        asm.push_str(&format!("\tsmpsVcDecayLevel\t{}\n", sl.join(", ")));
        asm.push_str(&format!("\tsmpsVcReleaseRate\t{}\n", rr.join(", ")));

        let tl: Vec<String> = (20..24).map(|j| format!("${:02X}", bytes[j] & 0x7F)).collect();
        asm.push_str(&format!("\tsmpsVcTotalLevel\t{}\n", tl.join(", ")));
    }

    asm
}

// --- Assembly Generation ---

use crate::model::song::Song;

pub fn sanitize_label(name: &str) -> String {
    let stripped = name.strip_prefix("Snd_").or_else(|| name.strip_prefix("Snd ")).unwrap_or(name);
    stripped.chars()
        .map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' })
        .collect()
}

fn channel_type_label(ch: &ChannelAssignment) -> &'static str {
    match ch {
        ChannelAssignment::Fm(_) => "FM",
        ChannelAssignment::Psg(_) => "PSG",
        ChannelAssignment::PsgNoise => "PSG_Noise",
        ChannelAssignment::Dac(_) => "DAC",
    }
}

fn channel_index(ch: &ChannelAssignment) -> u8 {
    match ch {
        ChannelAssignment::Fm(n) => n + 1,
        ChannelAssignment::Psg(n) => n + 1,
        ChannelAssignment::PsgNoise => 4,
        ChannelAssignment::Dac(n) => n + 1,
    }
}

fn flush_line(asm: &mut String, items: &mut Vec<String>) {
    if items.is_empty() { return; }
    asm.push_str(&format!("\tdc.b {}\n", items.join(", ")));
    items.clear();
}

fn format_channel_data(events: &[SmpsEvent]) -> String {
    let mut asm = String::new();
    let mut line_items: Vec<String> = Vec::new();
    let mut last_duration: Option<u64> = None;

    for event in events {
        match event {
            SmpsEvent::SetVoice(idx) => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsSetvoice\t${idx:02X}\n"));
                last_duration = None;
            }
            SmpsEvent::SetPan(pan_byte) => {
                flush_line(&mut asm, &mut line_items);
                let pan_name = match pan_byte {
                    0x80 => "panLeft",
                    0x40 => "panRight",
                    _ => "panCenter",
                };
                asm.push_str(&format!("\tsmpsPan\t\t\t{pan_name}, $00\n"));
            }
            SmpsEvent::SetPsgVoice(idx) => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsPSGvoice\t\tsTone_{idx:02X}\n"));
            }
            SmpsEvent::VolumeChange(delta) => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsFMAlterVol\t${:02X}\n", *delta as u8));
                last_duration = None;
            }
            SmpsEvent::SetModulation { wait, speed, delta, steps } => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsModSet\t\t${wait:02X}, ${speed:02X}, ${delta:02X}, ${steps:02X}\n"));
                last_duration = None;
            }
            SmpsEvent::PsgForm(byte) => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsPSGform\t\t${byte:02X}\n"));
                last_duration = None;
            }
            SmpsEvent::AlterNote(val) => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str(&format!("\tsmpsAlterNote\t${:02X}\n", *val as u8));
                last_duration = None;
            }
            SmpsEvent::Tie => {
                flush_line(&mut asm, &mut line_items);
                asm.push_str("\tsmpsNoAttack\n");
                last_duration = None;
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

/// Generate the complete music assembly file.
pub fn generate_music_asm(
    song: &Song,
    instruments: &InstrumentBank,
    voice_map: &HashMap<Uuid, u8>,
    params: &SmpsTempoParams,
) -> Result<String, Vec<ExportError>> {
    let label = sanitize_label(&song.metadata.name);
    let mut asm = String::new();

    asm.push_str("; ============================================================\n");
    asm.push_str(&format!("; Song: {}\n", song.metadata.name));
    asm.push_str("; Exported from Seraph\n");
    asm.push_str("; ============================================================\n\n");

    let active_tracks: Vec<&Track> = song.tracks.iter()
        .filter(|t| !t.muted && t.regions.iter().any(|r| !r.notes.is_empty()))
        .collect();
    let fm_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Fm(_))).count();
    let dac_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Dac(_))).count();
    let psg_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise)).count();
    // `smpsHeaderChan`'s FIRST byte counts DAC together with FM; it is not an
    // FM-only count. Checked against artifacts rather than reasoning (F30):
    //
    //   * The driver reads it as such -- `ld b, (iy+2) ; b = number of FM +
    //     DAC channels`, `ld a, (iy+3) ; Get number of PSG tracks`, and
    //     `ld de, 6 / add hl, de` for where the channel entries start
    //     (`skdisasm/Sound/Z80 Sound Driver.asm:1836-1839, 1859-1861`), which
    //     lines up byte-for-byte with the macro layout in
    //     `_smps2asm_inc.asm:306` (`smpsHeaderChan macro fm,psg / dc.b fm,psg`)
    //     following the 2-byte `smpsHeaderVoice` pointer.
    //   * Real S3K songs agree: 59 of the 60 files in `skdisasm/Sound/Music/`
    //     declare `smpsHeaderChan $06, $03` and carry exactly one
    //     `smpsHeaderDAC` + five `smpsHeaderFM` + three `smpsHeaderPSG`
    //     entries (e.g. `MGZ1.asm:4-15`). 6 = 1 DAC + 5 FM, not 5.
    //
    // So `fm_count + dac_count` is right. The out-of-driver channel that F30
    // was booked for is refused in `validate_for_export`, which `write_export`
    // runs before this function.
    //
    // F37: the FM/DAC channel entries are not a list, they are SLOTS, and slot
    // 0 is the drum slot. `.fm_dac_loop` copies header entry N into track slot
    // N (`Z80 Sound Driver.asm:1836-1857`), pairing each with a fixed init byte
    // from `zFMDACInitBytes` (`ibid.:1893-1906`) whose own comment reads "The
    // first is for DAC; then 0, 1, 2 then 4, 5, 6 for the FM channels". Slot 0
    // is `zSongFM6_DAC` (`ibid.:176-183`) and `zUpdateMusic` drives it through
    // `zUpdateDACTrack` unconditionally (`ibid.:717-719`), reading its bytes as
    // sample ids rather than notes.
    //
    // So a song with no drum track used to export its FIRST FM entry into the
    // drum slot: that instrument's notes played as drum samples, and the export
    // reported success. Every one of the 60 shipped songs in
    // `skdisasm/Sound/Music/` writes exactly one `smpsHeaderDAC`, first, at
    // line 7 -- including the one drumless song -- so the entry is an invariant
    // of the format, not a property of having drums.
    //
    // The fix synthesizes one for EVERY drumless export, whether or not the
    // song has FM tracks. Uniform on purpose (owner ruling `d-11`,
    // `synthesize`): one code path and one shape to reason about. For a
    // drumless song with no FM tracks nothing audible changes; for one with FM
    // tracks, its first instrument stops being played as drums.
    let needs_silent_dac = dac_count == 0;
    let silent_dac_label = format!("Snd_{label}_DAC_Silent");
    let fm_dac_count = fm_count + dac_count + usize::from(needs_silent_dac);

    asm.push_str(&format!("Snd_{label}_Header:\n"));
    asm.push_str("\tsmpsHeaderStartSong 3\n");
    asm.push_str(&format!("\tsmpsHeaderVoice\t\tSnd_{label}_Voices\n"));
    asm.push_str(&format!("\tsmpsHeaderChan\t\t${fm_dac_count:02X}, ${psg_count:02X}\n"));
    asm.push_str(&format!("\tsmpsHeaderTempo\t\t${:02X}, ${:02X}\n", params.divider, params.modifier));

    // The DAC entry goes FIRST -- it is slot 0 -- which is also where all 60
    // shipped songs put it (line 7 of every file in `skdisasm/Sound/Music/`).
    if needs_silent_dac {
        asm.push_str(&format!("\tsmpsHeaderDAC\t\t{silent_dac_label}, $00, $00\n"));
    }
    for track in &active_tracks {
        if !matches!(track.channel, ChannelAssignment::Dac(_)) { continue; }
        let ch_label = format!("Snd_{label}_DAC{}", channel_index(&track.channel));
        asm.push_str(&format!("\tsmpsHeaderDAC\t\t{ch_label}, $00, $00\n"));
    }
    for track in &active_tracks {
        if !matches!(track.channel, ChannelAssignment::Fm(_)) { continue; }
        let ch_label = format!("Snd_{label}_FM{}", channel_index(&track.channel));
        let vol = (127u8).saturating_sub(track.volume);
        let pitch = track.pitch_offset as u8;
        asm.push_str(&format!("\tsmpsHeaderFM\t\t{ch_label}, ${pitch:02X}, ${vol:02X}\n"));
    }
    for track in &active_tracks {
        if !matches!(track.channel, ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise) { continue; }
        let ch_label = format!("Snd_{label}_{}{}", channel_type_label(&track.channel), channel_index(&track.channel));
        let vol = 15u8.saturating_sub(((track.volume as u16 * 15 + 63) / 127) as u8);
        let pitch = track.pitch_offset as u8;
        let env_name = track.instrument_id
            .and_then(|id| instruments.psg.iter().find(|i| i.id == id))
            .and_then(|i| i.smps_envelope_index)
            .map(|idx| format!("sTone_{:02X}", idx))
            .unwrap_or_else(|| "$00".to_string());
        asm.push_str(&format!("\tsmpsHeaderPSG\t\t{ch_label}, ${pitch:02X}, ${vol:02X}, $00, {env_name}\n"));
    }

    asm.push('\n');

    // The body for the entry above. `smpsHeaderDAC` puts its operand through
    // `CheckedChannelPointer` (`_smps2asm_inc.asm:317-318`), so this label has
    // to be DEFINED or the exported file will not assemble at all.
    //
    // A lone `smpsStop` is copied from the one shipped drumless song, not
    // invented: `Chaos Emerald.asm:74-78` is a bare `Snd_Emerald_DAC:` falling
    // straight through into `Snd_Emerald_PSG3: / smpsStop`. That assembles to
    // the single byte $F2 (`_smps2asm_inc.asm:580-582`), whose handler
    // `cfStopTrack` does `res 7, (ix+zTrack.PlaybackControl)`
    // (`Z80 Sound Driver.asm:3443-3444`) -- precisely the bit `zUpdateMusic`
    // tests before it calls `zUpdateDACTrack` (`ibid.:717-719`). The slot is
    // therefore entered once, stops, and is never updated again: it neither
    // hangs nor plays a sample. Resting forever or looping on a rest would
    // instead keep the slot running for the whole song; the shipped song does
    // not do that, so this does not either.
    if needs_silent_dac {
        asm.push_str("; ------------------------------------------------------------\n");
        asm.push_str("; DAC Channel - silent placeholder (this song has no drum track)\n");
        asm.push_str("; Track slot 0 is always driven as drums, so it must be claimed\n");
        asm.push_str("; or the first FM entry would play through the drum channel.\n");
        asm.push_str("; ------------------------------------------------------------\n");
        asm.push_str(&format!("{silent_dac_label}:\n"));
        asm.push_str("\tsmpsStop\n");
        asm.push('\n');
    }

    let psg_env_map: HashMap<Uuid, u8> = instruments.psg.iter()
        .filter_map(|i| i.smps_envelope_index.map(|idx| (i.id, idx)))
        .collect();

    let mut all_errors = Vec::new();

    for track in &active_tracks {
        let type_label = channel_type_label(&track.channel);
        let ch_idx = channel_index(&track.channel);
        let ch_label = format!("Snd_{label}_{type_label}{ch_idx}");

        asm.push_str("; ------------------------------------------------------------\n");
        asm.push_str(&format!("; {type_label} Channel {ch_idx} - \"{}\"\n", track.name));
        asm.push_str("; ------------------------------------------------------------\n");
        asm.push_str(&format!("{ch_label}:\n"));

        let mut events: Vec<SmpsEvent> = Vec::new();

        if matches!(track.channel, ChannelAssignment::Fm(_) | ChannelAssignment::Dac(_)) {
            let has_note_pan = track.regions.iter()
                .flat_map(|r| &r.notes)
                .any(|n| n.pan_override.is_some());
            if !has_note_pan && !matches!(track.pan, Pan::Center) {
                let pan_byte = match track.pan {
                    Pan::Left => 0x80u8,
                    Pan::Right => 0x40,
                    Pan::Center => 0xC0,
                };
                events.push(SmpsEvent::SetPan(pan_byte));
            }
        }

        if matches!(track.channel, ChannelAssignment::PsgNoise) {
            if let Some(ref inst_id) = track.instrument_id {
                if let Some(psg_inst) = instruments.psg.iter().find(|i| &i.id == inst_id) {
                    if let Some(ref nm) = psg_inst.noise_mode {
                        let byte = match nm {
                            crate::model::instrument::NoiseMode::Periodic(f) => 0xE0 | ((*f as u8) & 0x03),
                            crate::model::instrument::NoiseMode::White(f) => 0xE0 | 0x04 | ((*f as u8) & 0x03),
                        };
                        events.push(SmpsEvent::PsgForm(byte));
                    }
                }
            }
        }

        let mut all_notes: Vec<(u64, u8, u64, u8, Option<Uuid>, i8, Option<u8>, Option<NoteModulation>)> = Vec::new();
        for region in &track.regions {
            for note in &region.notes {
                let abs_tick = region.start_tick + note.tick;
                all_notes.push((abs_tick, note.pitch, note.duration_ticks, note.velocity, note.instrument_id, note.detune, note.pan_override, note.modulation.clone()));
            }
        }
        all_notes.sort_by_key(|&(tick, _, _, _, _, _, _, _)| tick);

        let total_duration = track.regions.iter()
            .map(|r| r.start_tick + r.duration_ticks)
            .max()
            .unwrap_or(0);

        match encode_channel_events_with_voices(&all_notes, total_duration, params, &track.channel, voice_map, &psg_env_map) {
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

// --- Validation ---

/// Validate the song for SMPS export. Returns a list of errors (empty = valid).
///
/// `driver` is the profile the song will be exported for. It is needed because
/// which channels exist is a property of the driver, not of the format: see the
/// channel-existence check below (audit F30).
pub fn validate_for_export(
    song: &Song,
    instruments: &InstrumentBank,
    driver: &dyn DriverProfile,
    params: &SmpsTempoParams,
) -> Vec<ExportError> {
    let mut errors = Vec::new();
    let layout = driver.channel_layout();

    for track in &song.tracks {
        if track.muted { continue; }

        let has_notes = track.regions.iter().any(|r| !r.notes.is_empty());

        // F30. A track's channel must be one the driver actually has, or the
        // song header describes a voice that does not exist.
        //
        // `ChannelAssignment::Fm(n)` was never checked against the driver
        // anywhere in the app, so a project saved before F31 corrected
        // `FlamedriverProfile::channel_layout()` still carries an `Fm(5)`
        // track. Exporting it emitted `smpsHeaderFM Snd_<song>_FM6` and
        // reported success.
        //
        // On this driver that entry is not merely cosmetic. `zBGMLoad` fills
        // the FM/DAC track slots POSITIONALLY -- `ld b, (iy+2)` then a loop
        // copying into `zTracksStart` onward
        // (`skdisasm/Sound/Z80 Sound Driver.asm:1837-1857`) -- and the slots
        // are `zSongFM6_DAC, zSongFM1..zSongFM5` (177-182), six of them, with
        // slot 0 unconditionally driven as the DAC (717-719). So an "FM6"
        // entry lands on whatever slot its position happens to reach, and a
        // song using all six FM voices plus DAC needs seven entries where the
        // driver has six -- the seventh runs off the end into `zSongPSG1`.
        //
        // The check is gated on `has_notes` because that is exactly
        // `generate_music_asm`'s own rule for which tracks reach the header
        // (`!t.muted && ...any(|r| !r.notes.is_empty())`). A leftover empty
        // FM6 lane -- which every pre-F31 project has, since lanes were seeded
        // from the layout -- emits nothing and must not block an export.
        //
        // What such a track should SOUND like (steal a voice? merge onto the
        // DAC? drop it?) is a parked owner design call (audit F27). This
        // refuses and says which track and why rather than guessing, and
        // rather than the current silent success.
        if has_notes && layout.channel_name(&track.channel).is_none() {
            errors.push(ExportError {
                track_name: track.name.clone(),
                region_index: None,
                note_index: None,
                message: format!(
                    "Channel {}{} does not exist on driver \"{}\". Its channels are: {}. \
                     Exporting would write a song header entry for a voice this driver \
                     cannot play. Move this track to a channel the driver has.",
                    channel_type_label(&track.channel),
                    channel_index(&track.channel),
                    driver.name(),
                    layout.channel_names().join(", "),
                ),
            });
            continue;
        }

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

        if !has_notes {
            continue;
        }

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

        for (ri, region) in track.regions.iter().enumerate() {
            for (ni, note) in region.notes.iter().enumerate() {
                let skip_pitch_check = matches!(track.channel, ChannelAssignment::Dac(_) | ChannelAssignment::PsgNoise);
                // PSG has its OWN range and it is not the FM one. The check
                // below used `midi_to_smps_note` (MIDI 12-106) for every
                // pitched channel, which was wrong for PSG in BOTH directions:
                //
                //   * MIDI 12-35 PASSED validation and was then silently
                //     retuned -- `smps_note_name_psg` clamps with `.max(0)` and
                //     `midi_to_psg_period` returns the bottom entry -- so a low
                //     PSG bass note exported and previewed as a DIFFERENT note
                //     with nothing said. App and export agreed with each other
                //     while neither matched what the author wrote.
                //   * MIDI 107-119 was REJECTED even though the table and
                //     `smps_note_name_psg` both handle it.
                //
                // Range derived from the driver's own table, not chosen here:
                // `PSG_PERIOD_TABLE` spans z80 index 0-83 mapped as
                // midi = index + 36, i.e. 36-119.
                if matches!(track.channel, ChannelAssignment::Psg(_)) {
                    if !crate::audio::frequency::psg_pitch_is_representable(note.pitch) {
                        errors.push(ExportError {
                            track_name: track.name.clone(),
                            region_index: Some(ri),
                            note_index: Some(ni),
                            message: format!(
                                "Pitch {} is outside the PSG range (MIDI {}-{}). It would be \
                                 silently retuned to the nearest note this driver can play, \
                                 not left out, so the exported song would differ from what \
                                 you wrote.",
                                note.pitch,
                                crate::audio::frequency::PSG_MIDI_LOW,
                                crate::audio::frequency::PSG_MIDI_HIGH,
                            ),
                        });
                    }
                } else if !skip_pitch_check && midi_to_smps_note(note.pitch).is_none() {
                    errors.push(ExportError {
                        track_name: track.name.clone(),
                        region_index: Some(ri),
                        note_index: Some(ni),
                        message: format!("Pitch {} is outside SMPS range (C0-Bb7, MIDI 12-106)", note.pitch),
                    });
                }

                if !skip_pitch_check && daw_to_smps_duration(note.duration_ticks, params).is_none() {
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

    errors
}

// --- File Writing ---

use crate::export::ExportResult;
use std::fs;
use std::path::Path;

/// Write all export files to the output directory.
pub fn write_export(
    song: &Song,
    instruments: &InstrumentBank,
    driver: &dyn DriverProfile,
    output_dir: &Path,
    project_dir: Option<&Path>,
) -> Result<ExportResult, Vec<ExportError>> {
    let params = compute_tempo_params(song.metadata.tempo, song.metadata.ticks_per_beat);

    let errors = validate_for_export(song, instruments, driver, &params);
    if !errors.is_empty() {
        return Err(errors);
    }

    let (voice_map, voices) = build_voice_index(&song.tracks, instruments);

    let music_asm = generate_music_asm(song, instruments, &voice_map, &params)?;
    let voice_asm = generate_voice_bank_asm(&sanitize_label(&song.metadata.name), &voices, driver);

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

    let dac_tracks: Vec<&Track> = song.tracks.iter()
        .filter(|t| !t.muted && matches!(t.channel, ChannelAssignment::Dac(_)))
        .filter(|t| t.regions.iter().any(|r| !r.notes.is_empty()))
        .collect();
    if !dac_tracks.is_empty() {
        let dac_dir = output_dir.join("dac");
        fs::create_dir_all(&dac_dir).map_err(|e| vec![ExportError {
            track_name: String::new(), region_index: None, note_index: None,
            message: format!("Failed to create dac directory: {e}"),
        }])?;

        for track in dac_tracks {
            // `instrument_id` set and present in the bank are both already
            // guaranteed: `validate_for_export` above errors with "No
            // instrument assigned" / "Assigned instrument not found" and
            // `write_export` returns early on any validation error. These
            // remain as defensive matches, but they must not SKIP silently,
            // which is how F33 hid.
            let Some(inst_id) = &track.instrument_id else {
                return Err(vec![ExportError {
                    track_name: track.name.clone(), region_index: None, note_index: None,
                    message: "DAC track has no instrument assigned, so no sample could be exported".into(),
                }]);
            };
            let Some(inst) = instruments.dac.iter().find(|i| &i.id == inst_id) else {
                return Err(vec![ExportError {
                    track_name: track.name.clone(), region_index: None, note_index: None,
                    message: "DAC track's instrument is not in the instrument bank, so no sample could be exported".into(),
                }]);
            };

            // F33. `pcm_file` is a BARE FILENAME (`commands.rs` writes
            // `format!("{id}.pcm")`), and every other consumer resolves it as
            // `<project>/instruments/dac/<pcm_file>` (`manager.rs`,
            // `import/mod.rs`, `commands.rs`). This used to do
            // `Path::new(&inst.pcm_file)`, resolving it against the process
            // CWD, where it essentially never exists -- and the `.exists()`
            // guard had no `else`, so every DAC sample was dropped in silence
            // from the export that actually reaches the game.
            let Some(project_dir) = project_dir else {
                return Err(vec![ExportError {
                    track_name: track.name.clone(), region_index: None, note_index: None,
                    message: "Cannot export DAC samples because the project has not been saved to disk yet. Save the project, then export.".into(),
                }]);
            };
            let pcm_src = project_dir.join("instruments/dac").join(&inst.pcm_file);
            if !pcm_src.exists() {
                return Err(vec![ExportError {
                    track_name: track.name.clone(), region_index: None, note_index: None,
                    message: format!(
                        "DAC sample file is missing: expected it at {}. The export would \
                         otherwise have produced music with no percussion.",
                        pcm_src.display(),
                    ),
                }]);
            }
            let dest = dac_dir.join(&inst.pcm_file);
            if let Err(e) = fs::copy(&pcm_src, &dest) {
                return Err(vec![ExportError {
                    track_name: track.name.clone(), region_index: None, note_index: None,
                    message: format!("Failed to copy DAC sample from {}: {e}", pcm_src.display()),
                }]);
            }
            files.push(dest.to_string_lossy().into_owned());
        }
    }

    Ok(ExportResult { files })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::instrument::{DacInstrument, InstrumentMetadata};
    use crate::model::song::{Note, Region, Song, SongMetadata};
    use std::fs;

    fn psg_song_with_pitch(pitch: u8) -> Song {
        use crate::model::instrument::PsgInstrument;
        let inst_id = Uuid::new_v4();
        Song {
            metadata: SongMetadata {
                name: "PsgRange".into(),
                tempo: 120.0,
                time_signature: (4, 4),
                ticks_per_beat: 480,
                driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: Uuid::new_v4(),
                name: "PSG1".into(),
                channel: ChannelAssignment::Psg(0),
                instrument_id: Some(inst_id),
                regions: vec![Region {
                    id: Uuid::new_v4(),
                    start_tick: 0,
                    duration_ticks: 480,
                    notes: vec![Note {
                        tick: 0, pitch, velocity: 100, duration_ticks: 240,
                        instrument_id: Some(inst_id), detune: 0,
                        pan_override: None, modulation: None,
                    }],
                    instrument_id: None,
                }],
                muted: false, solo: false, volume: 127, pan: Pan::Center,
                pitch_offset: 0, modulation: None,
            }],
            instruments: InstrumentBank {
                fm: vec![],
                psg: vec![PsgInstrument {
                    id: inst_id,
                    name: "P".into(),
                    volume_sequence: vec![0],
                    loop_point: None,
                    silence_on_end: false,
                    noise_mode: None,
                    smps_envelope_index: None,
                    metadata: InstrumentMetadata::default(),
                }],
                dac: vec![],
            },
        }
    }

    fn psg_pitch_errors(pitch: u8) -> Vec<String> {
        let song = psg_song_with_pitch(pitch);
        let params = compute_tempo_params(120.0, 480);
        validate_for_export(&song, &song.instruments, &crate::driver::FlamedriverProfile, &params)
            .into_iter()
            .map(|e| e.message)
            .collect()
    }

    /// A PSG note below the driver's table used to PASS validation and then be
    /// silently retuned: `smps_note_name_psg` clamps with `.max(0)` and
    /// `midi_to_psg_period` returns the bottom entry, so the export and the
    /// preview agreed with each other while neither matched what was written.
    #[test]
    fn a_psg_note_below_the_drivers_range_is_reported_not_silently_retuned() {
        let msgs = psg_pitch_errors(20);
        assert!(
            msgs.iter().any(|m| m.contains("outside the PSG range") && m.contains("retuned")),
            "MIDI 20 on PSG must be reported as retunable, got {msgs:?}",
        );
    }

    /// The other direction of the same defect: MIDI 107-119 IS representable on
    /// PSG (the table spans 36-119) but the old channel-agnostic check used
    /// FM's 12-106 bound and rejected it.
    #[test]
    fn a_high_psg_note_the_driver_can_play_is_no_longer_falsely_rejected() {
        let msgs = psg_pitch_errors(115);
        assert!(
            !msgs.iter().any(|m| m.contains("range")),
            "MIDI 115 is within the PSG table (36-119) and must not be refused, got {msgs:?}",
        );
    }

    /// Control: the fix must not simply stop checking PSG pitches.
    #[test]
    fn a_psg_note_above_the_drivers_range_is_still_reported() {
        let msgs = psg_pitch_errors(125);
        assert!(
            msgs.iter().any(|m| m.contains("outside the PSG range")),
            "MIDI 125 is past the PSG table's top (119) and must be reported, got {msgs:?}",
        );
    }

    /// Control: an in-range PSG note must raise nothing at all, or the three
    /// assertions above could pass with a check that fires unconditionally.
    #[test]
    fn an_in_range_psg_note_raises_nothing() {
        let msgs = psg_pitch_errors(60);
        assert!(
            msgs.is_empty(),
            "MIDI 60 on PSG is ordinary and must raise nothing, got {msgs:?}",
        );
    }

    /// Builds a saved-project layout on disk with one DAC sample, plus a song
    /// whose single DAC track uses it. Returns (project_dir, song, bank).
    fn dac_project(sample_bytes: &[u8]) -> (tempfile::TempDir, Song, InstrumentBank) {
        let proj = tempfile::tempdir().unwrap();
        let dac_dir = proj.path().join("instruments/dac");
        fs::create_dir_all(&dac_dir).unwrap();
        let inst_id = Uuid::new_v4();
        let pcm_name = format!("{inst_id}.pcm");
        fs::write(dac_dir.join(&pcm_name), sample_bytes).unwrap();

        let inst = DacInstrument {
            id: inst_id,
            name: "Kick".into(),
            target_sample_rate: 8000,
            loop_start: None,
            loop_length: None,
            original_file: "kick.wav".into(),
            pcm_file: pcm_name,
            source_is_raw: false,
            metadata: InstrumentMetadata::default(),
        };
        let song = Song {
            metadata: SongMetadata {
                name: "DacSong".into(),
                tempo: 120.0,
                time_signature: (4, 4),
                ticks_per_beat: 480,
                driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: Uuid::new_v4(),
                name: "Drums".into(),
                channel: ChannelAssignment::Dac(0),
                instrument_id: Some(inst_id),
                regions: vec![Region {
                    id: Uuid::new_v4(),
                    start_tick: 0,
                    duration_ticks: 480,
                    notes: vec![Note {
                        tick: 0, pitch: 60, velocity: 100, duration_ticks: 240,
                        instrument_id: Some(inst_id), detune: 0,
                        pan_override: None, modulation: None,
                    }],
                    instrument_id: None,
                }],
                muted: false, solo: false, volume: 127, pan: Pan::Center,
                pitch_offset: 0, modulation: None,
            }],
            instruments: InstrumentBank { fm: vec![], psg: vec![], dac: vec![inst] },
        };
        let bank = song.instruments.clone();
        (proj, song, bank)
    }

    /// F33. The export that actually reaches the game must carry the drum
    /// sample. It used to resolve `pcm_file` (a bare filename) against the
    /// process CWD, where it never exists, and skip on a bare `.exists()`
    /// check with no `else` -- so every DAC sample vanished in silence while
    /// the export reported success.
    #[test]
    fn the_dac_sample_is_copied_into_the_export() {
        let bytes: Vec<u8> = (0u8..64).collect();
        let (proj, song, bank) = dac_project(&bytes);
        let out = tempfile::tempdir().unwrap();

        let result = write_export(
            &song, &bank, &crate::driver::FlamedriverProfile,
            out.path(), Some(proj.path()),
        ).expect("export should succeed");

        let copied = out.path().join("dac").join(&bank.dac[0].pcm_file);
        assert!(
            copied.exists(),
            "the DAC sample must be copied into the export; expected it at {}",
            copied.display(),
        );
        assert_eq!(
            fs::read(&copied).unwrap(), bytes,
            "the copied sample must be byte-identical to the source",
        );
        assert!(
            result.files.iter().any(|f| f.contains(".pcm")),
            "the exported file list must name the sample, got {:?}", result.files,
        );
    }

    /// The control: a missing sample must be a REPORTED failure, never a
    /// silent skip. Without this, the fix above could regress to skipping and
    /// the export would still "succeed" with no percussion.
    #[test]
    fn a_missing_dac_sample_is_reported_not_skipped() {
        let (proj, song, bank) = dac_project(&[1, 2, 3, 4]);
        fs::remove_file(proj.path().join("instruments/dac").join(&bank.dac[0].pcm_file)).unwrap();
        let out = tempfile::tempdir().unwrap();

        let errors = write_export(
            &song, &bank, &crate::driver::FlamedriverProfile,
            out.path(), Some(proj.path()),
        ).expect_err("a missing DAC sample must fail the export, not pass silently");

        assert!(
            errors.iter().any(|e| e.message.contains("DAC sample file is missing")),
            "the failure must name the missing sample, got {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>(),
        );
    }

    /// An unsaved project cannot resolve sample filenames at all. That must be
    /// a clear instruction, not a silent drop.
    #[test]
    fn exporting_dac_from_an_unsaved_project_says_to_save_first() {
        let (_proj, song, bank) = dac_project(&[9, 9, 9]);
        let out = tempfile::tempdir().unwrap();

        let errors = write_export(
            &song, &bank, &crate::driver::FlamedriverProfile,
            out.path(), None,
        ).expect_err("no project dir means samples cannot be resolved");

        assert!(
            errors.iter().any(|e| e.message.contains("has not been saved")),
            "must tell the user to save first, got {:?}",
            errors.iter().map(|e| &e.message).collect::<Vec<_>>(),
        );
    }

    #[test]
    fn test_tempo_120bpm_480tpb() {
        let params = compute_tempo_params(120.0, 480);
        assert!(params.modifier > 0);
        assert!(params.divider >= 1);
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
        let result = daw_to_smps_duration(0, &params);
        assert!(result.is_none());
    }

    #[test]
    fn test_long_note_converts() {
        let params = compute_tempo_params(120.0, 480);
        let whole = daw_to_smps_duration(1920, &params).unwrap();
        assert!(whole > 0);
    }

    #[test]
    fn test_midi_to_smps_note_c4() {
        assert_eq!(midi_to_smps_note(60), Some(0x81 + 48));
    }

    #[test]
    fn test_midi_to_smps_note_out_of_range() {
        assert_eq!(midi_to_smps_note(11), None);
        assert_eq!(midi_to_smps_note(107), None);
        assert_eq!(midi_to_smps_note(106), Some(0xDF));
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
        let notes = vec![(0u64, 60u8, 480u64)];
        let fm = ChannelAssignment::Fm(0);
        let events = encode_channel_events(&notes, 480, &params, &fm).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Note { .. })));
    }

    #[test]
    fn test_encode_gap_produces_rest() {
        let params = compute_tempo_params(120.0, 480);
        let notes = vec![(480u64, 60u8, 480u64)];
        let fm = ChannelAssignment::Fm(0);
        let events = encode_channel_events(&notes, 960, &params, &fm).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Rest { .. })));
    }

    #[test]
    fn test_long_note_splits_with_tie() {
        let params = SmpsTempoParams { divider: 1, modifier: 128, daw_ticks_per_smps_tick: 1.0 };
        let notes = vec![(0u64, 60u8, 200u64)];
        let fm = ChannelAssignment::Fm(0);
        let events = encode_channel_events(&notes, 200, &params, &fm).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Tie)));
    }

    #[test]
    fn test_build_voice_index_deduplicates() {
        use crate::model::instrument::*;

        let inst_id = Uuid::new_v4();
        let tracks = vec![
            Track {
                id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
                instrument_id: Some(inst_id), regions: vec![], muted: false, solo: false,
                volume: 100, pan: Pan::Center, pitch_offset: 0, modulation: None,
            },
            Track {
                id: Uuid::new_v4(), name: "FM2".into(), channel: ChannelAssignment::Fm(1),
                instrument_id: Some(inst_id), regions: vec![], muted: false, solo: false,
                volume: 100, pan: Pan::Center, pitch_offset: 0, modulation: None,
            },
        ];
        let bank = InstrumentBank {
            fm: vec![FmInstrument {
                id: inst_id, name: "Test".into(), algorithm: 0, feedback: 0,
                operators: [FmOperator::default(); 4],
                metadata: InstrumentMetadata::default(),
            }],
            psg: vec![], dac: vec![],
        };
        let (map, voices) = build_voice_index(&tracks, &bank);
        assert_eq!(voices.len(), 1);
        assert_eq!(map[&inst_id], 0);
    }

    #[test]
    fn test_muted_tracks_excluded_from_voice_index() {
        use crate::model::instrument::*;

        let inst_id = Uuid::new_v4();
        let tracks = vec![Track {
            id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
            instrument_id: Some(inst_id), regions: vec![], muted: true, solo: false,
            volume: 100, pan: Pan::Center, pitch_offset: 0, modulation: None,
        }];
        let bank = InstrumentBank {
            fm: vec![FmInstrument {
                id: inst_id, name: "Test".into(), algorithm: 0, feedback: 0,
                operators: [FmOperator::default(); 4],
                metadata: InstrumentMetadata::default(),
            }],
            psg: vec![], dac: vec![],
        };
        let (map, voices) = build_voice_index(&tracks, &bank);
        assert!(voices.is_empty());
        assert!(map.is_empty());
    }

    #[test]
    fn test_validate_missing_instrument() {
        use crate::model::song::*;
        use crate::model::instrument::*;

        let song = Song {
            metadata: SongMetadata {
                name: "Test".into(), tempo: 120.0, time_signature: (4, 4),
                ticks_per_beat: 480, driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: Uuid::new_v4(), name: "FM1".into(), channel: ChannelAssignment::Fm(0),
                instrument_id: None, regions: vec![], muted: false, solo: false,
                volume: 100, pan: Pan::Center, pitch_offset: 0, modulation: None,
            }],
            instruments: InstrumentBank { fm: vec![], psg: vec![], dac: vec![] },
        };
        let params = compute_tempo_params(120.0, 480);
        let errors = validate_for_export(&song, &song.instruments, &crate::driver::FlamedriverProfile, &params);
        assert!(!errors.is_empty());
        assert!(errors[0].message.contains("No instrument assigned"));
    }

    #[test]
    fn test_validate_pitch_out_of_range() {
        use crate::model::song::*;
        use crate::model::instrument::*;

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
                    notes: vec![Note { tick: 0, pitch: 5, velocity: 100, duration_ticks: 480, instrument_id: None, detune: 0, pan_override: None, modulation: None }],
                    instrument_id: None,
                }],
                muted: false, solo: false, volume: 100, pan: Pan::Center, pitch_offset: 0, modulation: None,
            }],
            instruments: InstrumentBank {
                fm: vec![FmInstrument {
                    id: inst_id, name: "Test".into(), algorithm: 0, feedback: 0,
                    operators: [FmOperator::default(); 4],
                    metadata: InstrumentMetadata::default(),
                }],
                psg: vec![], dac: vec![],
            },
        };
        let params = compute_tempo_params(120.0, 480);
        let errors = validate_for_export(&song, &song.instruments, &crate::driver::FlamedriverProfile, &params);
        assert!(errors.iter().any(|e| e.message.contains("outside SMPS range")));
    }

    #[test]
    fn test_generate_music_asm_basic() {
        use crate::model::song::*;
        use crate::model::instrument::*;

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
                        Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 480, instrument_id: Some(inst_id), detune: 0, pan_override: None, modulation: None },
                        Note { tick: 480, pitch: 64, velocity: 100, duration_ticks: 480, instrument_id: Some(inst_id), detune: 0, pan_override: None, modulation: None },
                    ],
                    instrument_id: None,
                }],
                muted: false, solo: false, volume: 100, pan: Pan::Center, pitch_offset: 0, modulation: None,
            }],
            instruments: InstrumentBank {
                fm: vec![FmInstrument {
                    id: inst_id, name: "TestPatch".into(), algorithm: 4, feedback: 7,
                    operators: [FmOperator::default(); 4],
                    metadata: InstrumentMetadata::default(),
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
        assert!(asm.contains("nC4"));
        assert!(asm.contains("nE4"));
        assert!(asm.contains("smpsStop"));
    }

    // ---- audit F30: a channel the driver does not have ----

    /// A song with one track per requested channel, each carrying a single
    /// note, plus a bank holding an instrument of every kind so that no test
    /// below trips the unrelated "No instrument assigned" path. `with_notes`
    /// false makes every lane empty, which is how a leftover lane looks.
    fn f30_song_inner(channels: &[ChannelAssignment], with_notes: bool) -> Song {
        use crate::model::song::*;
        use crate::model::instrument::*;
        let fm_id = Uuid::new_v4();
        let psg_id = Uuid::new_v4();
        let dac_id = Uuid::new_v4();
        let tracks: Vec<Track> = channels.iter().enumerate().map(|(i, ch)| {
            let inst = match ch {
                ChannelAssignment::Dac(_) => dac_id,
                ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise => psg_id,
                ChannelAssignment::Fm(_) => fm_id,
            };
            let notes = if with_notes {
                vec![Note { tick: 0, pitch: 60, velocity: 100, duration_ticks: 480, instrument_id: Some(inst), detune: 0, pan_override: None, modulation: None }]
            } else {
                vec![]
            };
            Track {
                id: Uuid::new_v4(),
                name: format!("T{i}"),
                channel: ch.clone(),
                instrument_id: Some(inst),
                regions: vec![Region {
                    id: Uuid::new_v4(), start_tick: 0, duration_ticks: 480,
                    notes,
                    instrument_id: None,
                }],
                muted: false, solo: false, volume: 100, pan: Pan::Center,
                pitch_offset: 0, modulation: None,
            }
        }).collect();
        Song {
            metadata: SongMetadata {
                name: "F30".into(), tempo: 120.0, time_signature: (4, 4),
                ticks_per_beat: 480, driver_id: "flamedriver".into(),
            },
            tracks,
            instruments: InstrumentBank {
                fm: vec![FmInstrument {
                    id: fm_id, name: "P".into(), algorithm: 0, feedback: 0,
                    operators: [FmOperator::default(); 4],
                    metadata: InstrumentMetadata::default(),
                }],
                psg: vec![PsgInstrument {
                    id: psg_id, name: "S".into(), volume_sequence: vec![0],
                    loop_point: None, silence_on_end: false, noise_mode: None,
                    smps_envelope_index: None,
                    metadata: InstrumentMetadata::default(),
                }],
                dac: vec![DacInstrument {
                    id: dac_id, name: "D".into(), target_sample_rate: 16000,
                    loop_start: None, loop_length: None,
                    original_file: String::new(), pcm_file: "d.pcm".into(),
                    source_is_raw: false, metadata: InstrumentMetadata::default(),
                }],
            },
        }
    }

    fn f30_song(channels: &[ChannelAssignment]) -> Song {
        f30_song_inner(channels, true)
    }

    fn f30_errors(channels: &[ChannelAssignment]) -> Vec<String> {
        let song = f30_song(channels);
        let params = compute_tempo_params(120.0, 480);
        validate_for_export(&song, &song.instruments, &crate::driver::FlamedriverProfile, &params)
            .into_iter()
            .map(|e| format!("{}: {}", e.track_name, e.message))
            .collect()
    }

    /// The booked defect, alone. `Fm(5)` is the sixth FM voice; this driver has
    /// no sixth FM music voice (`FlamedriverProfile::channel_layout` offers
    /// FM1..FM5, derived in F31 from
    /// `skdisasm/Sound/Z80 Sound Driver.asm:1907` "FM6 music track (does not
    /// exist in this driver)" and the six-slot track RAM at 177-182). Before
    /// the fix this exported `smpsHeaderChan $01, $00` /
    /// `smpsHeaderFM Snd_F30_FM6, ...` and reported success.
    #[test]
    fn an_fm_voice_the_driver_does_not_have_is_refused_by_name() {
        let msgs = f30_errors(&[ChannelAssignment::Fm(5)]);
        assert!(
            msgs.iter().any(|m| m.starts_with("T0: ")
                && m.contains("Channel FM6 does not exist")),
            "an Fm(5) track must be refused, naming the track and the channel, got {msgs:?}",
        );
    }

    /// The same defect with a DAC track present, which is the case the audit
    /// item names: the DAC and FM6 are one hardware slot
    /// (`zSongFM6_DAC`, driver 177-182), so the header claims two music voices
    /// where the driver has one slot to give.
    #[test]
    fn an_fm6_track_alongside_a_dac_track_is_refused() {
        let msgs = f30_errors(&[ChannelAssignment::Dac(0), ChannelAssignment::Fm(5)]);
        assert_eq!(
            msgs.iter().filter(|m| m.contains("does not exist")).count(), 1,
            "exactly the FM6 track is refused, not the DAC track, got {msgs:?}",
        );
        assert!(
            msgs.iter().any(|m| m.starts_with("T1: ") && m.contains("Channel FM6")),
            "the refusal must name the FM6 track (T1), got {msgs:?}",
        );
    }

    /// The refusal must reach the real export entry point, not just the
    /// validator, and it must leave nothing behind on disk.
    #[test]
    fn write_export_refuses_a_song_using_a_voice_the_driver_lacks() {
        let song = f30_song(&[ChannelAssignment::Fm(5)]);
        let out = tempfile::tempdir().unwrap();
        let err = write_export(
            &song, &song.instruments, &crate::driver::FlamedriverProfile,
            out.path(), None,
        ).unwrap_err();
        assert!(
            err.iter().any(|e| e.track_name == "T0" && e.message.contains("Channel FM6 does not exist")),
            "write_export must refuse and say which track, got {:?}",
            err.iter().map(|e| format!("{}: {}", e.track_name, e.message)).collect::<Vec<_>>(),
        );
        let written: Vec<_> = fs::read_dir(out.path()).unwrap().collect();
        assert!(written.is_empty(), "a refused export must write no files");
    }

    /// Control. An ordinary S3K-shaped song -- 1 DAC + 5 FM + 3 PSG + noise --
    /// must still export, and its header must still be the shape real S3K
    /// songs use. Without this, the three assertions above could be satisfied
    /// by a check that refuses everything.
    ///
    /// The expected `$06` is derived, not copied: the driver reads header byte
    /// 2 as `b = number of FM + DAC channels`
    /// (`skdisasm/Sound/Z80 Sound Driver.asm:1839`) and 59 of the 60 files in
    /// `skdisasm/Sound/Music/` declare `smpsHeaderChan $06, $03` over exactly
    /// one `smpsHeaderDAC` and five `smpsHeaderFM` (e.g. `MGZ1.asm:4-15`).
    /// Here PSG is 4 because this driver's layout counts the noise lane
    /// separately from PSG1..PSG3.
    #[test]
    fn an_ordinary_song_on_every_channel_the_driver_has_still_exports() {
        let all = [
            ChannelAssignment::Dac(0),
            ChannelAssignment::Fm(0), ChannelAssignment::Fm(1), ChannelAssignment::Fm(2),
            ChannelAssignment::Fm(3), ChannelAssignment::Fm(4),
            ChannelAssignment::Psg(0), ChannelAssignment::Psg(1), ChannelAssignment::Psg(2),
            ChannelAssignment::PsgNoise,
        ];
        let msgs = f30_errors(&all);
        assert!(msgs.is_empty(), "a song using only real channels must raise nothing, got {msgs:?}");

        let song = f30_song(&all);
        let params = compute_tempo_params(120.0, 480);
        let (voice_map, _v) = build_voice_index(&song.tracks, &song.instruments);
        let asm = generate_music_asm(&song, &song.instruments, &voice_map, &params).unwrap();
        assert!(
            asm.contains("smpsHeaderChan\t\t$06, $04"),
            "1 DAC + 5 FM must be counted together as $06, with 4 PSG lanes; header was:\n{}",
            asm.lines().take(10).collect::<Vec<_>>().join("\n"),
        );
    }

    /// Control, and the reason the check is gated on "has notes". Every
    /// project saved before F31 carries an empty FM6 lane, because lanes were
    /// seeded from the layout that then offered six FM voices. An empty lane
    /// contributes no header entry (`generate_music_asm` filters on
    /// `!muted && ...any(|r| !r.notes.is_empty())`), so it is not the defect
    /// and must not block the export.
    #[test]
    fn an_empty_leftover_fm6_lane_does_not_block_the_export() {
        let song = f30_song_inner(
            &[ChannelAssignment::Fm(0), ChannelAssignment::Fm(5)],
            false,
        );
        let params = compute_tempo_params(120.0, 480);
        let msgs: Vec<String> = validate_for_export(
            &song, &song.instruments, &crate::driver::FlamedriverProfile, &params,
        ).into_iter().map(|e| e.message).collect();
        assert!(
            !msgs.iter().any(|m| m.contains("does not exist")),
            "an empty FM6 lane emits no header entry and must not be refused, got {msgs:?}",
        );
    }

    /// The check must come from the driver profile, never from the literal
    /// index 5. Asserted over every driver `default_registry()` actually
    /// registers (the single registration site F34 extracted), so a profile
    /// added later is covered without this test being edited:
    ///
    ///   * every channel a profile's own layout offers must validate clean, and
    ///   * one FM index past the end of that profile's own FM list must be
    ///     refused -- for Flamedriver that index happens to be 5, but the test
    ///     never says so.
    #[test]
    fn channel_validity_is_read_from_each_registered_driver_not_a_fixed_index() {
        let registry = crate::driver::default_registry();
        let profiles: Vec<&dyn DriverProfile> = registry.profiles().collect();
        assert!(
            !profiles.is_empty(),
            "the registry must hold at least one driver, or this guard checks nothing",
        );
        let params = compute_tempo_params(120.0, 480);

        for driver in &profiles {
            let layout = driver.channel_layout();

            let every_real_channel: Vec<ChannelAssignment> =
                layout.dac_channels.iter().map(|c| ChannelAssignment::Dac(c.index))
                    .chain(layout.fm_channels.iter().map(|c| ChannelAssignment::Fm(c.index)))
                    .chain(layout.psg_channels.iter().map(|c| if c.is_noise {
                        ChannelAssignment::PsgNoise
                    } else {
                        ChannelAssignment::Psg(c.index)
                    }))
                    .collect();
            let song = f30_song(&every_real_channel);
            let refused: Vec<String> = validate_for_export(&song, &song.instruments, *driver, &params)
                .into_iter()
                .filter(|e| e.message.contains("does not exist"))
                .map(|e| e.message)
                .collect();
            assert!(
                refused.is_empty(),
                "driver `{}` refused a channel its own layout advertises: {refused:?}",
                driver.id(),
            );

            let past_end = layout.fm_channels.iter().map(|c| c.index).max().map(|m| m + 1).unwrap_or(0);
            let song = f30_song(&[ChannelAssignment::Fm(past_end)]);
            let msgs: Vec<String> = validate_for_export(&song, &song.instruments, *driver, &params)
                .into_iter().map(|e| e.message).collect();
            assert!(
                msgs.iter().any(|m| m.contains("does not exist")),
                "driver `{}` has {} FM voices, so FM index {past_end} is not one of them \
                 and must be refused, got {msgs:?}",
                driver.id(),
                layout.fm_channels.len(),
            );
        }
    }

    // ---- audit F37: the DAC slot is filled positionally ----

    /// Build a song on `channels` and return its exported music ASM.
    /// Reuses the F30 builder, which puts one note and a valid instrument on
    /// every requested lane, so nothing here trips an unrelated validator.
    fn f37_asm(channels: &[ChannelAssignment]) -> String {
        let mut song = f30_song(channels);
        song.metadata.name = "F37".into();
        let params = compute_tempo_params(120.0, 480);
        let (voice_map, _v) = build_voice_index(&song.tracks, &song.instruments);
        generate_music_asm(&song, &song.instruments, &voice_map, &params).unwrap()
    }

    /// The channel entries of an exported header, in the order the driver will
    /// copy them into its track slots, as `(kind, operand)` pairs.
    ///
    /// Order is the whole point: `zBGMLoad` copies entry N into track slot N
    /// (`skdisasm/Sound/Z80 Sound Driver.asm:1836-1857`), so reading them as a
    /// set would not be able to see this defect at all.
    fn header_channel_entries(asm: &str) -> Vec<(String, String)> {
        asm.lines()
            .filter_map(|l| {
                let t = l.trim();
                for kind in ["DAC", "FM", "PSG"] {
                    if let Some(rest) = t.strip_prefix(&format!("smpsHeader{kind}")) {
                        return Some((kind.to_string(), rest.trim().to_string()));
                    }
                }
                None
            })
            .collect()
    }

    /// The booked defect. A song with no drum track and three FM instruments
    /// used to export three channel entries, the first of which was FM1.
    ///
    /// Derived, not copied: `zBGMLoad` fills the FM/DAC track slots
    /// POSITIONALLY -- `ld b, (iy+2)` (the FM+DAC count) then `.fm_dac_loop`
    /// copying each header entry into `zTracksStart` onward
    /// (`skdisasm/Sound/Z80 Sound Driver.asm:1836-1857`) -- and the init bytes
    /// it pairs them with are fixed:
    ///
    ///     zFMDACInitBytes:
    ///             db   80h,   6      ; <- slot 0
    ///             db   80h,   0
    ///             ...
    ///
    /// whose own comment reads "The first is for DAC; then 0, 1, 2 then 4, 5,
    /// 6 for the FM channels" (`ibid.:1893-1906`). Slot 0 is `zSongFM6_DAC`
    /// (`ibid.:176-183`) and is driven unconditionally through
    /// `zUpdateDACTrack` (`ibid.:717-719`), which reads its data bytes as
    /// SAMPLE ids, not notes. So the old first entry, FM1, played as drums.
    ///
    /// After the fix the first entry must be a DAC entry, and the count must
    /// include it: 3 FM + 1 DAC = `$04`.
    #[test]
    fn a_drumless_song_does_not_put_its_first_instrument_on_the_drum_channel() {
        let asm = f37_asm(&[
            ChannelAssignment::Fm(0), ChannelAssignment::Fm(1), ChannelAssignment::Fm(2),
        ]);
        let entries = header_channel_entries(&asm);
        assert_eq!(
            entries.first().map(|(k, _)| k.as_str()),
            Some("DAC"),
            "track slot 0 is the DAC slot and is always driven as drums, so the FIRST \
             channel entry must be a DAC entry; entries were {entries:?}",
        );
        assert!(
            asm.contains("smpsHeaderChan\t\t$04, $00"),
            "the synthesized DAC entry must be counted: 3 FM + 1 DAC = $04; header was:\n{}",
            asm.lines().take(8).collect::<Vec<_>>().join("\n"),
        );
    }

    /// The other direction, and the reason this cannot be an unconditional
    /// prepend. A song that already carries drums must NOT gain a second DAC
    /// entry: two would shift every FM entry down one slot, so FM1's data
    /// would land in FM2's slot and the last FM entry would run off the end of
    /// the six FM/DAC slots into `zSongPSG1`
    /// (`skdisasm/Sound/Z80 Sound Driver.asm:176-184`).
    #[test]
    fn a_song_that_already_has_drums_does_not_get_a_second_drum_entry() {
        let asm = f37_asm(&[
            ChannelAssignment::Dac(0), ChannelAssignment::Fm(0), ChannelAssignment::Fm(1),
        ]);
        let entries = header_channel_entries(&asm);
        let dacs: Vec<_> = entries.iter().filter(|(k, _)| k == "DAC").collect();
        assert_eq!(
            dacs.len(), 1,
            "a song with its own drum track already fills slot 0; a second DAC entry \
             would push every FM entry one slot down. Entries were {entries:?}",
        );
        assert_eq!(
            dacs[0].1, "Snd_F37_DAC1, $00, $00",
            "the surviving entry must be the song's OWN drum track, not a synthesized \
             silent one; entries were {entries:?}",
        );
        assert!(
            asm.contains("smpsHeaderChan\t\t$03, $00"),
            "1 DAC + 2 FM = $03, unchanged by this parcel; header was:\n{}",
            asm.lines().take(8).collect::<Vec<_>>().join("\n"),
        );
    }

    /// Control: the synthesized entry must point at a track the driver can
    /// actually run, and its body is taken from a shipped song rather than
    /// invented.
    ///
    /// Exactly one of the 60 songs in `skdisasm/Sound/Music/` is drumless, and
    /// it is the shipped precedent for this entry: `Chaos Emerald.asm`
    /// declares `smpsHeaderDAC Snd_Emerald_DAC` at line 7 like every other
    /// song, and its track body (lines 74-78) is
    ///
    ///     ; DAC Data
    ///     Snd_Emerald_DAC:
    ///     ; PSG3 Data
    ///     Snd_Emerald_PSG3:
    ///             smpsStop
    ///
    /// -- a bare label falling straight into a single `smpsStop`. That is one
    /// byte, `$F2` (`skdisasm/Sound/_smps2asm_inc.asm:580-582`), and the
    /// driver's handler for it, `cfStopTrack`, does
    /// `res 7, (ix+zTrack.PlaybackControl)`
    /// (`skdisasm/Sound/Z80 Sound Driver.asm:3443-3444`), which is exactly the
    /// bit `zUpdateMusic` tests before calling `zUpdateDACTrack`
    /// (`ibid.:717-719`). So the slot is entered once, stops, and is never
    /// updated again: no hang, and no sample ever played. A body that rested
    /// forever or looped on a rest would instead keep the slot alive for the
    /// whole song; the shipped song does not do that, so neither does this.
    ///
    /// The label must also be DEFINED, not merely referenced -- `smpsHeaderDAC`
    /// runs its operand through `CheckedChannelPointer`
    /// (`_smps2asm_inc.asm:317-318`), so a dangling label is an assembly
    /// failure, i.e. an export that cannot be built at all.
    #[test]
    fn the_synthesized_drum_track_is_a_lone_smps_stop_and_its_label_is_defined() {
        let asm = f37_asm(&[ChannelAssignment::Fm(0)]);
        let entries = header_channel_entries(&asm);
        let (_, operand) = entries.first().expect("a drumless song must still emit entries");
        let label = operand.split(',').next().unwrap().trim().to_string();

        let body_start = asm
            .lines()
            .position(|l| l.trim_end() == format!("{label}:"))
            .unwrap_or_else(|| panic!(
                "header references `{label}` but no such label is defined; the export \
                 would not assemble. ASM was:\n{asm}"
            ));
        let body: Vec<&str> = asm.lines().skip(body_start + 1)
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with(';'))
            .take_while(|l| !l.ends_with(':'))
            .collect();
        assert_eq!(
            body, vec!["smpsStop"],
            "the silent DAC track is one `smpsStop`, as in `Chaos Emerald.asm:74-78`",
        );
    }

    /// Control, and the ruling's uniformity clause made checkable. A drumless
    /// song with NO FM tracks at all still gets the entry: one code path, not
    /// a conditional. The audible result is unchanged for such a song -- there
    /// is no first FM instrument to misroute -- but the header still describes
    /// the format's slot 0 correctly.
    #[test]
    fn a_drumless_song_with_no_fm_tracks_still_gets_the_entry() {
        let asm = f37_asm(&[ChannelAssignment::Psg(0), ChannelAssignment::Psg(1)]);
        let entries = header_channel_entries(&asm);
        assert_eq!(
            entries.first().map(|(k, _)| k.as_str()),
            Some("DAC"),
            "entries were {entries:?}",
        );
        assert!(
            asm.contains("smpsHeaderChan\t\t$01, $02"),
            "0 FM + 1 synthesized DAC = $01, and 2 PSG; header was:\n{}",
            asm.lines().take(8).collect::<Vec<_>>().join("\n"),
        );
    }

    /// Control: a song that exports correctly today must be untouched. Same
    /// shape as the F30 control, but asserts the full entry ORDER, which is
    /// what the driver actually reads.
    #[test]
    fn a_song_with_drums_keeps_its_exact_header_entry_order() {
        let asm = f37_asm(&[
            ChannelAssignment::Dac(0),
            ChannelAssignment::Fm(0), ChannelAssignment::Fm(1), ChannelAssignment::Fm(2),
            ChannelAssignment::Fm(3), ChannelAssignment::Fm(4),
            ChannelAssignment::Psg(0), ChannelAssignment::Psg(1), ChannelAssignment::Psg(2),
        ]);
        let kinds: Vec<String> = header_channel_entries(&asm)
            .into_iter().map(|(k, _)| k).collect();
        assert_eq!(
            kinds,
            vec!["DAC", "FM", "FM", "FM", "FM", "FM", "PSG", "PSG", "PSG"],
            "the S3K header shape -- 1 DAC then 5 FM then 3 PSG, as in \
             `skdisasm/Sound/Music/MGZ1.asm:4-15` -- must be unchanged",
        );
        assert!(
            asm.contains("smpsHeaderChan\t\t$06, $03"),
            "header was:\n{}",
            asm.lines().take(8).collect::<Vec<_>>().join("\n"),
        );
    }
}
