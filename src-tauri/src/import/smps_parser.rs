use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct SmpsFile {
    pub song_label: String,
    pub voice_ref: VoiceRef,
    pub fm_count: u8,
    pub psg_count: u8,
    pub tempo_divider: u8,
    pub tempo_modifier: u8,
    pub channels: Vec<SmpsChannel>,
    pub voices: Vec<[u8; 25]>,
}

#[derive(Debug, Clone)]
pub enum VoiceRef {
    Inline(String),
    Uvb,
    External(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmpsChannelKind {
    Dac,
    Fm,
    Psg,
}

#[derive(Debug, Clone)]
pub struct SmpsChannel {
    pub kind: SmpsChannelKind,
    pub label: String,
    pub initial_pitch: i8,
    pub initial_volume: u8,
    pub psg_envelope: Option<u8>,
    pub events: Vec<SmpsEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SmpsEvent {
    Note { pitch: u8, duration: u8 },
    Rest { duration: u8 },
    SetVoice(u8),
    SetPan(u8),
    Transpose(i8),
    Tie,
    Stop,
    Unsupported { name: String },
}

pub fn build_note_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    map.insert("nRst".into(), 0x80);
    let note_names = ["C", "Cs", "D", "Eb", "E", "F", "Fs", "G", "Ab", "A", "Bb", "B"];
    let aliases = [
        ("Db", "Cs"), ("Ds", "Eb"), ("Es", "F"), ("Fb", "E"),
        ("Gb", "Fs"), ("Gs", "Ab"), ("As", "Bb"), ("Bs", "C"),
        ("Cb", "B"),
    ];
    for octave in 0..=7u8 {
        for (semi, name) in note_names.iter().enumerate() {
            let byte = 0x81 + octave * 12 + semi as u8;
            if byte > 0x80 {
                map.insert(format!("n{name}{octave}"), byte);
            }
        }
    }
    for octave in 0..=7u8 {
        for &(alias, canonical) in &aliases {
            if alias == "Bs" {
                let target_octave = octave + 1;
                if let Some(&val) = map.get(&format!("n{canonical}{target_octave}")) {
                    map.insert(format!("n{alias}{octave}"), val);
                }
            } else if alias == "Cb" {
                if octave > 0 {
                    let target_octave = octave - 1;
                    if let Some(&val) = map.get(&format!("n{canonical}{target_octave}")) {
                        map.insert(format!("n{alias}{octave}"), val);
                    }
                }
            } else if let Some(&val) = map.get(&format!("n{canonical}{octave}")) {
                map.insert(format!("n{alias}{octave}"), val);
            }
        }
    }
    map
}

pub fn build_dac_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    let dac_names: &[(&str, u8)] = &[
        ("dSnareS3", 0x81), ("dHighTom", 0x82), ("dMidTomS3", 0x83),
        ("dLowTomS3", 0x84), ("dFloorTomS3", 0x85), ("dKickS3", 0x86),
        ("dMuffledSnare", 0x87), ("dCrashCymbal", 0x88), ("dRideCymbal", 0x89),
        ("dLowMetalHit", 0x8A), ("dMetalHit", 0x8B), ("dHighMetalHit", 0x8C),
        ("dHigherMetalHit", 0x8D), ("dMidMetalHit", 0x8E), ("dClapS3", 0x8F),
        ("dElectricHighTom", 0x90), ("dElectricMidTom", 0x91),
        ("dElectricLowTom", 0x92), ("dElectricFloorTom", 0x93),
        ("dTightSnare", 0x94), ("dMidpitchSnare", 0x95), ("dLooseSnare", 0x96),
        ("dLooserSnare", 0x97), ("dHiTimpaniS3", 0x98), ("dLowTimpaniS3", 0x99),
        ("dMidTimpaniS3", 0x9A), ("dQuickLooseSnare", 0x9B), ("dClick", 0x9C),
        ("dPowerKick", 0x9D), ("dQuickGlassCrash", 0x9E),
        ("dGlassCrashSnare", 0x9F), ("dGlassCrash", 0xA0),
        ("dGlassCrashKick", 0xA1), ("dQuietGlassCrash", 0xA2),
        ("dOddSnareKick", 0xA3), ("dKickExtraBass", 0xA4), ("dComeOn", 0xA5),
        ("dDanceSnare", 0xA6), ("dLooseKick", 0xA7), ("dModLooseKick", 0xA8),
        ("dWoo", 0xA9), ("dGo", 0xAA), ("dSnareGo", 0xAB), ("dPowerTom", 0xAC),
        ("dHiWoodBlock", 0xAD), ("dLowWoodBlock", 0xAE), ("dHiHitDrum", 0xAF),
        ("dLowHitDrum", 0xB0), ("dMetalCrashHit", 0xB1),
        ("dEchoedClapHit_S3", 0xB2), ("dLowerEchoedClapHit_S3", 0xB3),
        ("dHipHopHitKick", 0xB4), ("dHipHopHitPowerKick", 0xB5),
        ("dBassHey", 0xB6), ("dDanceStyleKick", 0xB7),
        ("dHipHopHitKick2", 0xB8), ("dHipHopHitKick3", 0xB9),
        ("dReverseFadingWind", 0xBA), ("dScratchS3", 0xBB),
        ("dLooseSnareNoise", 0xBC), ("dPowerKick2", 0xBD),
        ("dCrashingNoiseWoo", 0xBE), ("dQuickHit", 0xBF),
        ("dKickHey", 0xC0), ("dPowerKickHit", 0xC1),
        ("dLowPowerKickHit", 0xC2), ("dLowerPowerKickHit", 0xC3),
        ("dLowestPowerKickHit", 0xC4),
    ];
    for &(name, val) in dac_names {
        map.insert(name.into(), val);
    }
    map
}

pub fn build_pan_table() -> HashMap<String, u8> {
    let mut map = HashMap::new();
    map.insert("panNone".into(), 0x00);
    map.insert("panRight".into(), 0x40);
    map.insert("panLeft".into(), 0x80);
    map.insert("panCenter".into(), 0xC0);
    map.insert("panCentre".into(), 0xC0);
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_table_nrst() {
        let table = build_note_table();
        assert_eq!(table["nRst"], 0x80);
    }

    #[test]
    fn test_note_table_nc0_is_81() {
        let table = build_note_table();
        assert_eq!(table["nC0"], 0x81);
    }

    #[test]
    fn test_note_table_aliases() {
        let table = build_note_table();
        assert_eq!(table["nDb0"], table["nCs0"]);
        assert_eq!(table["nEb4"], table["nDs4"]);
        assert_eq!(table["nFs3"], table["nGb3"]);
    }

    #[test]
    fn test_dac_table() {
        let table = build_dac_table();
        assert_eq!(table["dKickS3"], 0x86);
        assert_eq!(table["dSnareS3"], 0x81);
    }

    #[test]
    fn test_pan_table() {
        let table = build_pan_table();
        assert_eq!(table["panCenter"], 0xC0);
        assert_eq!(table["panLeft"], 0x80);
        assert_eq!(table["panRight"], 0x40);
    }
}
