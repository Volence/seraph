use std::path::Path;
use crate::export::{ExportResult, ExportError};
use crate::model::driver::*;
use crate::model::instrument::*;
use crate::model::song::Song;
use uuid::Uuid;

pub struct FlamedriverProfile;

/// Bit i = 1 means operators[i] is a carrier for that algorithm.
/// Matches Flamedriver vcTLMask: op1=always, op2=algo≥5, op3=algo≥4, op4=algo==7.
const CARRIER_MASKS: [u8; 8] = [
    0b0001, // algo 0: op1
    0b0001, // algo 1: op1
    0b0001, // algo 2: op1
    0b0001, // algo 3: op1
    0b0101, // algo 4: op1, op3
    0b0111, // algo 5: op1, op2, op3
    0b0111, // algo 6: op1, op2, op3
    0b1111, // algo 7: all
];

const OP_ORDER: [usize; 4] = [0, 1, 2, 3];

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
                // No FM6 music voice. The driver says so itself at
                // `skdisasm/Sound/Z80 Sound Driver.asm:1907`
                // (`db 80h, 6 ; FM6 music track (does not exist in this driver)`),
                // its track RAM puts `zSongFM6_DAC` ahead of FM1..FM5 as ONE
                // shared slot (177-182), and its own "Number of FM tracks" is
                // `(zSongPSG1-zSongFM1)/zTrack.len` = 5 (2252). Channel 6 is the
                // DAC, declared once below. Advertising it here as well offered
                // seven voices on a six-voice chip (audit F31).
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
        // "binary" was advertised here with nothing implementing it (README
        // pass, item 5). Withdrawn rather than stubbed: this lane's standing
        // rule is that the app must not promise what it cannot do. Note the
        // method itself currently has NO callers, so this corrects a false
        // claim rather than a behaviour -- which is exactly why it could sit
        // here being wrong without anything failing.
        vec!["smps2asm"]
    }

    fn export_song(
        &self,
        song: &Song,
        instruments: &InstrumentBank,
        output_dir: &Path,
        project_dir: Option<&Path>,
    ) -> Result<ExportResult, Vec<ExportError>> {
        crate::export::smps::write_export(song, instruments, self, output_dir, project_dir)
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
                FmOperator { detune: 1, multiple: 2, rate_scale: 0, attack_rate: 31, amp_mod: false, d1r: 5, d2r: 3, sustain_level: 2, release_rate: 8, total_level: 10, ssg_eg: 0 },
                FmOperator { detune: 3, multiple: 4, rate_scale: 1, attack_rate: 28, amp_mod: true, d1r: 7, d2r: 5, sustain_level: 4, release_rate: 10, total_level: 20, ssg_eg: 0 },
                FmOperator { detune: 5, multiple: 6, rate_scale: 2, attack_rate: 25, amp_mod: false, d1r: 9, d2r: 7, sustain_level: 6, release_rate: 12, total_level: 30, ssg_eg: 0 },
                FmOperator { detune: 7, multiple: 8, rate_scale: 3, attack_rate: 22, amp_mod: true, d1r: 11, d2r: 9, sustain_level: 8, release_rate: 14, total_level: 40, ssg_eg: 0 },
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
    fn test_fm_to_bytes_operator_order() {
        let driver = FlamedriverProfile;
        let inst = make_test_instrument();
        let bytes = driver.fm_to_bytes(&inst);
        assert_eq!(bytes[0], (1 << 4) | 2);
        assert_eq!(bytes[1], (3 << 4) | 4);
        assert_eq!(bytes[2], (5 << 4) | 6);
        assert_eq!(bytes[3], (7 << 4) | 8);
    }

    #[test]
    fn test_fm_to_bytes_carrier_flags_algo4() {
        let driver = FlamedriverProfile;
        let inst = make_test_instrument(); // algorithm 4: carriers = op1 (operators[0]), op3 (operators[2])
        let bytes = driver.fm_to_bytes(&inst);
        assert_eq!(bytes[20], 10 | 0x80);  // operators[0] = op1 (carrier)
        assert_eq!(bytes[21], 20);          // operators[1] = op2 (modulator)
        assert_eq!(bytes[22], 30 | 0x80);  // operators[2] = op3 (carrier)
        assert_eq!(bytes[23], 40);          // operators[3] = op4 (modulator)
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
            silence_on_end: true,
            noise_mode: None,
            smps_envelope_index: None,
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
        // Five, not six: this driver has no FM6 music voice. Channel 6 is the
        // DAC and is declared once, in `dac_channels` (audit F31).
        assert_eq!(layout.fm_channels.len(), 5);
        assert_eq!(layout.psg_channels.len(), 4);
        assert_eq!(layout.dac_channels.len(), 1);
        assert!(layout.fm_channels[2].supports_special_mode);
        assert!(!layout.fm_channels[0].supports_special_mode);
        assert!(layout.psg_channels[3].is_noise);
        assert!(
            !layout.fm_channels.iter().any(|c| c.index == 5),
            "FM index 5 is the DAC slot on this driver and must not also be \
             offered as a music voice",
        );
    }

    /// The guard F31 was missing. The YM2612 has six FM slots and the DAC
    /// takes over the sixth, so a profile's music voices plus its DAC
    /// channels can never exceed six. `FlamedriverProfile` advertised six FM
    /// voices (including one named "FM6/DAC") *plus* a separate DAC channel:
    /// seven voices on a six-voice chip.
    ///
    /// SCOPE: this now covers **every driver the app registers**, not a list
    /// re-typed here. F31 shipped it over a hand-written vec and said so; F34
    /// extracted `driver::default_registry()` as the single registration site,
    /// so a profile added there is covered by this guard automatically and a
    /// new driver cannot quietly escape it.
    #[test]
    fn fm_voices_plus_dac_never_exceed_the_chips_six_slots() {
        let registry = crate::driver::default_registry();
        let profiles: Vec<&dyn DriverProfile> = registry.profiles().collect();
        assert!(
            !profiles.is_empty(),
            "the registry must contain at least one driver, or this guard \
             passes by having nothing to check",
        );
        for driver in &profiles {
            let layout = driver.channel_layout();
            let total = layout.fm_channels.len() + layout.dac_channels.len();
            assert!(
                total <= 6,
                "driver `{}` advertises {} FM voices plus {} DAC channels = {}, \
                 but the chip has only 6 FM slots and the DAC occupies one of them",
                driver.id(),
                layout.fm_channels.len(),
                layout.dac_channels.len(),
                total,
            );
        }
    }

    #[test]
    fn test_supports_feature() {
        let driver = FlamedriverProfile;
        assert!(driver.supports_feature(DriverFeature::Fm3SpecialMode));
        assert!(!driver.supports_feature(DriverFeature::SsgEg));
        assert!(!driver.supports_feature(DriverFeature::Dpcm));
    }
}
