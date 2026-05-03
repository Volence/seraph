pub mod snapshot;

pub use snapshot::*;

use crate::audio::frequency::{midi_to_fm_freq, midi_to_psg_period};
use std::sync::Arc;

// operators[0..3] correspond to SMPS macro op1..op4.
// Flamedriver maps: op1→$3C(slot4), op2→$38(slot2), op3→$34(slot3), op4→$30(slot1).
const PACKED_OP_SLOTS: [u8; 4] = [0x0C, 0x08, 0x04, 0x00];

pub struct Sequencer {
    snapshot: SequencerSnapshot,
    playing: bool,
    current_tick: f64,
    ticks_per_sample: f64,
    sample_rate: f64,
    channel_cursors: Vec<usize>,
    last_fm_patch: [[u8; 25]; 6],
    last_fm_pan: [u8; 6],
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
    PsgEnvelopeStart { hw_ch: u8, envelope: Arc<Vec<u8>>, loop_point: Option<usize>, volume_atten: u8 },
    PsgEnvelopeStop { hw_ch: u8 },
    FmModulationStart { hw_ch: u8, wait: u8, speed: u8, delta: i8, steps: u8, base_freq: u16 },
    FmModulationStop { hw_ch: u8 },
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
            last_fm_pan: [0xFF; 6],
            active_notes: Vec::new(),
        }
    }

    pub fn load_snapshot(&mut self, snapshot: SequencerSnapshot) {
        self.ticks_per_sample =
            (snapshot.tempo_bpm / 60.0) * snapshot.ticks_per_beat as f64 / self.sample_rate;
        self.channel_cursors = vec![0; snapshot.channels.len()];
        self.active_notes = vec![None; snapshot.channels.len()];
        self.last_fm_patch = [[0xFF; 25]; 6];
        self.last_fm_pan = [0xFF; 6];
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
        self.last_fm_patch = [[0xFF; 25]; 6];
        self.last_fm_pan = [0xFF; 6];
    }

    pub fn seek(&mut self, tick: u64, output: &mut Vec<SequencerOutput>) {
        self.current_tick = tick as f64;
        self.silence_all(output);
        self.last_fm_patch = [[0xFF; 25]; 6];
        self.last_fm_pan = [0xFF; 6];
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

    pub fn channel_activity(&self) -> Vec<(ChannelType, bool, u8)> {
        self.snapshot.channels.iter().enumerate().map(|(i, ch)| {
            let active = self.active_notes.get(i).map_or(false, |n| n.is_some());
            (ch.channel_type.clone(), active, ch.volume)
        }).collect()
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
            let volume = self.snapshot.channels[ch_idx].volume;
            for i in pending {
                let event = self.snapshot.channels[ch_idx].events[i].clone();
                self.process_event(ch_idx, &channel_type, volume, &event, output);
            }
        }
    }

    fn process_event(
        &mut self,
        ch_idx: usize,
        channel_type: &ChannelType,
        volume: u8,
        event: &SequencerEvent,
        output: &mut Vec<SequencerOutput>,
    ) {
        match event {
            SequencerEvent::NoteOn { pitch, velocity, instrument, modulation, pan_override, .. } => {
                if self.active_notes[ch_idx].is_some() {
                    self.key_off_channel(ch_idx, channel_type, output);
                }

                match channel_type {
                    ChannelType::Fm(hw_ch) => {
                        let pan = pan_override.unwrap_or(self.snapshot.channels[ch_idx].pan);
                        self.program_fm(*hw_ch, *pitch, volume, *velocity, pan, instrument, output);
                        if let Some(ref mod_params) = modulation {
                            let (block, fnum) = midi_to_fm_freq(*pitch);
                            let freq_msb = (block << 3) | ((fnum >> 8) as u8 & 0x07);
                            let freq_lsb = (fnum & 0xFF) as u8;
                            let base_freq = ((freq_msb as u16) << 8) | freq_lsb as u16;
                            output.push(SequencerOutput::FmModulationStart {
                                hw_ch: *hw_ch,
                                wait: mod_params.wait,
                                speed: mod_params.speed,
                                delta: mod_params.delta as i8,
                                steps: mod_params.steps,
                                base_freq,
                            });
                        } else {
                            output.push(SequencerOutput::FmModulationStop { hw_ch: *hw_ch });
                        }
                    }
                    ChannelType::Psg(hw_ch) => {
                        self.program_psg(*hw_ch, *pitch, volume, instrument, output);
                    }
                    ChannelType::PsgNoise => {
                        self.program_psg_noise(volume, instrument, output);
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

    fn program_fm(&mut self, hw_ch: u8, pitch: u8, volume: u8, velocity: u8, pan: u8, instrument: &InstrumentData, output: &mut Vec<SequencerOutput>) {
        let (port_base, ch_offset) = if hw_ch < 3 { (0u32, hw_ch) } else { (2u32, hw_ch - 3) };

        if let InstrumentData::FmPatch { bytes: patch, ssg_eg } = instrument {
            let patch_changed = self.last_fm_patch[hw_ch as usize] != *patch;

            let ch_encoded = if hw_ch < 3 { hw_ch } else { hw_ch + 1 };

            if patch_changed {
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 0, data: 0x28 }));
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 1, data: ch_encoded }));

                for &slot_off in &PACKED_OP_SLOTS {
                    self.fm_write(port_base, 0x80 + slot_off + ch_offset, 0xFF, output);
                }

                for (i, &slot_off) in PACKED_OP_SLOTS.iter().enumerate() {
                    let slot = slot_off + ch_offset;
                    self.fm_write(port_base, 0x30 + slot, patch[i], output);
                    self.fm_write(port_base, 0x50 + slot, patch[4 + i], output);
                    self.fm_write(port_base, 0x60 + slot, patch[8 + i], output);
                    self.fm_write(port_base, 0x70 + slot, patch[12 + i], output);
                    self.fm_write(port_base, 0x80 + slot, patch[16 + i], output);
                    self.fm_write(port_base, 0x90 + slot, ssg_eg[i], output);
                }
                self.fm_write(port_base, 0xB0 + ch_offset, patch[24], output);
                self.fm_write(port_base, 0xB4 + ch_offset, pan, output);
                self.last_fm_patch[hw_ch as usize] = *patch;
                self.last_fm_pan[hw_ch as usize] = pan;
            } else if self.last_fm_pan[hw_ch as usize] != pan {
                self.fm_write(port_base, 0xB4 + ch_offset, pan, output);
                self.last_fm_pan[hw_ch as usize] = pan;
            }

            let vol_offset = 127u16.saturating_sub(volume as u16)
                + 127u16.saturating_sub(velocity as u16);
            let vol_offset = vol_offset.min(127);

            for (i, &slot_off) in PACKED_OP_SLOTS.iter().enumerate() {
                let slot = slot_off + ch_offset;
                let raw_tl = (patch[20 + i] & 0x7F) as u16;
                let is_carrier = patch[20 + i] & 0x80 != 0;
                let tl = if is_carrier {
                    (raw_tl + vol_offset).min(127) as u8
                } else {
                    raw_tl as u8
                };
                self.fm_write(port_base, 0x40 + slot, tl, output);
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

    fn program_psg(&self, hw_ch: u8, pitch: u8, volume: u8, instrument: &InstrumentData, output: &mut Vec<SequencerOutput>) {
        let period = midi_to_psg_period(pitch);
        let low_nibble = (period & 0x0F) as u8;
        let high_bits = ((period >> 4) & 0x3F) as u8;
        output.push(SequencerOutput::PsgWrite(0x80 | (hw_ch << 5) | low_nibble));
        output.push(SequencerOutput::PsgWrite(high_bits));

        if let InstrumentData::PsgEnvelope { envelope, loop_point, .. } = instrument {
            if !envelope.is_empty() {
                let vol_atten = ((127u16.saturating_sub(volume as u16)) * 15 / 127) as u8;
                output.push(SequencerOutput::PsgEnvelopeStart {
                    hw_ch,
                    envelope: envelope.clone(),
                    loop_point: *loop_point,
                    volume_atten: vol_atten,
                });
                return;
            }
        }
        let atten = ((127u16.saturating_sub(volume as u16)) * 15 / 127) as u8;
        output.push(SequencerOutput::PsgWrite(0x90 | (hw_ch << 5) | (atten & 0x0F)));
    }

    fn program_psg_noise(&self, volume: u8, instrument: &InstrumentData, output: &mut Vec<SequencerOutput>) {
        output.push(SequencerOutput::PsgWrite(0xE0 | 0x04));

        if let InstrumentData::PsgEnvelope { envelope, loop_point, .. } = instrument {
            if !envelope.is_empty() {
                let vol_atten = ((127u16.saturating_sub(volume as u16)) * 15 / 127) as u8;
                output.push(SequencerOutput::PsgEnvelopeStart {
                    hw_ch: 3,
                    envelope: envelope.clone(),
                    loop_point: *loop_point,
                    volume_atten: vol_atten,
                });
                return;
            }
        }
        let atten = ((127u16.saturating_sub(volume as u16)) * 15 / 127) as u8;
        output.push(SequencerOutput::PsgWrite(0x90 | (3 << 5) | (atten & 0x0F)));
    }

    fn key_off_channel(&self, _ch_idx: usize, channel_type: &ChannelType, output: &mut Vec<SequencerOutput>) {
        match channel_type {
            ChannelType::Fm(hw_ch) => {
                let ch_encoded = if *hw_ch < 3 { *hw_ch } else { *hw_ch + 1 };
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 0, data: 0x28 }));
                output.push(SequencerOutput::FmWrite(FmRegisterWrite { port: 1, data: ch_encoded }));
                output.push(SequencerOutput::FmModulationStop { hw_ch: *hw_ch });
            }
            ChannelType::Psg(hw_ch) => {
                output.push(SequencerOutput::PsgEnvelopeStop { hw_ch: *hw_ch });
                output.push(SequencerOutput::PsgWrite(0x90 | (hw_ch << 5) | 0x0F));
            }
            ChannelType::PsgNoise => {
                output.push(SequencerOutput::PsgEnvelopeStop { hw_ch: 3 });
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
                volume: 127,
                pan: 0xC0,
                modulation: None,
                events: vec![
                    SequencerEvent::NoteOn {
                        tick: 0,
                        pitch: 60,
                        velocity: 100,
                        duration_ticks: 480,
                        instrument: InstrumentData::FmPatch { bytes: [0; 25], ssg_eg: [0; 4] },
                        modulation: None,
                        pan_override: None,
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
    fn test_program_fm_register_mapping() {
        let mut seq = Sequencer::new(44100);
        // DEZ1 voice 0: algo=0 fb=2, packed [Op1,Op2,Op3,Op4] (SMPS macro order)
        let mut patch = [0u8; 25];
        // DT/MUL: Op1=(4,1) Op2=(6,4) Op3=(5,0) Op4=(4,5)
        patch[0] = 0x41; patch[1] = 0x64; patch[2] = 0x50; patch[3] = 0x45;
        // RS/AR: Op1=(0,31) Op2=(1,31) Op3=(0,31) Op4=(0,31)
        patch[4] = 0x1F; patch[5] = 0x7F; patch[6] = 0x1F; patch[7] = 0x1F;
        // AM/D1R
        patch[8] = 0x04; patch[9] = 0x08; patch[10] = 0x08; patch[11] = 0x04;
        // D2R
        patch[12] = 0x00; patch[13] = 0x00; patch[14] = 0x0F; patch[15] = 0x00;
        // SL/RR
        patch[16] = 0x18; patch[17] = 0x58; patch[18] = 0x38; patch[19] = 0x18;
        // TL: Op1=0x05|0x80(carrier) Op2=0x1C Op3=0x25 Op4=0x20
        patch[20] = 0x85; patch[21] = 0x1C; patch[22] = 0x25; patch[23] = 0x20;
        // FB/ALG
        patch[24] = 0x10; // fb=2, alg=0

        let inst = InstrumentData::FmPatch { bytes: patch, ssg_eg: [0; 4] };

        let snap = SequencerSnapshot {
            tempo_bpm: 120.0,
            ticks_per_beat: 480,
            loop_start: None,
            loop_end: None,
            channels: vec![ChannelSequence {
                channel_type: ChannelType::Fm(0),
                volume: 127,
                pan: 0xC0,
                modulation: None,
                events: vec![
                    SequencerEvent::NoteOn {
                        tick: 0, pitch: 60, velocity: 127, duration_ticks: 480,
                        instrument: inst,
                        modulation: None,
                        pan_override: None,
                    },
                    SequencerEvent::NoteOff { tick: 480, pitch: 60 },
                ],
                overlaps: vec![],
            }],
        };
        seq.load_snapshot(snap);
        seq.play();

        let mut output = Vec::new();
        for _ in 0..100 { seq.advance(&mut output); }

        let writes: Vec<(u32, u8)> = output.iter().filter_map(|o| match o {
            SequencerOutput::FmWrite(w) => Some((w.port, w.data)),
            _ => None,
        }).collect();

        // Extract addr→data pairs from port 0/1 writes
        let mut regs: Vec<(u8, u8)> = Vec::new();
        let mut i = 0;
        while i + 1 < writes.len() {
            let (p0, addr) = writes[i];
            let (p1, data) = writes[i + 1];
            if (p0 == 0 && p1 == 1) || (p0 == 2 && p1 == 3) {
                regs.push((addr, data));
            }
            i += 2;
        }

        // Verify Flamedriver register mapping for channel 0:
        // Op1 DT/MUL (0x41) → $3C (slot +0x0C)
        assert!(regs.contains(&(0x3C, 0x41)), "Op1 DT/MUL → $3C: {:?}", regs);
        // Op2 DT/MUL (0x64) → $38 (slot +0x08)
        assert!(regs.contains(&(0x38, 0x64)), "Op2 DT/MUL → $38");
        // Op3 DT/MUL (0x50) → $34 (slot +0x04)
        assert!(regs.contains(&(0x34, 0x50)), "Op3 DT/MUL → $34");
        // Op4 DT/MUL (0x45) → $30 (slot +0x00)
        assert!(regs.contains(&(0x30, 0x45)), "Op4 DT/MUL → $30");

        // Op1 TL (carrier, vol=127 → offset=0, raw=5) → $4C
        assert!(regs.contains(&(0x4C, 0x05)), "Op1 TL → $4C: {:?}",
            regs.iter().filter(|(a,_)| *a == 0x4C).collect::<Vec<_>>());
        // Op4 TL (modulator, raw=0x20) → $40
        assert!(regs.contains(&(0x40, 0x20)), "Op4 TL → $40");

        // FB/ALG → $B0
        assert!(regs.contains(&(0xB0, 0x10)), "FB/ALG → $B0");
        // Key-on → $28 with 0xF0 (all ops, ch0)
        assert!(regs.contains(&(0x28, 0xF0)), "Key-on → $28");
    }

    #[test]
    fn test_full_pipeline_smps_to_registers() {
        use crate::driver::flamedriver::FlamedriverProfile;
        use crate::model::driver::DriverProfile;

        let driver = FlamedriverProfile;

        // DEZ1 voice 0 as produced by build_voice_bytes (identity op_order).
        // Packed order: [Op1, Op2, Op3, Op4] matching SMPS macro arg order.
        //   smpsVcDetune     $04, $06, $05, $04  (op1,op2,op3,op4)
        //   smpsVcCoarseFreq $01, $04, $00, $05
        //   smpsVcTotalLevel $05, $1C, $25, $20
        let smps_bytes: [u8; 25] = [
            // DT/MUL: Op1=(4,1), Op2=(6,4), Op3=(5,0), Op4=(4,5)
            0x41, 0x64, 0x50, 0x45,
            // RS/AR
            0x1F, 0x5F, 0x1F, 0x1F,
            // AM/D1R
            0x04, 0x08, 0x08, 0x04,
            // D2R
            0x00, 0x00, 0x0F, 0x00,
            // SL/RR
            0x18, 0x58, 0x38, 0x18,
            // TL (raw, no carrier bits yet)
            0x05, 0x1C, 0x25, 0x20,
            // FB/ALG
            0x10,
        ];

        // Round-trip through fm_from_bytes → fm_to_bytes (simulates import + snapshot build)
        let inst = driver.fm_from_bytes(&smps_bytes).unwrap();
        let patch_bytes = driver.fm_to_bytes(&inst);
        let mut patch = [0u8; 25];
        patch.copy_from_slice(&patch_bytes);

        // Carrier bit should be set on operators[0] (Op1) for algo 0
        assert_eq!(patch[20] & 0x80, 0x80, "Op1 should have carrier bit");
        assert_eq!(patch[23] & 0x80, 0x00, "Op4 should not have carrier bit");

        // Build snapshot and run sequencer
        let ssg = [
            inst.operators[0].ssg_eg,
            inst.operators[1].ssg_eg,
            inst.operators[2].ssg_eg,
            inst.operators[3].ssg_eg,
        ];
        let instrument = InstrumentData::FmPatch { bytes: patch, ssg_eg: ssg };
        let snap = SequencerSnapshot {
            tempo_bpm: 120.0,
            ticks_per_beat: 480,
            loop_start: None,
            loop_end: None,
            channels: vec![ChannelSequence {
                channel_type: ChannelType::Fm(0),
                volume: 112,
                pan: 0xC0,
                modulation: None,
                events: vec![
                    SequencerEvent::NoteOn {
                        tick: 0, pitch: 60, velocity: 100, duration_ticks: 480,
                        instrument,
                        modulation: None,
                        pan_override: None,
                    },
                    SequencerEvent::NoteOff { tick: 480, pitch: 60 },
                ],
                overlaps: vec![],
            }],
        };
        let mut seq = Sequencer::new(44100);
        seq.load_snapshot(snap);
        seq.play();

        let mut output = Vec::new();
        for _ in 0..100 { seq.advance(&mut output); }

        let mut regs: Vec<(u8, u8)> = Vec::new();
        let writes: Vec<(u32, u8)> = output.iter().filter_map(|o| match o {
            SequencerOutput::FmWrite(w) => Some((w.port, w.data)),
            _ => None,
        }).collect();
        let mut i = 0;
        while i + 1 < writes.len() {
            let (p0, addr) = writes[i];
            let (p1, data) = writes[i + 1];
            if (p0 == 0 && p1 == 1) || (p0 == 2 && p1 == 3) {
                regs.push((addr, data));
            }
            i += 2;
        }

        // Flamedriver register mapping: Op1→$3C, Op2→$38, Op3→$34, Op4→$30
        assert!(regs.contains(&(0x3C, 0x41)), "Op1 DT/MUL=0x41 at $3C");
        assert!(regs.contains(&(0x38, 0x64)), "Op2 DT/MUL=0x64 at $38");
        assert!(regs.contains(&(0x34, 0x50)), "Op3 DT/MUL=0x50 at $34");
        assert!(regs.contains(&(0x30, 0x45)), "Op4 DT/MUL=0x45 at $30");

        // Op1 carrier TL: raw=5, vol_offset=(127-112)+(127-100)=42, tl=5+42=47
        assert!(regs.contains(&(0x4C, 47)), "Op1 carrier TL at $4C: {:?}",
            regs.iter().filter(|(a,_)| *a == 0x4C).collect::<Vec<_>>());
        // Op4 modulator TL unchanged: 0x20 = 32
        assert!(regs.contains(&(0x40, 0x20)), "Op4 mod TL at $40");

        assert!(regs.contains(&(0xB0, 0x10)), "FB/ALG=$10 at $B0");
        assert!(regs.contains(&(0x28, 0xF0)), "key-on all ops ch0");
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
