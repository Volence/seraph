use crate::export::ExportError;

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
}
