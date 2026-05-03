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

// SMPS PSG period lookup — matches Flamedriver/S3K Z80 driver exactly.
// 84 entries for z80_index 0-83, mapped to MIDI 36-119 via (midi = z80_index + 36).
const PSG_PERIOD_TABLE: [u16; 84] = [
    0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3F7,0x3BE,0x388,
    0x356,0x326,0x2F9,0x2CE,0x2A5,0x280,0x25C,0x23A,0x21A,0x1FB,0x1DF,0x1C4,
    0x1AB,0x193,0x17D,0x167,0x153,0x140,0x12E,0x11D,0x10D,0x0FE,0x0EF,0x0E2,
    0x0D6,0x0C9,0x0BE,0x0B4,0x0A9,0x0A0,0x097,0x08F,0x087,0x07F,0x078,0x071,
    0x06B,0x065,0x05F,0x05A,0x055,0x050,0x04B,0x047,0x043,0x040,0x03C,0x039,
    0x036,0x033,0x030,0x02D,0x02B,0x028,0x026,0x024,0x022,0x020,0x01F,0x01D,
    0x01B,0x01A,0x018,0x017,0x016,0x015,0x013,0x012,0x011,0x010,0x000,0x000,
];

pub fn midi_to_psg_period(midi_note: u8) -> u16 {
    let idx = midi_note as i16 - 36;
    if idx >= 0 && idx < 84 {
        PSG_PERIOD_TABLE[idx as usize]
    } else if idx < 0 {
        1023
    } else {
        0
    }
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
    fn test_psg_a4_period_exact() {
        // MIDI 69 = A4, z80_index 33, table entry 0x0FE = 254
        assert_eq!(midi_to_psg_period(69), 0x0FE);
    }

    #[test]
    fn test_psg_below_table_clamps_to_1023() {
        assert_eq!(midi_to_psg_period(12), 1023);
        assert_eq!(midi_to_psg_period(35), 1023);
    }

    #[test]
    fn test_psg_table_boundaries() {
        assert_eq!(midi_to_psg_period(36), 0x3FF); // z80_index 0
        assert_eq!(midi_to_psg_period(119), 0x000); // z80_index 83
    }

    #[test]
    fn test_psg_period_decreases_with_pitch() {
        let p_low = midi_to_psg_period(48);
        let p_high = midi_to_psg_period(72);
        assert!(p_low > p_high);
    }
}
