use crate::model::driver::*;
use crate::model::instrument::*;
use uuid::Uuid;

pub struct FlamedriverProfile;

/// Bit i = 1 means operator (i+1) is a carrier for that algorithm.
const CARRIER_MASKS: [u8; 8] = [
    0b1000, // algo 0: op4
    0b1000, // algo 1: op4
    0b1000, // algo 2: op4
    0b1000, // algo 3: op4
    0b1010, // algo 4: op2, op4
    0b1110, // algo 5: op2, op3, op4
    0b1110, // algo 6: op2, op3, op4
    0b1111, // algo 7: all
];

const OP_ORDER: [usize; 4] = [3, 2, 1, 0]; // ops 4,3,2,1

impl DriverProfile for FlamedriverProfile {
    fn name(&self) -> &str {
        "Flamedriver (S3K)"
    }

    fn id(&self) -> &str {
        "flamedriver"
    }

    fn channel_layout(&self) -> ChannelLayout {
        ChannelLayout {
            fm_channels: vec![
                FmChannelInfo { index: 0, name: "FM1".into(), supports_special_mode: false },
                FmChannelInfo { index: 1, name: "FM2".into(), supports_special_mode: false },
                FmChannelInfo { index: 2, name: "FM3".into(), supports_special_mode: true },
                FmChannelInfo { index: 3, name: "FM4".into(), supports_special_mode: false },
                FmChannelInfo { index: 4, name: "FM5".into(), supports_special_mode: false },
                FmChannelInfo { index: 5, name: "FM6/DAC".into(), supports_special_mode: false },
            ],
            psg_channels: vec![
                PsgChannelInfo { index: 0, name: "PSG1".into(), is_noise: false },
                PsgChannelInfo { index: 1, name: "PSG2".into(), is_noise: false },
                PsgChannelInfo { index: 2, name: "PSG3".into(), is_noise: false },
                PsgChannelInfo { index: 3, name: "PSG Noise".into(), is_noise: true },
            ],
            dac_channels: vec![
                DacChannelInfo { index: 0, name: "DAC".into() },
            ],
        }
    }

    fn supports_feature(&self, feature: DriverFeature) -> bool {
        matches!(feature, DriverFeature::Fm3SpecialMode)
    }

