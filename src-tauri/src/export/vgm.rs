use crate::audio::frequency::{midi_to_fm_freq, midi_to_psg_period};
use crate::model::instrument::*;
use crate::model::song::*;

const VGM_SAMPLE_RATE: f64 = 44100.0;
const SN76489_CLOCK: u32 = 3_579_545;
const YM2612_CLOCK: u32 = 7_670_454;
// PACKED_OP_SLOTS comes from `crate::model::instrument` (glob-imported above).

pub struct VgmWriter {
    data: Vec<u8>,
    total_samples: u32,
}

impl VgmWriter {
    pub fn new() -> Self {
        Self {
            data: Vec::with_capacity(64 * 1024),
            total_samples: 0,
        }
    }

    pub fn ym2612_write(&mut self, port: u8, addr: u8, data: u8) {
        self.data.push(if port == 0 { 0x52 } else { 0x53 });
        self.data.push(addr);
        self.data.push(data);
    }

    pub fn sn76489_write(&mut self, data: u8) {
        self.data.push(0x50);
        self.data.push(data);
    }

    pub fn wait(&mut self, samples: u32) {
        self.total_samples += samples;
        let mut remaining = samples;
        while remaining > 0 {
            if remaining >= 735 && remaining < 882 {
                self.data.push(0x62);
                remaining -= 735;
            } else if remaining >= 882 {
                self.data.push(0x63);
                remaining -= 882;
            } else {
                self.data.push(0x61);
                self.data.push((remaining & 0xFF) as u8);
                self.data.push(((remaining >> 8) & 0xFF) as u8);
                remaining = 0;
            }
        }
    }

    pub fn end(&mut self) {
        self.data.push(0x66);
    }

