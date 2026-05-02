use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstrumentBank {
    pub fm: Vec<FmInstrument>,
    pub psg: Vec<PsgInstrument>,
    pub dac: Vec<DacInstrument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmInstrument {
    pub id: Uuid,
    pub name: String,
    pub algorithm: u8,
    pub feedback: u8,
    pub operators: [FmOperator; 4],
    pub metadata: InstrumentMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FmOperator {
    pub detune: u8,
    pub multiple: u8,
    pub rate_scale: u8,
    pub attack_rate: u8,
    pub amp_mod: bool,
    pub d1r: u8,
    pub d2r: u8,
    pub sustain_level: u8,
    pub release_rate: u8,
    pub total_level: u8,
}

impl Default for FmOperator {
    fn default() -> Self {
        Self {
            detune: 0, multiple: 1, rate_scale: 0, attack_rate: 31,
            amp_mod: false, d1r: 0, d2r: 0, sustain_level: 0,
            release_rate: 15, total_level: 127,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PsgInstrument {
    pub id: Uuid,
    pub name: String,
    pub volume_sequence: Vec<u8>,
    pub loop_point: Option<usize>,
    pub noise_mode: Option<NoiseMode>,
    pub metadata: InstrumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NoiseMode {
    Periodic(u16),
    White(u16),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DacInstrument {
    pub id: Uuid,
    pub name: String,
    pub target_sample_rate: u32,
    pub loop_start: Option<u32>,
    pub loop_length: Option<u32>,
    pub original_file: String,
    pub pcm_file: String,
    pub source_is_raw: bool,
    pub metadata: InstrumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstrumentMetadata {
    pub category: String,
    pub author: String,
    pub tags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_fm_operator() -> FmOperator {
        FmOperator {
            detune: 4, multiple: 1, rate_scale: 0, attack_rate: 31,
            amp_mod: false, d1r: 4, d2r: 0, sustain_level: 1,
            release_rate: 8, total_level: 5,
        }
    }

    #[test]
    fn test_fm_instrument_json_round_trip() {
        let inst = FmInstrument {
            id: Uuid::new_v4(),
            name: "DEZ Bass".into(),
            algorithm: 0,
            feedback: 2,
            operators: [test_fm_operator(), test_fm_operator(), test_fm_operator(), test_fm_operator()],
            metadata: InstrumentMetadata::default(),
        };
        let json = serde_json::to_string_pretty(&inst).unwrap();
        assert!(json.contains("\"attackRate\""));
        assert!(json.contains("\"ampMod\""));
        let parsed: FmInstrument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.algorithm, 0);
        assert_eq!(parsed.operators[0].detune, 4);
    }

    #[test]
    fn test_psg_instrument_json_round_trip() {
        let inst = PsgInstrument {
            id: Uuid::new_v4(),
            name: "Pluck".into(),
            volume_sequence: vec![15, 14, 12, 10, 8, 6, 4, 2, 0],
            loop_point: Some(5),
            noise_mode: None,
            metadata: InstrumentMetadata::default(),
        };
        let json = serde_json::to_string(&inst).unwrap();
        let parsed: PsgInstrument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.volume_sequence.len(), 9);
        assert_eq!(parsed.loop_point, Some(5));
    }

    #[test]
    fn test_dac_instrument_json_round_trip() {
        let inst = DacInstrument {
            id: Uuid::new_v4(),
            name: "Kick".into(),
            target_sample_rate: 16000,
            loop_start: None,
            loop_length: None,
            original_file: "kick.wav".into(),
            pcm_file: "kick.pcm".into(),
            source_is_raw: false,
            metadata: InstrumentMetadata::default(),
        };
        let json = serde_json::to_string(&inst).unwrap();
        assert!(json.contains("\"targetSampleRate\":16000"));
        assert!(json.contains("\"sourceIsRaw\":false"));
        let parsed: DacInstrument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.target_sample_rate, 16000);
    }

    #[test]
    fn test_instrument_bank_default_is_empty() {
        let bank = InstrumentBank::default();
        assert!(bank.fm.is_empty());
        assert!(bank.psg.is_empty());
        assert!(bank.dac.is_empty());
    }
}
