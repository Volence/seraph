pub mod snapshot;

pub use snapshot::*;

use crate::audio::frequency::{midi_to_fm_freq, midi_to_psg_period};
use std::sync::Arc;

// Flamedriver packs operators in order: op4, op3, op2, op1.
// Map each packed position to the YM2612 register slot offset.
const PACKED_OP_SLOTS: [u8; 4] = [0x0C, 0x04, 0x08, 0x00];

pub struct Sequencer {
    snapshot: SequencerSnapshot,
    playing: bool,
    current_tick: f64,
    ticks_per_sample: f64,
    sample_rate: f64,
    channel_cursors: Vec<usize>,
    last_fm_patch: [[u8; 25]; 6],
    active_notes: Vec<Option<u8>>,
}

#[derive(Debug, Clone)]
pub struct FmRegisterWrite {
    pub port: u32,
    pub data: u8,
}

#[derive(Debug, Clone)]
pub enum SequencerOutput {
    FmWrite(FmRegisterWrite),
    PsgWrite(u8),
    DacPlayback { samples: Arc<Vec<u8>>, sample_rate: u32 },
}

impl Sequencer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            snapshot: SequencerSnapshot::empty(),
            playing: false,
            current_tick: 0.0,
            ticks_per_sample: 0.0,
            sample_rate: sample_rate as f64,
            channel_cursors: Vec::new(),
            last_fm_patch: [[0xFF; 25]; 6],
            active_notes: Vec::new(),
        }
    }

    pub fn load_snapshot(&mut self, snapshot: SequencerSnapshot) {
        self.ticks_per_sample =
            (snapshot.tempo_bpm / 60.0) * snapshot.ticks_per_beat as f64 / self.sample_rate;
        self.channel_cursors = vec![0; snapshot.channels.len()];
        self.active_notes = vec![None; snapshot.channels.len()];
        self.snapshot = snapshot;
    }

    pub fn reload_snapshot(&mut self, snapshot: SequencerSnapshot, output: &mut Vec<SequencerOutput>) {
        let was_playing = self.playing;
        let saved_tick = self.current_tick;
        let loop_start = self.snapshot.loop_start;
        let loop_end = self.snapshot.loop_end;
        self.silence_all(output);
        self.load_snapshot(snapshot);
        self.snapshot.loop_start = loop_start;
        self.snapshot.loop_end = loop_end;
        self.current_tick = saved_tick;
        self.seek_cursors();
        self.playing = was_playing;
    }

    pub fn play(&mut self) {
        if self.snapshot.channels.is_empty() && self.snapshot.tempo_bpm == 120.0 {
            return;
        }
        self.playing = true;
        self.seek_cursors();
    }

    pub fn stop(&mut self, output: &mut Vec<SequencerOutput>) {
        self.playing = false;
        self.silence_all(output);
    }

    pub fn seek(&mut self, tick: u64, output: &mut Vec<SequencerOutput>) {
        self.current_tick = tick as f64;
        self.silence_all(output);
        self.seek_cursors();
    }

    pub fn set_loop(&mut self, start: u64, end: u64) {
        self.snapshot.loop_start = Some(start);
        self.snapshot.loop_end = Some(end);
    }

    pub fn clear_loop(&mut self) {
        self.snapshot.loop_start = None;
        self.snapshot.loop_end = None;
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    pub fn current_tick_u64(&self) -> u64 {
        self.current_tick as u64
    }

    pub fn advance(&mut self, output: &mut Vec<SequencerOutput>) {
        if !self.playing {
            return;
        }

        let prev_tick = self.current_tick as u64;
        self.current_tick += self.ticks_per_sample;
        let new_tick = self.current_tick as u64;

        if let (Some(loop_start), Some(loop_end)) = (self.snapshot.loop_start, self.snapshot.loop_end) {
            if new_tick >= loop_end {
                self.current_tick = loop_start as f64 + (self.current_tick - loop_end as f64);
                self.silence_all(output);
                self.seek_cursors();
                return;
            }
        }

        if new_tick == prev_tick {
            return;
        }

        for ch_idx in 0..self.snapshot.channels.len() {
            let cursor = self.channel_cursors[ch_idx];

            // Collect event indices to process (avoids borrow conflict)
            let mut pending: Vec<usize> = Vec::new();
            let mut new_cursor = cursor;
            let events = &self.snapshot.channels[ch_idx].events;
            for i in cursor..events.len() {
                let et = events[i].tick();
                if et > new_tick {
                    break;
                }
                if et <= new_tick {
                    pending.push(i);
                }
                new_cursor = i + 1;
            }
            self.channel_cursors[ch_idx] = new_cursor;

            let channel_type = self.snapshot.channels[ch_idx].channel_type.clone();
            for i in pending {
                let event = self.snapshot.channels[ch_idx].events[i].clone();
                self.process_event(ch_idx, &channel_type, &event, output);
            }
        }
    }

    fn process_event(
        &mut self,
        ch_idx: usize,
        channel_type: &ChannelType,
        event: &SequencerEvent,
        output: &mut Vec<SequencerOutput>,
    ) {
        match event {
            SequencerEvent::NoteOn { pitch, velocity: _, instrument, .. } => {
                if self.active_notes[ch_idx].is_some() {
                    self.key_off_channel(ch_idx, channel_type, output);
                }

                match channel_type {
                    ChannelType::Fm(hw_ch) => {
                        self.program_fm(*hw_ch, *pitch, instrument, output);
                    }
                    ChannelType::Psg(hw_ch) => {
                        self.program_psg(*hw_ch, *pitch, output);
                    }
                    ChannelType::PsgNoise => {
                        self.program_psg_noise(output);
                    }
                    ChannelType::Dac(_) => {
                        if let InstrumentData::DacSample { samples, sample_rate } = instrument {
                            output.push(SequencerOutput::DacPlayback {
                                samples: samples.clone(),
                                sample_rate: *sample_rate,
                            });
                        }
                    }
                }
                self.active_notes[ch_idx] = Some(*pitch);
            }
            SequencerEvent::NoteOff { .. } => {
                self.key_off_channel(ch_idx, channel_type, output);
                self.active_notes[ch_idx] = None;
            }
        }
    }

    fn program_fm(&mut self, hw_ch: u8, pitch: u8, instrument: &InstrumentData, output: &mut Vec<SequencerOutput>) {
        let (port_base, ch_offset) = if hw_ch < 3 { (0u32, hw_ch) } else { (2u32, hw_ch - 3) };

        if let InstrumentData::FmPatch(patch) = instrument {
            if self.last_fm_patch[hw_ch as usize] != *patch {
                // Packed layout from fm_to_bytes (Flamedriver order: op4,op3,op2,op1):
                //   [0..4]  DT/MUL   → 0x30
                //   [4..8]  RS/AR    → 0x50
                //   [8..12] AM/D1R   → 0x60
                //   [12..16] D2R     → 0x70
                //   [16..20] SL/RR   → 0x80
                //   [20..24] TL      → 0x40 (bit 7 = carrier flag, strip it)
                //   [24]    FB/ALG   → 0xB0
                for (i, &slot_off) in PACKED_OP_SLOTS.iter().enumerate() {
                    let slot = slot_off + ch_offset;
                    self.fm_write(port_base, 0x30 + slot, patch[i], output);
                    self.fm_write(port_base, 0x50 + slot, patch[4 + i], output);
                    self.fm_write(port_base, 0x60 + slot, patch[8 + i], output);
                    self.fm_write(port_base, 0x70 + slot, patch[12 + i], output);
                    self.fm_write(port_base, 0x80 + slot, patch[16 + i], output);
                    self.fm_write(port_base, 0x40 + slot, patch[20 + i] & 0x7F, output);
                }
                self.fm_write(port_base, 0xB0 + ch_offset, patch[24], output);
                self.fm_write(port_base, 0xB4 + ch_offset, 0xC0, output);
                self.last_fm_patch[hw_ch as usize] = *patch;
            }
        }

        let (block, fnum) = midi_to_fm_freq(pitch);
        let freq_msb = (block << 3) | ((fnum >> 8) as u8 & 0x07);
        let freq_lsb = (fnum & 0xFF) as u8;
        self.fm_write(port_base, 0xA4 + ch_offset, freq_msb, output);
        self.fm_write(port_base, 0xA0 + ch_offset, freq_lsb, output);

        let ch_encoded = if hw_ch < 3 { hw_ch } else { hw_ch + 1 };
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 0, data: 0x28 }));
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 1, data: 0xF0 | ch_encoded }));
    }

    fn program_psg(&self, hw_ch: u8, pitch: u8, output: &mut Vec<SequencerOutput>) {
        let period = midi_to_psg_period(pitch);
        let low_nibble = (period & 0x0F) as u8;
        let high_bits = ((period >> 4) & 0x3F) as u8;
        output.push(SequencerOutput::PsgWrite(0x80 | (hw_ch << 5) | low_nibble));
        output.push(SequencerOutput::PsgWrite(high_bits));
        output.push(SequencerOutput::PsgWrite(0x90 | (hw_ch << 5) | 0x00));
    }

    fn program_psg_noise(&self, output: &mut Vec<SequencerOutput>) {
        output.push(SequencerOutput::PsgWrite(0xE0 | 0x04));
        output.push(SequencerOutput::PsgWrite(0x90 | (3 << 5) | 0x00));
    }

    fn key_off_channel(&self, _ch_idx: usize, channel_type: &ChannelType, output: &mut Vec<SequencerOutput>) {
        match channel_type {
            ChannelType::Fm(hw_ch) => {
                let ch_encoded = if *hw_ch < 3 { *hw_ch } else { *hw_ch + 1 };
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 0, data: 0x28 }));
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 1, data: ch_encoded }));
            }
            ChannelType::Psg(hw_ch) => {
                output.push(SequencerOutput::PsgWrite(0x90 | (hw_ch << 5) | 0x0F));
            }
            ChannelType::PsgNoise => {
                output.push(SequencerOutput::PsgWrite(0x90 | (3 << 5) | 0x0F));
            }
            ChannelType::Dac(_) => {}
        }
    }

    fn silence_all(&mut self, output: &mut Vec<SequencerOutput>) {
        for ch_idx in 0..self.active_notes.len() {
            if self.active_notes[ch_idx].is_some() {
                let ct = self.snapshot.channels[ch_idx].channel_type.clone();
                self.key_off_channel(ch_idx, &ct, output);
                self.active_notes[ch_idx] = None;
            }
        }
    }

    fn seek_cursors(&mut self) {
        let tick = self.current_tick as u64;
        for (ch_idx, ch) in self.snapshot.channels.iter().enumerate() {
            self.channel_cursors[ch_idx] = ch
                .events
                .partition_point(|e| e.tick() < tick);
        }
    }

    fn fm_write(&self, port_base: u32, addr: u8, data: u8, output: &mut Vec<SequencerOutput>) {
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: port_base, data: addr }));
        output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: port_base + 1, data }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_fm_snapshot() -> SequencerSnapshot {
        SequencerSnapshot {
            tempo_bpm: 120.0,
            ticks_per_beat: 480,
            loop_start: None,
            loop_end: None,
            channels: vec![ChannelSequence {
                channel_type: ChannelType::Fm(0),
                events: vec![
                    SequencerEvent::NoteOn {
                        tick: 0,
                        pitch: 60,
                        velocity: 100,
                        duration_ticks: 480,
                        instrument: InstrumentData::FmPatch([0; 25]),
                    },
                    SequencerEvent::NoteOff { tick: 480, pitch: 60 },
                ],
                overlaps: vec![],
            }],
        }
    }

    #[test]
    fn test_sequencer_starts_stopped() {
        let seq = Sequencer::new(44100);
        assert!(!seq.is_playing());
        assert_eq!(seq.current_tick_u64(), 0);
    }

    #[test]
    fn test_sequencer_play_advances_tick() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());
        seq.play();
        assert!(seq.is_playing());

        let mut output = Vec::new();
        for _ in 0..1000 {
            seq.advance(&mut output);
        }
        assert!(seq.current_tick_u64() > 0);
    }

    #[test]
    fn test_sequencer_emits_note_on() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());
        seq.play();

        let mut output = Vec::new();
        for _ in 0..100 {
            seq.advance(&mut output);
        }
        let has_fm_write = output.iter().any(|o| matches!(o, SequencerOutput::FmWrite(_)));
        assert!(has_fm_write, "should emit FM register writes for NoteOn");
    }

    #[test]
    fn test_sequencer_stop_silences() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());
        seq.play();

        let mut output = Vec::new();
        for _ in 0..100 {
            seq.advance(&mut output);
        }
        output.clear();

        seq.stop(&mut output);
        assert!(!seq.is_playing());
    }

    #[test]
    fn test_sequencer_seek() {
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(make_fm_snapshot());

        let mut output = Vec::new();
        seq.seek(240, &mut output);
        assert_eq!(seq.current_tick_u64(), 240);
    }

    #[test]
    fn test_sequencer_loop() {
        let mut seq = Sequencer::new(44100);
        let mut snap = make_fm_snapshot();
        snap.loop_start = Some(0);
        snap.loop_end = Some(480);
        seq.load_snapshot(snap);
        seq.play();

        let mut output = Vec::new();
        for _ in 0..50000 {
            seq.advance(&mut output);
        }
        assert!(seq.current_tick_u64() < 480, "should have looped: tick={}", seq.current_tick_u64());
    }
}
