pub struct PsgEnvelopeEntry {
    pub volumes: &'static [i8],
    pub loop_point: Option<usize>,
    pub silence_on_end: bool,
}

pub fn get_envelope(index: u8) -> Option<&'static PsgEnvelopeEntry> {
    FLAMEDRIVER_PSG_ENVELOPES.get(index as usize)
}

pub const FLAMEDRIVER_PSG_ENVELOPES: &[PsgEnvelopeEntry] = &[
    // $00: VolEnv_00 — StopTrack
    PsgEnvelopeEntry { volumes: &[2], loop_point: None, silence_on_end: true },
    // $01: VolEnv_01 (also VolEnv_0E) — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x10], loop_point: None, silence_on_end: true },
    // $02: VolEnv_02 — RestTrack
    PsgEnvelopeEntry { volumes: &[2, 1, 0, 0, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5], loop_point: None, silence_on_end: false },
    // $03: VolEnv_03 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 2, 3, 4, 4, 5, 5, 5, 6, 6], loop_point: None, silence_on_end: false },
    // $04: VolEnv_04 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 0, 1, 1, 1, 2, 3, 4, 4, 5], loop_point: None, silence_on_end: false },
    // $05: VolEnv_05 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 2, 3, 4, 5, 5, 6, 8, 7, 7, 6], loop_point: None, silence_on_end: false },
    // $06: VolEnv_06 — Reset (loop)
    PsgEnvelopeEntry { volumes: &[1, 0x0C, 3, 0x0F, 2, 7, 3, 0x0F], loop_point: Some(0), silence_on_end: false },
    // $07: VolEnv_07 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 2, 3, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0E, 0x0F], loop_point: None, silence_on_end: true },
    // $08: VolEnv_08 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 2, 1, 1, 0, 0, 1, 2, 3, 4], loop_point: None, silence_on_end: false },
    // $09: VolEnv_09 — RestTrack
    PsgEnvelopeEntry { volumes: &[1, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5], loop_point: None, silence_on_end: false },
    // $0A: VolEnv_0A — Reset (loop)
    PsgEnvelopeEntry { volumes: &[0x10, 0x20, 0x30, 0x40, 0x30, 0x20, 0x10, 0, -0x10], loop_point: Some(0), silence_on_end: false },
    // $0B: VolEnv_0B — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 3, 3, 4, 5], loop_point: None, silence_on_end: true },
    // $0C: VolEnv_0C — RestTrack
    PsgEnvelopeEntry { volumes: &[0], loop_point: None, silence_on_end: false },
    // $0D: VolEnv_0D — StopTrack
    PsgEnvelopeEntry { volumes: &[2], loop_point: None, silence_on_end: true },
    // $0E: VolEnv_0E — StopTrack (same data as $01)
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x10], loop_point: None, silence_on_end: true },
    // $0F: VolEnv_0F — RestTrack
    PsgEnvelopeEntry { volumes: &[9, 9, 9, 8, 8, 8, 7, 7, 7, 6, 6, 6, 5, 5, 5, 4, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1, 0, 0, 0], loop_point: None, silence_on_end: false },
    // $10: VolEnv_10 — RestTrack
    PsgEnvelopeEntry { volumes: &[1, 1, 1, 0, 0, 0], loop_point: None, silence_on_end: false },
    // $11: VolEnv_11 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 0, 1, 1, 1, 2, 3, 4, 4, 5], loop_point: None, silence_on_end: false },
    // $12: VolEnv_12 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 2, 3, 4, 5, 5, 6, 8, 7, 7, 6], loop_point: None, silence_on_end: false },
    // $13: VolEnv_13 — StopTrack
    PsgEnvelopeEntry { volumes: &[0x0A, 5, 0, 4, 8], loop_point: None, silence_on_end: true },
    // $14: VolEnv_14 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 2, 3, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0E, 0x0F], loop_point: None, silence_on_end: true },
    // $15: VolEnv_15 — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 2, 1, 1, 0, 0, 1, 2, 3, 4], loop_point: None, silence_on_end: false },
    // $16: VolEnv_16 — RestTrack
    PsgEnvelopeEntry { volumes: &[1, 0, 0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 5, 5], loop_point: None, silence_on_end: false },
    // $17: VolEnv_17 — Reset (loop)
    PsgEnvelopeEntry { volumes: &[0x10, 0x20, 0x30, 0x40, 0x30, 0x20, 0x10, 0], loop_point: Some(0), silence_on_end: false },
    // $18: VolEnv_18 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 3, 3, 4, 5], loop_point: None, silence_on_end: true },
    // $19: VolEnv_19 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x16], loop_point: None, silence_on_end: true },
    // $1A: VolEnv_1A — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 3, 3, 4, 5], loop_point: None, silence_on_end: true },
    // $1B: VolEnv_1B — StopTrack
    PsgEnvelopeEntry { volumes: &[4, 4, 4, 4, 3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1], loop_point: None, silence_on_end: true },
    // $1C: VolEnv_1C — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 6, 6, 6, 6, 7, 7, 7, 7, 8, 8, 8, 8, 9, 9, 9, 9, 0x0A, 0x0A, 0x0A, 0x0A], loop_point: None, silence_on_end: false },
    // $1D: VolEnv_1D — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0x0A], loop_point: None, silence_on_end: true },
    // $1E: VolEnv_1E — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 4], loop_point: None, silence_on_end: false },
    // $1F: VolEnv_1F — RestTrack
    PsgEnvelopeEntry { volumes: &[0x30, 0x20, 0x10, 0, 0, 0, 0, 0, 8, 0x10, 0x20, 0x30], loop_point: None, silence_on_end: false },
    // $20: VolEnv_20 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 6, 6, 6, 8, 8, 0x0A], loop_point: None, silence_on_end: true },
    // $21: VolEnv_21 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 3, 4, 6, 7], loop_point: None, silence_on_end: false },
    // $22: VolEnv_22 — RestTrack
    PsgEnvelopeEntry { volumes: &[2, 1, 0, 0, 0, 2, 4, 7], loop_point: None, silence_on_end: false },
    // $23: VolEnv_23 — StopTrack
    PsgEnvelopeEntry { volumes: &[0x0F, 1, 5], loop_point: None, silence_on_end: true },
    // $24: VolEnv_24 — StopTrack
    PsgEnvelopeEntry { volumes: &[8, 6, 2, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10], loop_point: None, silence_on_end: true },
    // $25: VolEnv_25 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9, 9, 9], loop_point: None, silence_on_end: true },
    // $26: VolEnv_26 — StopTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5], loop_point: None, silence_on_end: true },
    // $27: VolEnv_27 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 1, 1, 1, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 6, 6, 7], loop_point: None, silence_on_end: false },
    // $28: VolEnv_28 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 2, 4, 6, 8, 0x10], loop_point: None, silence_on_end: false },
    // $29: VolEnv_29 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7], loop_point: None, silence_on_end: false },
    // $2A: VolEnv_2A — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 2, 3, 4, 4, 5, 5, 5, 6], loop_point: None, silence_on_end: false },
    // $2B: VolEnv_2B — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 4], loop_point: None, silence_on_end: false },
    // $2C: VolEnv_2C — RestTrack
    PsgEnvelopeEntry { volumes: &[3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 0, 0, 0, 0], loop_point: None, silence_on_end: false },
    // $2D: VolEnv_2D — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 4, 4, 4, 5, 5, 5, 6, 7], loop_point: None, silence_on_end: false },
    // $2E: VolEnv_2E — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 7, 7, 7], loop_point: None, silence_on_end: false },
    // $2F: VolEnv_2F — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F], loop_point: None, silence_on_end: false },
    // $30: VolEnv_30 — RestTrack
    PsgEnvelopeEntry { volumes: &[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4], loop_point: None, silence_on_end: false },
    // $31: VolEnv_31 — RestTrack
    PsgEnvelopeEntry { volumes: &[4, 4, 4, 3, 3, 3, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 4], loop_point: None, silence_on_end: false },
    // $32: VolEnv_32 — RestTrack
    PsgEnvelopeEntry { volumes: &[4, 4, 3, 3, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 7], loop_point: None, silence_on_end: false },
    // $33: VolEnv_33 — RestTrack
    PsgEnvelopeEntry { volumes: &[0x0E, 0x0D, 0x0C, 0x0B, 0x0A, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0], loop_point: None, silence_on_end: false },
];

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
    fn test_envelope_0e_matches_01() {
        let e01 = get_envelope(1).unwrap();
        let e0e = get_envelope(0x0E).unwrap();
        assert_eq!(e01.volumes, e0e.volumes);
    }

    #[test]
    fn test_out_of_range_returns_none() {
        assert!(get_envelope(52).is_none());
        assert!(get_envelope(255).is_none());
    }

    #[test]
    fn test_stop_track_envelopes_silence() {
        let env_0b = get_envelope(0x0B).unwrap();
        assert!(env_0b.silence_on_end);
    }

    #[test]
    fn test_rest_track_envelopes_sustain() {
        let env_0c = get_envelope(0x0C).unwrap();
        assert!(!env_0c.silence_on_end);
    }
}
