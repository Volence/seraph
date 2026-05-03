const FM_FNUM_TABLE: [u16; 12] = [
    644,  // C
    683,  // C#
    723,  // D
    766,  // D#
    813,  // E
    860,  // F
    911,  // F#
    965,  // G
    1023, // G#
    1084, // A
    1148, // A#
    1216, // B
];

pub fn midi_to_fm_freq(midi_note: u8) -> (u8, u16) {
    let semitone = (midi_note % 12) as usize;
    let octave = midi_note / 12;
    let block = octave.saturating_sub(1).min(7);
    (block, FM_FNUM_TABLE[semitone])
}

pub fn midi_to_psg_period(midi_note: u8) -> u16 {
    let freq = 440.0 * 2.0_f64.powf((midi_note as f64 - 69.0) / 12.0);
    let period = (3_579_545.0 / (32.0 * freq)) as u16;
    period.min(1023)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_60_is_block_4() {
        let (block, _) = midi_to_fm_freq(60);
        assert_eq!(block, 4);
    }

    #[test]
    fn test_midi_48_is_block_3() {
        let (block, _) = midi_to_fm_freq(48);
        assert_eq!(block, 3);
    }

    #[test]
    fn test_midi_72_is_block_5() {
        let (block, _) = midi_to_fm_freq(72);
        assert_eq!(block, 5);
    }

    #[test]
    fn test_fm_a4_fnum_is_1084() {
        let (_, fnum) = midi_to_fm_freq(69);
        assert_eq!(fnum, 1084);
    }

    #[test]
    fn test_psg_a4_period_near_254() {
        let period = midi_to_psg_period(69);
        assert!((period as i32 - 254).abs() <= 1);
    }

    #[test]
    fn test_psg_high_note_clamps_to_1023() {
        let period = midi_to_psg_period(12);
        assert!(period <= 1023);
    }

    #[test]
    fn test_psg_period_decreases_with_pitch() {
        let p_low = midi_to_psg_period(48);
        let p_high = midi_to_psg_period(72);
        assert!(p_low > p_high);
    }
}