    pub fn build(self) -> Vec<u8> {
        let data_len = self.data.len();
        let total_file_size = 0x40 + data_len;

        let mut out = vec![0u8; 0x40];

        // VGM magic
        out[0] = 0x56; out[1] = 0x67; out[2] = 0x6D; out[3] = 0x20;

        // EOF offset (relative to 0x04)
        let eof_offset = (total_file_size - 4) as u32;
        out[0x04..0x08].copy_from_slice(&eof_offset.to_le_bytes());

        // Version 1.50
        out[0x08..0x0C].copy_from_slice(&0x00000150u32.to_le_bytes());

        // SN76489 clock
        out[0x0C..0x10].copy_from_slice(&SN76489_CLOCK.to_le_bytes());

        // Total samples
        out[0x18..0x1C].copy_from_slice(&self.total_samples.to_le_bytes());

        // SN76489 feedback (0x0009 for Genesis)
        out[0x28..0x2A].copy_from_slice(&0x0009u16.to_le_bytes());
        out[0x2A] = 16; // shift register width
        out[0x2B] = 0;  // flags

        // YM2612 clock
        out[0x2C..0x30].copy_from_slice(&YM2612_CLOCK.to_le_bytes());

        // VGM data offset (relative to 0x34) → data starts at 0x40
        out[0x34..0x38].copy_from_slice(&0x0000000Cu32.to_le_bytes());

        out.extend_from_slice(&self.data);
        out
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ChType { Fm, Psg, PsgNoise }

struct TimelineEvent {
    tick: u64,
    is_on: bool,
    ch_type: ChType,
    hw_ch: u8,
    pitch: u8,
    velocity: u8,
    volume: u8,
    instrument_id: Option<uuid::Uuid>,
    detune: i8,
    pan: u8,
}

fn pan_to_byte(pan: &Pan) -> u8 {
    match pan {
        Pan::Left => 0x80,
        Pan::Right => 0x40,
        Pan::Center => 0xC0,
    }
}

pub fn export_vgm_data(
    song: &Song,
    instruments: &InstrumentBank,
    duration_seconds: Option<f64>,
) -> Result<Vec<u8>, String> {
    let mut events: Vec<TimelineEvent> = Vec::new();

    for track in &song.tracks {
        if track.muted { continue; }
        let (ch_type, hw_ch) = match track.channel {
            ChannelAssignment::Fm(n) => (ChType::Fm, n),
            ChannelAssignment::Psg(n) => (ChType::Psg, n),
            ChannelAssignment::PsgNoise => (ChType::PsgNoise, 3),
            ChannelAssignment::Dac(_) => continue,
        };

        for region in &track.regions {
            for note in &region.notes {
                let abs_tick = region.start_tick + note.tick;
                let end_tick = abs_tick + note.duration_ticks;

                events.push(TimelineEvent {
                    tick: abs_tick,
                    is_on: true,
                    ch_type,
                    hw_ch,
                    pitch: note.pitch,
                    velocity: note.velocity,
                    volume: track.volume,
                    instrument_id: note.instrument_id.or(track.instrument_id),
                    detune: note.detune,
                    pan: note.pan_override.unwrap_or_else(|| pan_to_byte(&track.pan)),
                });
                events.push(TimelineEvent {
                    tick: end_tick,
                    is_on: false,
                    ch_type,
                    hw_ch,
                    pitch: 0,
                    velocity: 0,
                    volume: 0,
                    instrument_id: None,
                    detune: 0,
                    pan: 0,
                });
            }
        }
    }

    // Note-offs before note-ons at the same tick
    events.sort_by(|a, b| a.tick.cmp(&b.tick).then(a.is_on.cmp(&b.is_on)));

    let samples_per_tick = VGM_SAMPLE_RATE * 60.0
        / (song.metadata.tempo * song.metadata.ticks_per_beat as f64);

    let max_sample = duration_seconds
        .map(|d| (d * VGM_SAMPLE_RATE) as u64)
        .unwrap_or(u64::MAX);

    let mut writer = VgmWriter::new();

    // Initialize: enable LR on all FM channels, mute all PSG
    for ch in 0..6u8 {
        let (port, ch_off) = if ch < 3 { (0u8, ch) } else { (1, ch - 3) };
        writer.ym2612_write(port, 0xB4 + ch_off, 0xC0);
    }
    for ch in 0..4u8 {
        writer.sn76489_write(0x90 | (ch << 5) | 0x0F);
    }

    let mut current_sample: f64 = 0.0;
    let mut last_fm_inst: [Option<uuid::Uuid>; 6] = [None; 6];

    for event in &events {
        let target_sample = event.tick as f64 * samples_per_tick;
        if target_sample as u64 > max_sample {
            break;
        }

        let wait = (target_sample - current_sample).round().max(0.0) as u32;
        if wait > 0 {
            writer.wait(wait);
            current_sample += wait as f64;
        }

        match (event.ch_type, event.is_on) {
            (ChType::Fm, true) => {
                program_fm_note(&mut writer, event, instruments, &mut last_fm_inst);
            }
            (ChType::Fm, false) => {
                key_off_fm(&mut writer, event.hw_ch);
            }
            (ChType::Psg, true) => {
                program_psg_note(&mut writer, event);
            }
            (ChType::Psg, false) => {
                mute_psg(&mut writer, event.hw_ch);
            }
            (ChType::PsgNoise, true) => {
                program_psg_noise_note(&mut writer, event);
            }
            (ChType::PsgNoise, false) => {
                mute_psg(&mut writer, 3);
            }
        }
    }

    writer.end();
    Ok(writer.build())
}

fn program_fm_note(
    writer: &mut VgmWriter,
    event: &TimelineEvent,
    instruments: &InstrumentBank,
    last_inst: &mut [Option<uuid::Uuid>; 6],
) {
    let hw_ch = event.hw_ch;
    let (port, ch_off) = if hw_ch < 3 { (0u8, hw_ch) } else { (1, hw_ch - 3) };
    let ch_encoded = if hw_ch < 3 { hw_ch } else { hw_ch + 1 };

    if let Some(inst_id) = event.instrument_id {
        let need_patch = last_inst[hw_ch as usize] != Some(inst_id);
        if need_patch {
            if let Some(inst) = instruments.fm.iter().find(|i| i.id == inst_id) {
                // Key off first
                writer.ym2612_write(0, 0x28, ch_encoded);

                // Release all operators
                for &slot_off in &PACKED_OP_SLOTS {
                    writer.ym2612_write(port, 0x80 + slot_off + ch_off, 0xFF);
                }

                let patch = inst.pack_patch();

                for (i, &slot_off) in PACKED_OP_SLOTS.iter().enumerate() {
                    let slot = slot_off + ch_off;
                    writer.ym2612_write(port, 0x30 + slot, patch[i]);       // DT|MUL
                    writer.ym2612_write(port, 0x50 + slot, patch[4 + i]);   // RS|AR
                    writer.ym2612_write(port, 0x60 + slot, patch[8 + i]);   // AM|D1R
                    writer.ym2612_write(port, 0x70 + slot, patch[12 + i]);  // D2R
                    writer.ym2612_write(port, 0x80 + slot, patch[16 + i]);  // SL|RR
                }

                writer.ym2612_write(port, 0xB0 + ch_off, patch[24]); // FB|ALG

                last_inst[hw_ch as usize] = Some(inst_id);
            }
        }
    }

    // Pan
    writer.ym2612_write(port, 0xB4 + ch_off, event.pan);

    // TL with volume/velocity attenuation
    if let Some(inst_id) = event.instrument_id {
        if let Some(inst) = instruments.fm.iter().find(|i| i.id == inst_id) {
            let patch = inst.pack_patch();
            let vol_offset = 127u16.saturating_sub(event.volume as u16)
                + 127u16.saturating_sub(event.velocity as u16);
            let vol_offset = vol_offset.min(127);

            for (i, &slot_off) in PACKED_OP_SLOTS.iter().enumerate() {
                let slot = slot_off + ch_off;
                let raw_tl = (patch[20 + i] & 0x7F) as u16;
                let is_carrier = patch[20 + i] & 0x80 != 0;
                let tl = if is_carrier {
                    (raw_tl + vol_offset).min(127) as u8
                } else {
                    raw_tl as u8
                };
                writer.ym2612_write(port, 0x40 + slot, tl);
            }
        }
    }

    // Frequency
    let (block, fnum) = midi_to_fm_freq(event.pitch);
    let freq_word = ((block as u16) << 11) | fnum;
    let detuned = (freq_word as i32 + event.detune as i32).clamp(0, 0x3FFF) as u16;
    let freq_msb = (detuned >> 8) as u8;
    let freq_lsb = (detuned & 0xFF) as u8;
    writer.ym2612_write(port, 0xA4 + ch_off, freq_msb);
    writer.ym2612_write(port, 0xA0 + ch_off, freq_lsb);

    // Key on
    writer.ym2612_write(0, 0x28, 0xF0 | ch_encoded);
}

fn key_off_fm(writer: &mut VgmWriter, hw_ch: u8) {
    let ch_encoded = if hw_ch < 3 { hw_ch } else { hw_ch + 1 };
    writer.ym2612_write(0, 0x28, ch_encoded);
}

fn program_psg_note(writer: &mut VgmWriter, event: &TimelineEvent) {
    let period = midi_to_psg_period(event.pitch);
    let detuned = (period as i32 + event.detune as i32).clamp(0, 1023) as u16;
    let ch = event.hw_ch;

    writer.sn76489_write(0x80 | (ch << 5) | (detuned & 0x0F) as u8);
    writer.sn76489_write(((detuned >> 4) & 0x3F) as u8);

    let vol_atten = ((127u16.saturating_sub(event.volume as u16)) * 15 / 127
        + (127u16.saturating_sub(event.velocity as u16)) * 15 / 127).min(15) as u8;
    writer.sn76489_write(0x90 | (ch << 5) | vol_atten);
}

fn program_psg_noise_note(writer: &mut VgmWriter, event: &TimelineEvent) {
    if let Some(_inst_id) = event.instrument_id {
        writer.sn76489_write(0xE0 | 0x04 | 0x03);
    } else {
        writer.sn76489_write(0xE0 | 0x04 | 0x03);
    }

    let vol_atten = ((127u16.saturating_sub(event.volume as u16)) * 15 / 127
        + (127u16.saturating_sub(event.velocity as u16)) * 15 / 127).min(15) as u8;
    writer.sn76489_write(0x90 | (3 << 5) | vol_atten);
}

fn mute_psg(writer: &mut VgmWriter, hw_ch: u8) {
    writer.sn76489_write(0x90 | (hw_ch << 5) | 0x0F);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vgm_writer_builds_valid_header() {
        let mut w = VgmWriter::new();
        w.wait(735);
        w.end();
        let data = w.build();

        assert_eq!(&data[0..4], b"Vgm ");
        let version = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);
        assert_eq!(version, 0x150);
        let sn_clock = u32::from_le_bytes([data[0x0C], data[0x0D], data[0x0E], data[0x0F]]);
        assert_eq!(sn_clock, SN76489_CLOCK);
        let ym_clock = u32::from_le_bytes([data[0x2C], data[0x2D], data[0x2E], data[0x2F]]);
        assert_eq!(ym_clock, YM2612_CLOCK);
    }

    #[test]
    fn test_vgm_writer_wait_uses_ntsc_shortcut() {
        let mut w = VgmWriter::new();
        w.wait(735);
        w.end();
        let data = w.build();
        assert!(data[0x40..].contains(&0x62));
    }

    #[test]
    fn test_vgm_writer_ym2612_command() {
        let mut w = VgmWriter::new();
        w.ym2612_write(0, 0x30, 0x71);
        w.end();
        let data = w.build();
        let cmd_start = 0x40;
        assert_eq!(data[cmd_start], 0x52);
        assert_eq!(data[cmd_start + 1], 0x30);
        assert_eq!(data[cmd_start + 2], 0x71);
    }

    #[test]
    fn test_vgm_writer_psg_command() {
        let mut w = VgmWriter::new();
        w.sn76489_write(0x9F);
        w.end();
        let data = w.build();
        assert_eq!(data[0x40], 0x50);
        assert_eq!(data[0x41], 0x9F);
    }

    #[test]
    fn test_export_empty_song() {
        let song = Song {
            metadata: SongMetadata {
                name: "Empty".into(),
                tempo: 120.0,
                time_signature: (4, 4),
                ticks_per_beat: 480,
                driver_id: "flamedriver".into(),
            },
            tracks: vec![],
            instruments: InstrumentBank::default(),
        };
        let data = export_vgm_data(&song, &song.instruments, Some(1.0)).unwrap();
        assert_eq!(&data[0..4], b"Vgm ");
    }

    #[test]
    fn test_export_single_fm_note() {
        let inst_id = uuid::Uuid::new_v4();
        let song = Song {
            metadata: SongMetadata {
                name: "Test".into(),
                tempo: 120.0,
                time_signature: (4, 4),
                ticks_per_beat: 480,
                driver_id: "flamedriver".into(),
            },
            tracks: vec![Track {
                id: uuid::Uuid::new_v4(),
                name: "FM1".into(),
                channel: ChannelAssignment::Fm(0),
                instrument_id: Some(inst_id),
                regions: vec![Region {
                    id: uuid::Uuid::new_v4(),
                    start_tick: 0,
                    duration_ticks: 480,
                    notes: vec![Note {
                        tick: 0,
                        pitch: 60,
                        velocity: 100,
                        duration_ticks: 480,
                        instrument_id: Some(inst_id),
                        detune: 0,
                        pan_override: None,
                        modulation: None,
                    }],
                    instrument_id: None,
                }],
                muted: false,
                solo: false,
                volume: 127,
                pan: Pan::Center,
                pitch_offset: 0,
                modulation: None,
            }],
            instruments: InstrumentBank {
                fm: vec![FmInstrument {
                    id: inst_id,
                    name: "Test".into(),
                    algorithm: 4,
                    feedback: 5,
                    operators: [FmOperator::default(); 4],
                    metadata: InstrumentMetadata::default(),
                }],
                psg: vec![],
                dac: vec![],
            },
        };

        let data = export_vgm_data(&song, &song.instruments, Some(2.0)).unwrap();
        assert_eq!(&data[0..4], b"Vgm ");
        // Should contain YM2612 writes (0x52)
        assert!(data[0x40..].iter().any(|&b| b == 0x52));
        // Should end with 0x66
        assert_eq!(*data.last().unwrap(), 0x66);
    }
}