    fn validate_fm(&self, inst: &FmInstrument) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if inst.algorithm > 7 {
            errors.push(format!("algorithm {} > 7", inst.algorithm));
        }
        if inst.feedback > 7 {
            errors.push(format!("feedback {} > 7", inst.feedback));
        }
        for (i, op) in inst.operators.iter().enumerate() {
            let n = i + 1;
            if op.detune > 7 { errors.push(format!("op{n} detune {} > 7", op.detune)); }
            if op.multiple > 15 { errors.push(format!("op{n} multiple {} > 15", op.multiple)); }
            if op.rate_scale > 3 { errors.push(format!("op{n} rate_scale {} > 3", op.rate_scale)); }
            if op.attack_rate > 31 { errors.push(format!("op{n} attack_rate {} > 31", op.attack_rate)); }
            if op.d1r > 31 { errors.push(format!("op{n} d1r {} > 31", op.d1r)); }
            if op.d2r > 31 { errors.push(format!("op{n} d2r {} > 31", op.d2r)); }
            if op.sustain_level > 15 { errors.push(format!("op{n} sustain_level {} > 15", op.sustain_level)); }
            if op.release_rate > 15 { errors.push(format!("op{n} release_rate {} > 15", op.release_rate)); }
            if op.total_level > 127 { errors.push(format!("op{n} total_level {} > 127", op.total_level)); }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    fn validate_psg(&self, inst: &PsgInstrument) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        if inst.volume_sequence.is_empty() {
            errors.push("volume_sequence is empty".into());
        }
        for (i, &v) in inst.volume_sequence.iter().enumerate() {
            if v > 15 {
                errors.push(format!("volume_sequence[{i}] = {v} > 15"));
            }
        }
        if let Some(lp) = inst.loop_point {
            if lp >= inst.volume_sequence.len() {
                errors.push(format!(
                    "loop_point {lp} >= sequence length {}",
                    inst.volume_sequence.len()
                ));
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    fn validate_dac(&self, inst: &DacInstrument) -> Result<(), Vec<String>> {
        let valid_rates = [8000, 11025, 16000, 22050, 32000];
        if valid_rates.contains(&inst.target_sample_rate) {
            Ok(())
        } else {
            Err(vec![format!(
                "target_sample_rate {} not in {:?}",
                inst.target_sample_rate, valid_rates
            )])
        }
    }

    fn fm_to_bytes(&self, inst: &FmInstrument) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(25);
        let carrier_mask = CARRIER_MASKS[inst.algorithm as usize];

        for &idx in &OP_ORDER {
            let op = &inst.operators[idx];
            bytes.push((op.detune << 4) | op.multiple);
        }
        for &idx in &OP_ORDER {
            let op = &inst.operators[idx];
            bytes.push((op.rate_scale << 6) | op.attack_rate);
        }
        for &idx in &OP_ORDER {
            let op = &inst.operators[idx];
            bytes.push(((op.amp_mod as u8) << 7) | op.d1r);
        }
        for &idx in &OP_ORDER {
            bytes.push(inst.operators[idx].d2r);
        }
        for &idx in &OP_ORDER {
            let op = &inst.operators[idx];
            bytes.push((op.sustain_level << 4) | op.release_rate);
        }
        for &idx in &OP_ORDER {
            let op = &inst.operators[idx];
            let is_carrier = (carrier_mask >> idx) & 1 == 1;
            bytes.push(op.total_level | if is_carrier { 0x80 } else { 0 });
        }
        bytes.push((inst.feedback << 3) | inst.algorithm);

        bytes
    }

    fn fm_from_bytes(&self, bytes: &[u8]) -> Result<FmInstrument, String> {
        if bytes.len() != 25 {
            return Err(format!("expected 25 bytes, got {}", bytes.len()));
        }

        let fb_alg = bytes[24];
        let algorithm = fb_alg & 0x07;
        let feedback = (fb_alg >> 3) & 0x07;

        let mut operators = [
            FmOperator::default(),
            FmOperator::default(),
            FmOperator::default(),
            FmOperator::default(),
        ];

        for (pos, &idx) in OP_ORDER.iter().enumerate() {
            operators[idx].detune = bytes[pos] >> 4;
            operators[idx].multiple = bytes[pos] & 0x0F;
            operators[idx].rate_scale = bytes[4 + pos] >> 6;
            operators[idx].attack_rate = bytes[4 + pos] & 0x1F;
            operators[idx].amp_mod = bytes[8 + pos] & 0x80 != 0;
            operators[idx].d1r = bytes[8 + pos] & 0x1F;
            operators[idx].d2r = bytes[12 + pos] & 0x1F;
            operators[idx].sustain_level = bytes[16 + pos] >> 4;
            operators[idx].release_rate = bytes[16 + pos] & 0x0F;
            operators[idx].total_level = bytes[20 + pos] & 0x7F;
        }

        Ok(FmInstrument {
            id: Uuid::new_v4(),
            name: String::new(),
            algorithm,
            feedback,
            operators,
            metadata: InstrumentMetadata::default(),
        })
    }

    fn import_formats(&self) -> Vec<&str> {
        vec!["smps2asm"]
    }

    fn export_formats(&self) -> Vec<&str> {
        vec!["smps2asm", "binary"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_instrument() -> FmInstrument {
        FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 4,
            feedback: 5,
            operators: [
                FmOperator { detune: 1, multiple: 2, rate_scale: 0, attack_rate: 31, amp_mod: false, d1r: 5, d2r: 3, sustain_level: 2, release_rate: 8, total_level: 10 },
                FmOperator { detune: 3, multiple: 4, rate_scale: 1, attack_rate: 28, amp_mod: true, d1r: 7, d2r: 5, sustain_level: 4, release_rate: 10, total_level: 20 },
                FmOperator { detune: 5, multiple: 6, rate_scale: 2, attack_rate: 25, amp_mod: false, d1r: 9, d2r: 7, sustain_level: 6, release_rate: 12, total_level: 30 },
                FmOperator { detune: 7, multiple: 8, rate_scale: 3, attack_rate: 22, amp_mod: true, d1r: 11, d2r: 9, sustain_level: 8, release_rate: 14, total_level: 40 },
            ],
            metadata: InstrumentMetadata::default(),
        }
    }

    #[test]
    fn test_fm_to_bytes_length() {
        let driver = FlamedriverProfile;
        let inst = make_test_instrument();
        let bytes = driver.fm_to_bytes(&inst);
        assert_eq!(bytes.len(), 25);
    }

    #[test]
    fn test_fm_to_bytes_algo_feedback_is_last() {
        let driver = FlamedriverProfile;
        let inst = make_test_instrument();
        let bytes = driver.fm_to_bytes(&inst);
        assert_eq!(bytes[24], (5 << 3) | 4);
    }

    #[test]
    fn test_fm_to_bytes_operator_order_is_4321() {
        let driver = FlamedriverProfile;
        let inst = make_test_instrument();
        let bytes = driver.fm_to_bytes(&inst);
        // Op4 (idx 3): detune=7, mul=8 → (7<<4)|8 = 0x78
        assert_eq!(bytes[0], (7 << 4) | 8);
        // Op3 (idx 2): detune=5, mul=6 → (5<<4)|6 = 0x56
        assert_eq!(bytes[1], (5 << 4) | 6);
        // Op2 (idx 1): detune=3, mul=4 → (3<<4)|4 = 0x34
        assert_eq!(bytes[2], (3 << 4) | 4);
        // Op1 (idx 0): detune=1, mul=2 → (1<<4)|2 = 0x12
        assert_eq!(bytes[3], (1 << 4) | 2);
    }

    #[test]
    fn test_fm_to_bytes_carrier_flags_algo4() {
        let driver = FlamedriverProfile;
        let inst = make_test_instrument(); // algorithm 4: carriers = op2, op4
        let bytes = driver.fm_to_bytes(&inst);
        // TL bytes 20-23: ops 4,3,2,1
        assert_eq!(bytes[20], 40 | 0x80); // Op4 (carrier)
        assert_eq!(bytes[21], 30);         // Op3 (modulator)
        assert_eq!(bytes[22], 20 | 0x80); // Op2 (carrier)
        assert_eq!(bytes[23], 10);         // Op1 (modulator)
    }

    #[test]
    fn test_fm_round_trip() {
        let driver = FlamedriverProfile;
        let original = make_test_instrument();
        let bytes = driver.fm_to_bytes(&original);
        let parsed = driver.fm_from_bytes(&bytes).unwrap();
        assert_eq!(parsed.algorithm, original.algorithm);
        assert_eq!(parsed.feedback, original.feedback);
        for i in 0..4 {
            assert_eq!(parsed.operators[i].detune, original.operators[i].detune);
            assert_eq!(parsed.operators[i].multiple, original.operators[i].multiple);
            assert_eq!(parsed.operators[i].rate_scale, original.operators[i].rate_scale);
            assert_eq!(parsed.operators[i].attack_rate, original.operators[i].attack_rate);
            assert_eq!(parsed.operators[i].amp_mod, original.operators[i].amp_mod);
            assert_eq!(parsed.operators[i].d1r, original.operators[i].d1r);
            assert_eq!(parsed.operators[i].d2r, original.operators[i].d2r);
            assert_eq!(parsed.operators[i].sustain_level, original.operators[i].sustain_level);
            assert_eq!(parsed.operators[i].release_rate, original.operators[i].release_rate);
            assert_eq!(parsed.operators[i].total_level, original.operators[i].total_level);
        }
    }

    #[test]
    fn test_fm_from_bytes_rejects_wrong_length() {
        let driver = FlamedriverProfile;
        assert!(driver.fm_from_bytes(&[0u8; 24]).is_err());
        assert!(driver.fm_from_bytes(&[0u8; 26]).is_err());
    }

    #[test]
    fn test_validate_fm_accepts_valid() {
        let driver = FlamedriverProfile;
        let inst = make_test_instrument();
        assert!(driver.validate_fm(&inst).is_ok());
    }

    #[test]
    fn test_validate_fm_catches_bad_algorithm() {
        let driver = FlamedriverProfile;
        let mut inst = make_test_instrument();
        inst.algorithm = 8;
        let err = driver.validate_fm(&inst).unwrap_err();
        assert!(err.iter().any(|e| e.contains("algorithm")));
    }

    #[test]
    fn test_validate_psg_catches_empty_envelope() {
        let driver = FlamedriverProfile;
        let inst = PsgInstrument {
            id: Uuid::nil(),
            name: "Bad".into(),
            volume_sequence: vec![],
            loop_point: None,
            noise_mode: None,
            metadata: InstrumentMetadata::default(),
        };
        let err = driver.validate_psg(&inst).unwrap_err();
        assert!(err.iter().any(|e| e.contains("empty")));
    }

    #[test]
    fn test_validate_dac_catches_bad_rate() {
        let driver = FlamedriverProfile;
        let inst = DacInstrument {
            id: Uuid::nil(),
            name: "Bad".into(),
            target_sample_rate: 12345,
            loop_start: None,
            loop_length: None,
            original_file: String::new(),
            pcm_file: String::new(),
            source_is_raw: false,
            metadata: InstrumentMetadata::default(),
        };
        let err = driver.validate_dac(&inst).unwrap_err();
        assert!(err.iter().any(|e| e.contains("target_sample_rate")));
    }

    #[test]
    fn test_channel_layout() {
        let driver = FlamedriverProfile;
        let layout = driver.channel_layout();
        assert_eq!(layout.fm_channels.len(), 6);
        assert_eq!(layout.psg_channels.len(), 4);
        assert_eq!(layout.dac_channels.len(), 1);
        assert!(layout.fm_channels[2].supports_special_mode);
        assert!(!layout.fm_channels[0].supports_special_mode);
        assert!(layout.psg_channels[3].is_noise);
    }

    #[test]
    fn test_supports_feature() {
        let driver = FlamedriverProfile;
        assert!(driver.supports_feature(DriverFeature::Fm3SpecialMode));
        assert!(!driver.supports_feature(DriverFeature::SsgEg));
        assert!(!driver.supports_feature(DriverFeature::Dpcm));
    }
}
