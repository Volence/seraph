use std::sync::Arc;
use serde::Serialize;

#[derive(Debug, Clone)]
pub enum ChannelType {
    Fm(u8),
    Psg(u8),
    PsgNoise,
    Dac(u8),
}

#[derive(Debug, Clone)]
pub enum InstrumentData {
    FmPatch { bytes: [u8; 25], ssg_eg: [u8; 4] },
    PsgEnvelope { period: u16, envelope: Arc<Vec<u8>>, loop_point: Option<usize> },
    DacSample { samples: Arc<Vec<u8>>, sample_rate: u32 },
}

#[derive(Debug, Clone)]
pub enum SequencerEvent {
    NoteOn { tick: u64, pitch: u8, velocity: u8, duration_ticks: u64, instrument: InstrumentData },
    NoteOff { tick: u64, pitch: u8 },
}

impl SequencerEvent {
    pub fn tick(&self) -> u64 {
        match self {
            SequencerEvent::NoteOn { tick, .. } => *tick,
            SequencerEvent::NoteOff { tick, .. } => *tick,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlapWarning {
    pub channel_name: String,
    pub tick_start: u64,
    pub tick_end: u64,
    pub track_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ChannelSequence {
    pub channel_type: ChannelType,
    pub volume: u8,
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
            duration_ticks: 240,
            instrument: InstrumentData::FmPatch { bytes: [0; 25], ssg_eg: [0; 4] },
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
