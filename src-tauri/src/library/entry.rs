//! Library entry file format: a thin metadata wrapper around the existing
//! instrument serde types, plus content-hash identity.
//! See docs/superpowers/specs/2026-07-16-instrument-library-design.md.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::model::instrument::{FmInstrument, NoiseMode, PsgInstrument};

pub const LIBRARY_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provenance {
    pub game: String,
    /// Every song the (deduped) voice appears in.
    #[serde(default)]
    pub songs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<u8>,
    /// Content hash — the entry's identity across roots and re-extractions.
    pub hash: String,
}

/// `{"type":"fm","instrument":{...}}` shape per the spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "instrument", rename_all = "lowercase")]
pub enum LibraryInstrument {
    Fm(FmInstrument),
    Psg(PsgInstrument),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryEntryFile {
    pub schema: u32,
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub provenance: Provenance,
    #[serde(flatten)]
    pub instrument: LibraryInstrument,
}

/// Canonical byte string for an FM patch: fields only, fixed order, no JSON,
/// no floats — identical sound == identical bytes == identical hash.
pub fn fm_canonical_bytes(inst: &FmInstrument) -> Vec<u8> {
    let mut b = vec![inst.algorithm, inst.feedback];
    for op in &inst.operators {
        b.extend_from_slice(&[
            op.detune, op.multiple, op.rate_scale, op.attack_rate,
            op.amp_mod as u8, op.d1r, op.d2r, op.sustain_level,
            op.release_rate, op.total_level, op.ssg_eg,
        ]);
    }
    b
}

/// Canonical bytes for a PSG preset. `smps_envelope_index` is EXCLUDED
/// (provenance, not sound); `noise_mode` is included (it is sound).
pub fn psg_canonical_bytes(inst: &PsgInstrument) -> Vec<u8> {
    let mut b = Vec::new();
    b.extend_from_slice(&inst.volume_sequence);
    b.push(0xFE); // field separator (volumes are 0..15, never 0xFE)
    match inst.loop_point {
        Some(lp) => b.extend_from_slice(&(lp as u64).to_le_bytes()),
        None => b.extend_from_slice(&u64::MAX.to_le_bytes()),
    }
    b.push(inst.silence_on_end as u8);
    match &inst.noise_mode {
        None => b.push(0),
        Some(NoiseMode::Periodic(p)) => { b.push(1); b.extend_from_slice(&p.to_le_bytes()); }
        Some(NoiseMode::White(p)) => { b.push(2); b.extend_from_slice(&p.to_le_bytes()); }
    }
    b
}

pub fn content_hash(instrument: &LibraryInstrument) -> String {
    let bytes = match instrument {
        LibraryInstrument::Fm(i) => fm_canonical_bytes(i),
        LibraryInstrument::Psg(i) => psg_canonical_bytes(i),
    };
    format!("sha256:{:x}", Sha256::digest(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::instrument::{FmOperator, InstrumentMetadata};
    use uuid::Uuid;

    fn sample_fm() -> FmInstrument {
        FmInstrument {
            id: Uuid::nil(),
            name: "Test".into(),
            algorithm: 4,
            feedback: 5,
            operators: [FmOperator::default(); 4],
            metadata: InstrumentMetadata::default(),
        }
    }

    #[test]
    fn wrapper_json_shape_matches_spec() {
        let entry = LibraryEntryFile {
            schema: LIBRARY_SCHEMA,
            name: "EHZ Lead".into(),
            tags: vec!["lead".into()],
            provenance: Provenance {
                game: "Sonic 2".into(),
                songs: vec!["EHZ".into()],
                slot: Some(3),
                hash: "sha256:abc".into(),
            },
            instrument: LibraryInstrument::Fm(sample_fm()),
        };
        let v: serde_json::Value = serde_json::to_value(&entry).unwrap();
        assert_eq!(v["type"], "fm");
        assert!(v["instrument"]["algorithm"].is_number());
        assert_eq!(v["schema"], 1);
        assert_eq!(v["provenance"]["game"], "Sonic 2");
        // round-trip
        let back: LibraryEntryFile = serde_json::from_value(v).unwrap();
        assert_eq!(back.name, "EHZ Lead");
    }

    #[test]
    fn fm_hash_ignores_name_and_id_but_not_patch() {
        let a = sample_fm();
        let mut b = sample_fm();
        b.name = "Different".into();
        b.id = Uuid::new_v4();
        assert_eq!(
            content_hash(&LibraryInstrument::Fm(a.clone())),
            content_hash(&LibraryInstrument::Fm(b.clone()))
        );
        b.algorithm = 7;
        assert_ne!(
            content_hash(&LibraryInstrument::Fm(a)),
            content_hash(&LibraryInstrument::Fm(b))
        );
    }

    #[test]
    fn psg_hash_excludes_envelope_index() {
        let mk = |idx: Option<u8>| PsgInstrument {
            id: Uuid::nil(),
            name: "e".into(),
            volume_sequence: vec![15, 12, 8, 4, 0],
            loop_point: Some(2),
            silence_on_end: true,
            noise_mode: None,
            smps_envelope_index: idx,
            metadata: InstrumentMetadata::default(),
        };
        assert_eq!(
            content_hash(&LibraryInstrument::Psg(mk(Some(3)))),
            content_hash(&LibraryInstrument::Psg(mk(None)))
        );
    }
}
