use std::collections::HashMap;
use uuid::Uuid;
use crate::driver::flamedriver::FlamedriverProfile;
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

/// DAC sample byte → sample name for assembly output.
pub fn dac_sample_name(pitch: u8) -> String {
    use crate::import::smps_parser::build_dac_table;
    let table = build_dac_table();
    for (name, byte) in &table {
        if *byte == pitch {
            return name.clone();
        }
    }
    format!("${:02X}", pitch)
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
    Stop,
}

/// Encode notes into SmpsEvents. Notes must be sorted by tick.
/// Gaps between notes become rests. Long durations split with ties.
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

    let mut add_voice = |id: &Uuid, map: &mut HashMap<Uuid, u8>, voices: &mut Vec<&'a FmInstrument>| {
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
pub fn generate_voice_bank_asm(song_label: &str, voices: &[&FmInstrument], driver: &FlamedriverProfile) -> String {
    let mut asm = String::new();
    asm.push_str("; ============================================================\n");
    asm.push_str(&format!("; Voice Bank: {song_label}\n"));
    asm.push_str("; Exported from MegaDAW\n");
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
    asm.push_str("; Exported from MegaDAW\n");
    asm.push_str("; ============================================================\n\n");

    let active_tracks: Vec<&Track> = song.tracks.iter()
        .filter(|t| !t.muted && t.regions.iter().any(|r| !r.notes.is_empty()))
        .collect();
    let fm_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Fm(_))).count();
    let dac_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Dac(_))).count();
    let psg_count = active_tracks.iter().filter(|t| matches!(t.channel, ChannelAssignment::Psg(_) | ChannelAssignment::PsgNoise)).count();
    let fm_dac_count = fm_count + dac_count;

    asm.push_str(&format!("Snd_{label}_Header:\n"));
    asm.push_str("\tsmpsHeaderStartSong 3\n");
    asm.push_str(&format!("\tsmpsHeaderVoice\t\tSnd_{label}_Voices\n"));
    asm.push_str(&format!("\tsmpsHeaderChan\t\t${fm_dac_count:02X}, ${psg_count:02X}\n"));
    asm.push_str(&format!("\tsmpsHeaderTempo\t\t${:02X}, ${:02X}\n", params.divider, params.modifier));

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
pub fn validate_for_export(
    song: &Song,
    instruments: &InstrumentBank,
    params: &SmpsTempoParams,
) -> Vec<ExportError> {
    let mut errors = Vec::new();

    for track in &song.tracks {
        if track.muted { continue; }

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
                if !skip_pitch_check && midi_to_smps_note(note.pitch).is_none() {
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
    driver: &FlamedriverProfile,
    output_dir: &Path,
) -> Result<ExportResult, Vec<ExportError>> {
    let params = compute_tempo_params(song.metadata.tempo, song.metadata.ticks_per_beat);

    let errors = validate_for_export(song, instruments, &params);
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

#[cfg(test)]
mod tests {
    use super::*;

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
        let errors = validate_for_export(&song, &song.instruments, &params);
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
        let errors = validate_for_export(&song, &song.instruments, &params);
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
}
