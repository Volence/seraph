use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
pub struct InstrumentBank {
    pub fm: Vec<FmInstrument>,
    pub psg: Vec<PsgInstrument>,
    pub dac: Vec<DacInstrument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FmInstrument {
    pub id: Uuid,
    pub name: String,
    pub algorithm: u8,
    pub feedback: u8,
    pub operators: [FmOperator; 4],
    pub metadata: InstrumentMetadata,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct FmOperator {
    pub detune: u8,
    pub multiple: u8,
    pub rate_scale: u8,
    pub attack_rate: u8,
    pub amp_mod: bool,
    // serde's camelCase leaves `d1r`/`d2r` unchanged, but specta's camelCase
    // would emit `d1R`/`d2R`; pin the specta name to match the wire format.
    #[specta(rename = "d1r")]
    pub d1r: u8,
    #[specta(rename = "d2r")]
    pub d2r: u8,
    pub sustain_level: u8,
    pub release_rate: u8,
    pub total_level: u8,
    #[serde(default)]
    pub ssg_eg: u8,
}

const CARRIER_MASKS: [u8; 8] = [
    0b0001, 0b0001, 0b0001, 0b0001,
    0b0101, 0b0111, 0b0111, 0b1111,
];

/// YM2612 register slot offset for `FmInstrument::operators[i]`.
///
/// Seraph stores operators in SMPS macro-argument order — the REVERSE of
/// Yamaha documentation order: `operators[0]` is Yamaha Op4 (the algorithm's
/// primary carrier, hardware slot +$0C) down to `operators[3]` = Yamaha Op1
/// (the feedback operator, slot +$00). Add the per-channel offset (0..=2) to
/// form the low bits of the register address.
///
/// SINGLE AUTHORITY: the sequencer, VGM export and the IPC FM preview all
/// program operators through this table. A private reversed copy in the
/// preview path was the "FM audition static" bug (carrier programmed into
/// the feedback slot) — do not re-derive this mapping locally.
pub const PACKED_OP_SLOTS: [u8; 4] = [0x0C, 0x04, 0x08, 0x00];

impl FmInstrument {
    pub fn pack_patch(&self) -> [u8; 25] {
        let mut b = [0u8; 25];
        let cm = CARRIER_MASKS[self.algorithm as usize];
        for i in 0..4 {
            let op = &self.operators[i];
            b[i]      = (op.detune << 4) | op.multiple;
            b[4 + i]  = (op.rate_scale << 6) | op.attack_rate;
            b[8 + i]  = ((op.amp_mod as u8) << 7) | op.d1r;
            b[12 + i] = op.d2r;
            b[16 + i] = (op.sustain_level << 4) | op.release_rate;
            b[20 + i] = op.total_level | if (cm >> i) & 1 == 1 { 0x80 } else { 0 };
        }
        b[24] = (self.feedback << 3) | self.algorithm;
        b
    }
}

impl Default for FmOperator {
    fn default() -> Self {
        Self {
            detune: 0, multiple: 1, rate_scale: 0, attack_rate: 31,
            amp_mod: false, d1r: 0, d2r: 0, sustain_level: 0,
            release_rate: 15, total_level: 127, ssg_eg: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PsgInstrument {
    pub id: Uuid,
    pub name: String,
    pub volume_sequence: Vec<u8>,
    pub loop_point: Option<usize>,
    #[serde(default)]
    pub silence_on_end: bool,
    pub noise_mode: Option<NoiseMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smps_envelope_index: Option<u8>,
    pub metadata: InstrumentMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub enum NoiseMode {
    Periodic(u16),
    White(u16),
}

#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, specta::Type)]
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
            release_rate: 8, total_level: 5, ssg_eg: 0,
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
            silence_on_end: true,
            noise_mode: None,
            smps_envelope_index: None,
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

    // --- serde <-> specta field-name parity guard ---
    //
    // specta's camelCase and serde's camelCase disagree on digit-boundary
    // identifiers (serde: `d1r` -> `d1r`, specta: `d1r` -> `d1R`), which is why
    // `FmOperator` pins those two fields with `#[specta(rename = ...)]`. That pin
    // is a second representation kept in sync by hand — exactly the failure mode
    // this task exists to kill. These tests mechanically assert that the keys
    // serde puts on the wire equal the keys specta emitted into the committed
    // `src/bindings.ts`, so any future divergence (a renamed field, a new
    // digit-boundary field) fails CI instead of silently shipping `undefined`.

    /// Extract the object-literal field keys for `export type <name> = { ... }`
    /// from the generated bindings, stripping the optional `?` marker.
    fn bindings_type_keys(name: &str) -> std::collections::BTreeSet<String> {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../src/bindings.ts"))
            .expect("src/bindings.ts must be generated (run `cargo test`)");
        let needle = format!("export type {name} = {{");
        let start = src
            .find(&needle)
            .unwrap_or_else(|| panic!("type `{name}` not found in bindings.ts"))
            + needle.len();
        let body = &src[start..];
        let end = body.find('}').expect("unterminated type body in bindings.ts");
        body[..end]
            .split(';')
            .filter_map(|field| {
                let field = field.trim();
                if field.is_empty() {
                    return None;
                }
                // "detune: number" or "ssgEg?: number" -> key
                let key = field.split(':').next()?.trim();
                Some(key.trim_end_matches('?').to_string())
            })
            .collect()
    }

    /// Assert that the JSON keys serde produces for `value` match the field names
    /// specta wrote into `bindings.ts` for `type_name`.
    fn assert_field_parity<T: serde::Serialize>(type_name: &str, value: &T) {
        let json = serde_json::to_value(value).expect("serialize sample");
        let serde_keys: std::collections::BTreeSet<String> = json
            .as_object()
            .expect("sample must serialize to a JSON object")
            .keys()
            .cloned()
            .collect();
        let specta_keys = bindings_type_keys(type_name);
        assert_eq!(
            serde_keys, specta_keys,
            "serde/specta field-name mismatch for `{type_name}`: \
             serde emits {serde_keys:?} but bindings.ts has {specta_keys:?}. \
             Reconcile with `#[specta(rename = ...)]` and regenerate bindings.",
        );
    }

    #[test]
    fn fm_operator_serde_specta_field_parity() {
        // Guards the `d1r`/`d2r` digit-boundary rename specifically.
        let op = FmOperator::default();
        assert_field_parity("FmOperator", &op);
        // Explicit belt-and-braces: the wire uses `d1r`/`d2r`, never `d1R`/`d2R`.
        let json = serde_json::to_value(op).unwrap();
        let obj = json.as_object().unwrap();
        assert!(obj.contains_key("d1r") && obj.contains_key("d2r"));
        assert!(!obj.contains_key("d1R") && !obj.contains_key("d2R"));
    }

    #[test]
    fn instrument_types_serde_specta_field_parity() {
        assert_field_parity("InstrumentMetadata", &InstrumentMetadata::default());
        assert_field_parity(
            "FmInstrument",
            &FmInstrument {
                id: Uuid::new_v4(),
                name: "sample".into(),
                algorithm: 0,
                feedback: 0,
                operators: [FmOperator::default(); 4],
                metadata: InstrumentMetadata::default(),
            },
        );
        assert_field_parity(
            "DacInstrument",
            &DacInstrument {
                id: Uuid::new_v4(),
                name: "sample".into(),
                target_sample_rate: 16000,
                loop_start: None,
                loop_length: None,
                original_file: "s.wav".into(),
                pcm_file: "s.pcm".into(),
                source_is_raw: false,
                metadata: InstrumentMetadata::default(),
            },
        );
    }
}
