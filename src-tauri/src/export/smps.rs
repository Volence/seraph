use std::collections::HashMap;
use uuid::Uuid;
use crate::driver::flamedriver::FlamedriverProfile;
use crate::export::ExportError;
use crate::model::driver::DriverProfile;
use crate::model::instrument::{FmInstrument, InstrumentBank};
use crate::model::song::{ChannelAssignment, Pan, Track};

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
            let smps_ticks_per_sec = (modifier as f64 / 256.0) * 60.0;
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

// --- Note Encoding ---

const SMPS_MAX_DURATION: u64 = 127;

/// MIDI pitch → SMPS note byte. Valid MIDI range: 12 (C0) through 95 (Bb7).
pub fn midi_to_smps_note(midi_pitch: u8) -> Option<u8> {
    if midi_pitch < 12 || midi_pitch > 95 {
        return None;
    }
    Some(0x81 + (midi_pitch - 12))
}

/// SMPS note name for assembly output.
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

/// A single event in the SMPS output stream.
#[derive(Debug, Clone)]
pub enum SmpsEvent {
    Note { pitch_name: String, duration: u64 },
    Rest { duration: u64 },
    Tie,
    SetVoice(u8),
    SetPan(u8),
    SetPsgVoice(u8),
    Stop,
}

/// Encode notes into SmpsEvents. Notes must be sorted by tick.
/// Gaps between notes become rests. Long durations split with ties.
pub fn encode_channel_events(
    notes: &[(u64, u8, u64)],
    region_duration: u64,
    params: &SmpsTempoParams,
) -> Result<Vec<SmpsEvent>, ExportError> {
    let mut out = Vec::new();
    let mut cursor: u64 = 0;

    for &(tick, pitch, dur_ticks) in notes {
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

/// Build a deduplicated voice index from active FM tracks.
pub fn build_voice_index<'a>(tracks: &[Track], instruments: &'a InstrumentBank) -> (HashMap<Uuid, u8>, Vec<&'a FmInstrument>) {
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
        assert_eq!(midi_to_smps_note(96), None);
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
        let events = encode_channel_events(&notes, 480, &params).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Note { .. })));
    }

    #[test]
    fn test_encode_gap_produces_rest() {
        let params = compute_tempo_params(120.0, 480);
        let notes = vec![(480u64, 60u8, 480u64)];
        let events = encode_channel_events(&notes, 960, &params).unwrap();
        assert!(events.iter().any(|e| matches!(e, SmpsEvent::Rest { .. })));
    }

    #[test]
    fn test_long_note_splits_with_tie() {
        let params = SmpsTempoParams { divider: 1, modifier: 128, daw_ticks_per_smps_tick: 1.0 };
        let notes = vec![(0u64, 60u8, 200u64)];
        let events = encode_channel_events(&notes, 200, &params).unwrap();
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
            volume: 100, pan: Pan::Center,
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
}
