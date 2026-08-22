use std::sync::Arc;
use serde::Serialize;

/// Identifies a hardware channel. `PartialEq` matters: `reload_snapshot`
/// re-identifies a sounding channel across a snapshot swap by channel type,
/// not by index — the index is a position in a `BTreeMap` of non-muted tracks
/// and shifts whenever a track is muted, soloed or deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelType {
    Fm(u8),
    Psg(u8),
    PsgNoise,
    Dac(u8),
}

#[derive(Debug, Clone)]
pub enum InstrumentData {
    FmPatch { bytes: [u8; 25], ssg_eg: [u8; 4] },
    PsgEnvelope { envelope: Arc<Vec<u8>>, loop_point: Option<usize>, silence_on_end: bool },
    DacSample { samples: Arc<Vec<u8>>, sample_rate: u32 },
}

#[derive(Debug, Clone)]
pub enum SequencerEvent {
    NoteOn { tick: u64, pitch: u8, velocity: u8, detune: i8, duration_ticks: u64, instrument: InstrumentData, modulation: Option<ModulationParams>, pan_override: Option<u8> },
    // FINDING: `pitch` carries the correct (transposed) pitch but `process_event`
    // keys off unconditionally, so a stale NoteOff truncates a later overlapping
    // note on the same channel. Kept as the evidence for that gap.
    NoteOff { tick: u64, #[allow(dead_code)] pitch: u8 },
}

impl SequencerEvent {
    pub fn tick(&self) -> u64 {
        match self {
            SequencerEvent::NoteOn { tick, .. } => *tick,
            SequencerEvent::NoteOff { tick, .. } => *tick,
        }
    }
}

#[derive(Debug, Clone, Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OverlapWarning {
    pub channel_name: String,
    pub tick_start: u64,
    pub tick_end: u64,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModulationParams {
    pub wait: u8,
    pub speed: u8,
    pub delta: u8,
    pub steps: u8,
}

#[derive(Debug, Clone)]
pub struct ChannelSequence {
    pub channel_type: ChannelType,
    pub volume: u8,
    pub pan: u8,
    pub noise_reg: u8,
    pub events: Vec<SequencerEvent>,
    pub overlaps: Vec<OverlapWarning>,
}

#[derive(Debug, Clone)]
pub struct SequencerSnapshot {
    pub tempo_bpm: f64,
    pub ticks_per_beat: u32,
    pub loop_start: Option<u64>,
    pub loop_end: Option<u64>,
    pub channels: Vec<ChannelSequence>,
}

impl SequencerSnapshot {
    pub fn empty() -> Self {
        Self {
            tempo_bpm: 120.0,
            ticks_per_beat: 480,
            loop_start: None,
            loop_end: None,
            channels: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_snapshot() {
        let snap = SequencerSnapshot::empty();
        assert_eq!(snap.tempo_bpm, 120.0);
        assert!(snap.channels.is_empty());
    }

    #[test]
    fn test_event_tick_accessor() {
        let on = SequencerEvent::NoteOn {
            tick: 480,
            pitch: 60,
            velocity: 100,
            detune: 0,
            duration_ticks: 240,
            instrument: InstrumentData::FmPatch { bytes: [0; 25], ssg_eg: [0; 4] },
            modulation: None,
            pan_override: None,
        };
        assert_eq!(on.tick(), 480);
        let off = SequencerEvent::NoteOff { tick: 720, pitch: 60 };
        assert_eq!(off.tick(), 720);
    }

    #[test]
    fn test_instrument_data_clone() {
        let fm = InstrumentData::FmPatch { bytes: [42; 25], ssg_eg: [0; 4] };
        let fm2 = fm.clone();
        match fm2 {
            InstrumentData::FmPatch { bytes, .. } => assert_eq!(bytes[0], 42),
            _ => panic!("wrong variant"),
        }
    }
}
