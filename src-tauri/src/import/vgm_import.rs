use std::io::Read as _;
use std::path::Path;
use uuid::Uuid;

use crate::audio::frequency::{fm_freq_to_midi, psg_period_to_midi};
use crate::import::{ImportResult, ImportWarning};
use crate::model::instrument::*;
use crate::model::song::*;

fn decompress_if_gzip(data: &[u8]) -> Result<Vec<u8>, String> {
    if data.len() >= 2 && data[0] == 0x1F && data[1] == 0x8B {
        let mut decoder = flate2::read::GzDecoder::new(data);
        let mut decompressed = Vec::new();
        decoder
            .read_to_end(&mut decompressed)
            .map_err(|e| format!("failed to decompress VGZ: {e}"))?;
        Ok(decompressed)
    } else {
        Ok(data.to_vec())
    }
}

const HW_SLOT_TO_DAW: [usize; 4] = [3, 1, 2, 0];
const CARRIER_MASKS: [u8; 8] = [
    0b0001, 0b0001, 0b0001, 0b0001,
    0b0101, 0b0111, 0b0111, 0b1111,
];
const DEFAULT_BPM: f64 = 150.0;
const DEFAULT_TICKS_PER_BEAT: u32 = 480;
const VGM_SAMPLE_RATE: f64 = 44100.0;
const DAC_GAP_THRESHOLD: u64 = 4410;
// Notes shorter than this are dropped (re-trigger artifacts). ~74 samples ≈ 2 DAW ticks.
const MIN_NOTE_SAMPLES: u64 = 74;

// --- VGM Header ---

struct VgmHeader {
    eof_offset: u32,
    sn76489_clock: u32,
    ym2612_clock: u32,
    gd3_offset: u32,
    #[allow(dead_code)]
    total_samples: u32,
    data_offset: usize,
}

fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

fn parse_vgm_header(data: &[u8]) -> Result<VgmHeader, String> {
    if data.len() < 0x40 {
        return Err("VGM file too small (< 64 bytes)".into());
    }
    if &data[0..4] != b"Vgm " {
        return Err("Not a VGM file (missing 'Vgm ' magic)".into());
    }

    let version = read_u32_le(data, 0x08);

    let data_offset = if version >= 0x0150 {
        let rel = read_u32_le(data, 0x34);
        if rel == 0 {
            0x40
        } else {
            0x34 + rel as usize
        }
    } else {
        0x40
    };

    Ok(VgmHeader {
        eof_offset: read_u32_le(data, 0x04),
        sn76489_clock: read_u32_le(data, 0x0C),
        ym2612_clock: read_u32_le(data, 0x2C),
        gd3_offset: read_u32_le(data, 0x14),
        total_samples: read_u32_le(data, 0x18),
        data_offset,
    })
}

fn parse_gd3_title(data: &[u8], gd3_offset_field: u32) -> Option<String> {
    if gd3_offset_field == 0 {
        return None;
    }
    let abs_offset = 0x14 + gd3_offset_field as usize;
    if abs_offset + 12 > data.len() {
        return None;
    }
    if &data[abs_offset..abs_offset + 4] != b"Gd3 " {
        return None;
    }

    let data_size = read_u32_le(data, abs_offset + 8) as usize;
    let str_start = abs_offset + 12;
    let str_end = (str_start + data_size).min(data.len());
    let str_data = &data[str_start..str_end];

    let mut chars = Vec::new();
    let mut i = 0;
    while i + 1 < str_data.len() {
        let ch = u16::from_le_bytes([str_data[i], str_data[i + 1]]);
        if ch == 0 {
            break;
        }
        chars.push(ch);
        i += 2;
    }

    if chars.is_empty() {
        return None;
    }
    String::from_utf16(&chars).ok().filter(|s| !s.is_empty())
}

// --- Channel State ---

struct FmChannelState {
    dt_mul: [u8; 4],
    tl: [u8; 4],
    rs_ar: [u8; 4],
    am_d1r: [u8; 4],
    d2r: [u8; 4],
    sl_rr: [u8; 4],
    ssg_eg: [u8; 4],
    fb_alg: u8,
    pan_ams_fms: u8, // 0xC0 = center, both speakers
    freq_msb: u8,
    freq_lsb: u8,
    key_on: bool,
    note_start_sample: u64,
    current_midi: u8,
    current_detune: i16,
    current_instrument_idx: Option<usize>,
    current_velocity: u8,
    ever_used: bool,
    pitch_log: Vec<(u64, u16)>,
    initial_freq_word: u16,
}

impl Default for FmChannelState {
    fn default() -> Self {
        Self {
            dt_mul: [0; 4],
            tl: [0; 4],
            rs_ar: [0; 4],
            am_d1r: [0; 4],
            d2r: [0; 4],
            sl_rr: [0; 4],
            ssg_eg: [0; 4],
            fb_alg: 0,
            pan_ams_fms: 0xC0,
            freq_msb: 0,
            freq_lsb: 0,
            key_on: false,
            note_start_sample: 0,
            current_midi: 0,
            current_detune: 0,
            current_instrument_idx: None,
            current_velocity: 0,
            ever_used: false,
            pitch_log: Vec::new(),
            initial_freq_word: 0,
        }
    }
}

struct PsgChannelData {
    period_low: u8,
    period_high: u8,
    full_period: u16,
    volume: u8,
    key_on: bool,
    note_start_sample: u64,
    current_midi: u8,
    ever_used: bool,
}

impl Default for PsgChannelData {
    fn default() -> Self {
        Self {
            period_low: 0,
            period_high: 0,
            full_period: 0,
            volume: 15,
            key_on: false,
            note_start_sample: 0,
            current_midi: 0,
            ever_used: false,
        }
    }
}

struct PsgState {
    channels: [PsgChannelData; 4],
    latched_ch: usize,
    latched_type: u8,
}

impl Default for PsgState {
    fn default() -> Self {
        Self {
            channels: Default::default(),
            latched_ch: 0,
            latched_type: 0,
        }
    }
}

// --- Notes ---

struct RecordedNote {
    channel_type: ChannelType,
    hw_channel: usize,
    midi_note: u8,
    velocity: u8,
    start_sample: u64,
    end_sample: u64,
    detune: i16,
    instrument_idx: Option<usize>,
    pan: u8,
    modulation: Option<NoteModulation>,
}

#[derive(Clone, Copy, PartialEq)]
enum ChannelType {
    Fm,
    Psg,
    PsgNoise,
    Dac,
}

// --- Import State ---

struct ImportState {
    fm_channels: [FmChannelState; 6],
    psg: PsgState,
    dac_enabled: bool,
    instruments: Vec<FmInstrument>,
    patch_map: Vec<[u8; 25]>,
    notes: Vec<RecordedNote>,
    warnings: Vec<ImportWarning>,
    sample_pos: u64,
    lfo_enabled: bool,
    lfo_speed: u8,
    ch3_special_ever: bool,
    dac_ever_enabled: bool,
    dac_note_active: bool,
    dac_note_start: u64,
    last_dac_write: u64,
    dac_had_writes: bool,
}

impl ImportState {
    fn new() -> Self {
        Self {
            fm_channels: Default::default(),
            psg: PsgState::default(),
            dac_enabled: false,
            instruments: Vec::new(),
            patch_map: Vec::new(),
            notes: Vec::new(),
            warnings: Vec::new(),
            sample_pos: 0,
            lfo_enabled: false,
            lfo_speed: 0,
            ch3_special_ever: false,
            dac_ever_enabled: false,
            dac_note_active: false,
            dac_note_start: 0,
            last_dac_write: 0,
            dac_had_writes: false,
        }
    }

    fn push_note(&mut self, note: RecordedNote) {
        if note.end_sample.saturating_sub(note.start_sample) >= MIN_NOTE_SAMPLES {
            self.notes.push(note);
        }
    }

    fn check_dac_gap(&mut self) {
        if self.dac_note_active
            && self.sample_pos.saturating_sub(self.last_dac_write) > DAC_GAP_THRESHOLD
        {
            self.push_note(RecordedNote {
                channel_type: ChannelType::Dac,
                hw_channel: 5,
                midi_note: 60,
                velocity: 127,
                start_sample: self.dac_note_start,
                end_sample: self.last_dac_write,
                detune: 0,
                instrument_idx: None,
                pan: 0xC0,
                modulation: None,
            });
            self.dac_note_active = false;
        }
    }

    fn record_dac_write(&mut self) {
        if self.dac_enabled {
            self.dac_had_writes = true;
            self.last_dac_write = self.sample_pos;
            if !self.dac_note_active {
                self.dac_note_active = true;
                self.dac_note_start = self.sample_pos;
            }
        }
    }

    fn make_fm_note(&self, hw_ch: usize) -> RecordedNote {
        let ch = &self.fm_channels[hw_ch];
        let modulation = if !ch.pitch_log.is_empty() {
            analyze_pitch_log(ch.initial_freq_word, &ch.pitch_log)
        } else if self.lfo_enabled {
            lfo_to_modulation(self.lfo_speed, ch.pan_ams_fms & 0x07)
        } else {
            None
        };

        RecordedNote {
            channel_type: ChannelType::Fm,
            hw_channel: hw_ch,
            midi_note: ch.current_midi,
            velocity: ch.current_velocity,
            start_sample: ch.note_start_sample,
            end_sample: self.sample_pos,
            detune: ch.current_detune,
            instrument_idx: ch.current_instrument_idx,
            pan: ch.pan_ams_fms,
            modulation,
        }
    }

    // --- YM2612 ---

    fn process_ym2612(&mut self, port: u8, addr: u8, val: u8) {
        if port == 0 && addr == 0x2A {
            self.record_dac_write();
            return;
        }
        if port == 0 && addr == 0x2B {
            let enabled = val & 0x80 != 0;
            if enabled {
                self.dac_ever_enabled = true;
            }
            self.dac_enabled = enabled;
            return;
        }
        if port == 0 && addr == 0x22 {
            self.lfo_enabled = val & 0x08 != 0;
            self.lfo_speed = val & 0x07;
            return;
        }
        if port == 0 && addr == 0x27 {
            if (val >> 6) & 0x03 != 0 {
                self.ch3_special_ever = true;
            }
            return;
        }
        if port == 0 && addr == 0x28 {
            self.process_key_on(val);
            return;
        }

        if (0x30..=0x9F).contains(&addr) {
            let ch_in_port = (addr & 0x03) as usize;
            if ch_in_port == 3 {
                return;
            }
            let hw_ch = if port == 0 {
                ch_in_port
            } else {
                ch_in_port + 3
            };
            let slot = ((addr >> 2) & 0x03) as usize;

            match addr & 0xF0 {
                0x30 => self.fm_channels[hw_ch].dt_mul[slot] = val,
                0x40 => self.fm_channels[hw_ch].tl[slot] = val,
                0x50 => self.fm_channels[hw_ch].rs_ar[slot] = val,
                0x60 => self.fm_channels[hw_ch].am_d1r[slot] = val,
                0x70 => self.fm_channels[hw_ch].d2r[slot] = val,
                0x80 => self.fm_channels[hw_ch].sl_rr[slot] = val,
                0x90 => self.fm_channels[hw_ch].ssg_eg[slot] = val,
                _ => {}
            }
            return;
        }

        if (0xA0..=0xBF).contains(&addr) {
            let ch_in_port = (addr & 0x03) as usize;
            if ch_in_port == 3 {
                return;
            }
            let hw_ch = if port == 0 {
                ch_in_port
            } else {
                ch_in_port + 3
            };

            match addr & 0xFC {
                0xA0 => {
                    self.fm_channels[hw_ch].freq_lsb = val;
                    if self.fm_channels[hw_ch].key_on {
                        let fw = ((self.fm_channels[hw_ch].freq_msb as u16 & 0x3F) << 8)
                            | val as u16;
                        let sp = self.sample_pos;
                        self.fm_channels[hw_ch].pitch_log.push((sp, fw));
                    }
                }
                0xA4 => {
                    self.fm_channels[hw_ch].freq_msb = val;
                }
                0xB0 => self.fm_channels[hw_ch].fb_alg = val,
                0xB4 => self.fm_channels[hw_ch].pan_ams_fms = val,
                _ => {}
            }
        }
    }

    fn process_key_on(&mut self, val: u8) {
        let ch_raw = val & 0x07;
        let hw_ch = match ch_raw {
            0..=2 => ch_raw as usize,
            4..=6 => (ch_raw - 1) as usize,
            _ => return,
        };

        if hw_ch == 5 && self.dac_enabled {
            return;
        }

        let op_mask = (val >> 4) & 0x0F;
        let new_key_on = op_mask != 0;

        if self.fm_channels[hw_ch].key_on && !new_key_on {
            let note = self.make_fm_note(hw_ch);
            self.push_note(note);
            self.fm_channels[hw_ch].key_on = false;
            return;
        }

        if !new_key_on {
            return;
        }

        if self.fm_channels[hw_ch].key_on {
            let note = self.make_fm_note(hw_ch);
            self.push_note(note);
        }

        let (inst, velocity) =
            snapshot_fm_instrument(&self.fm_channels[hw_ch], self.instruments.len());
        let packed = inst.pack_patch();
        let inst_idx =
            if let Some(existing) = self.patch_map.iter().position(|p| p == &packed) {
                existing
            } else {
                let idx = self.instruments.len();
                self.patch_map.push(packed);
                self.instruments.push(inst);
                idx
            };

        let freq_word = ((self.fm_channels[hw_ch].freq_msb as u16 & 0x3F) << 8)
            | self.fm_channels[hw_ch].freq_lsb as u16;
        let block = ((freq_word >> 11) & 0x07) as u8;
        let fnum = freq_word & 0x07FF;
        let (midi, detune) = fm_freq_to_midi(block, fnum);

        let ch = &mut self.fm_channels[hw_ch];
        ch.key_on = true;
        ch.note_start_sample = self.sample_pos;
        ch.current_midi = midi;
        ch.current_detune = detune;
        ch.current_instrument_idx = Some(inst_idx);
        ch.current_velocity = velocity;
        ch.ever_used = true;
        ch.pitch_log.clear();
        ch.initial_freq_word = freq_word;
    }

    // --- PSG ---

    fn process_psg(&mut self, val: u8) {
        if val & 0x80 != 0 {
            let ch = ((val >> 5) & 0x03) as usize;
            let reg_type = (val >> 4) & 0x01;
            let data_bits = val & 0x0F;

            self.psg.latched_ch = ch;
            self.psg.latched_type = reg_type;

            if reg_type == 1 {
                let old_vol = self.psg.channels[ch].volume;
                self.psg.channels[ch].volume = data_bits;
                self.psg_volume_changed(ch, old_vol, data_bits);
            } else {
                self.psg.channels[ch].period_low = data_bits;
                if ch < 3 {
                    let period =
                        (self.psg.channels[ch].period_high as u16) << 4 | data_bits as u16;
                    self.psg_period_changed(ch, period);
                }
            }
        } else {
            let ch = self.psg.latched_ch;
            let data_bits = val & 0x3F;

            if self.psg.latched_type == 1 {
                let old_vol = self.psg.channels[ch].volume;
                let new_vol = data_bits & 0x0F;
                self.psg.channels[ch].volume = new_vol;
                self.psg_volume_changed(ch, old_vol, new_vol);
            } else if ch < 3 {
                self.psg.channels[ch].period_high = data_bits;
                let period =
                    (data_bits as u16) << 4 | self.psg.channels[ch].period_low as u16;
                self.psg_period_changed(ch, period);
            }
        }
    }

    fn psg_volume_changed(&mut self, ch: usize, old_vol: u8, new_vol: u8) {
        let was_on = old_vol < 15;
        let is_on = new_vol < 15;

        if was_on && !is_on && self.psg.channels[ch].key_on {
            let vel = psg_vol_to_velocity(old_vol);
            self.push_note(RecordedNote {
                channel_type: if ch == 3 {
                    ChannelType::PsgNoise
                } else {
                    ChannelType::Psg
                },
                hw_channel: ch,
                midi_note: self.psg.channels[ch].current_midi,
                velocity: vel,
                start_sample: self.psg.channels[ch].note_start_sample,
                end_sample: self.sample_pos,
                detune: 0,
                instrument_idx: None,
                pan: 0xC0,
                modulation: None,
            });
            self.psg.channels[ch].key_on = false;
        } else if !was_on && is_on {
            let midi = if ch < 3 {
                psg_period_to_midi(self.psg.channels[ch].full_period)
            } else {
                60
            };
            self.psg.channels[ch].key_on = true;
            self.psg.channels[ch].note_start_sample = self.sample_pos;
            self.psg.channels[ch].current_midi = midi;
            self.psg.channels[ch].ever_used = true;
        }
    }

    fn psg_period_changed(&mut self, ch: usize, new_period: u16) {
        self.psg.channels[ch].full_period = new_period;
        if !self.psg.channels[ch].key_on || ch >= 3 {
            return;
        }
        let new_midi = psg_period_to_midi(new_period);
        if new_midi != self.psg.channels[ch].current_midi {
            let vel = psg_vol_to_velocity(self.psg.channels[ch].volume);
            self.push_note(RecordedNote {
                channel_type: ChannelType::Psg,
                hw_channel: ch,
                midi_note: self.psg.channels[ch].current_midi,
                velocity: vel,
                start_sample: self.psg.channels[ch].note_start_sample,
                end_sample: self.sample_pos,
                detune: 0,
                instrument_idx: None,
                pan: 0xC0,
                modulation: None,
            });
            self.psg.channels[ch].note_start_sample = self.sample_pos;
            self.psg.channels[ch].current_midi = new_midi;
        }
    }

    // --- Finalization ---

    fn close_open_notes(&mut self) {
        for ch_idx in 0..6 {
            if self.fm_channels[ch_idx].key_on {
                let note = self.make_fm_note(ch_idx);
                self.push_note(note);
                self.fm_channels[ch_idx].key_on = false;
            }
        }
        for ch_idx in 0..4 {
            if self.psg.channels[ch_idx].key_on {
                let vel = psg_vol_to_velocity(self.psg.channels[ch_idx].volume);
                self.push_note(RecordedNote {
                    channel_type: if ch_idx == 3 {
                        ChannelType::PsgNoise
                    } else {
                        ChannelType::Psg
                    },
                    hw_channel: ch_idx,
                    midi_note: self.psg.channels[ch_idx].current_midi,
                    velocity: vel,
                    start_sample: self.psg.channels[ch_idx].note_start_sample,
                    end_sample: self.sample_pos,
                    detune: 0,
                    instrument_idx: None,
                    pan: 0xC0,
                    modulation: None,
                });
                self.psg.channels[ch_idx].key_on = false;
            }
        }
        if self.dac_note_active {
            self.push_note(RecordedNote {
                channel_type: ChannelType::Dac,
                hw_channel: 5,
                midi_note: 60,
                velocity: 127,
                start_sample: self.dac_note_start,
                end_sample: self.sample_pos,
                detune: 0,
                instrument_idx: None,
                pan: 0xC0,
                modulation: None,
            });
            self.dac_note_active = false;
        }
    }
}

// --- Helpers ---

fn psg_vol_to_velocity(vol: u8) -> u8 {
    ((15u16.saturating_sub(vol as u16)) * 127 / 15).max(1) as u8
}

fn snapshot_fm_instrument(
    ch: &FmChannelState,
    instrument_count: usize,
) -> (FmInstrument, u8) {
    let algorithm = ch.fb_alg & 0x07;
    let feedback = (ch.fb_alg >> 3) & 0x07;
    let carrier_mask = CARRIER_MASKS[algorithm as usize];

    let mut min_carrier_tl: u8 = 127;
    for slot in 0..4 {
        let daw_op = HW_SLOT_TO_DAW[slot];
        let tl = ch.tl[slot] & 0x7F;
        if (carrier_mask >> daw_op) & 1 == 1 {
            min_carrier_tl = min_carrier_tl.min(tl);
        }
    }

    let mut operators = [FmOperator::default(); 4];
    for slot in 0..4 {
        let daw_op = HW_SLOT_TO_DAW[slot];
        let tl = ch.tl[slot] & 0x7F;
        let is_carrier = (carrier_mask >> daw_op) & 1 == 1;

        operators[daw_op] = FmOperator {
            detune: (ch.dt_mul[slot] >> 4) & 0x07,
            multiple: ch.dt_mul[slot] & 0x0F,
            rate_scale: (ch.rs_ar[slot] >> 6) & 0x03,
            attack_rate: ch.rs_ar[slot] & 0x1F,
            amp_mod: ch.am_d1r[slot] & 0x80 != 0,
            d1r: ch.am_d1r[slot] & 0x1F,
            d2r: ch.d2r[slot] & 0x1F,
            sustain_level: (ch.sl_rr[slot] >> 4) & 0x0F,
            release_rate: ch.sl_rr[slot] & 0x0F,
            total_level: if is_carrier { tl - min_carrier_tl } else { tl },
            ssg_eg: ch.ssg_eg[slot] & 0x0F,
        };
    }

    let velocity = 127u8.saturating_sub(min_carrier_tl).max(1);

    (
        FmInstrument {
            id: Uuid::new_v4(),
            name: format!("FM Patch {}", instrument_count + 1),
            algorithm,
            feedback,
            operators,
            metadata: InstrumentMetadata {
                category: "VGM Import".into(),
                ..Default::default()
            },
        },
        velocity,
    )
}

fn lfo_to_modulation(lfo_speed: u8, fms: u8) -> Option<NoteModulation> {
    if fms == 0 {
        return None;
    }

    let (speed, steps) = match lfo_speed {
        0 => (2u8, 4u8),
        1 => (1, 5),
        2 => (1, 5),
        3 => (1, 5),
        4 => (1, 4),
        5 => (1, 3),
        _ => (1, 1),
    };

    let delta = match fms {
        1 => 1u8,
        2 => 1,
        3 => 2,
        4 => 3,
        5 => 4,
        6 => 8,
        7 => 16,
        _ => return None,
    };

    Some(NoteModulation {
        wait: 1,
        speed,
        delta,
        steps,
    })
}

fn analyze_pitch_log(initial: u16, log: &[(u64, u16)]) -> Option<NoteModulation> {
    if log.len() < 4 {
        return None;
    }

    let init = initial as i32;
    let deviations: Vec<i32> = log.iter().map(|(_, f)| *f as i32 - init).collect();

    let sign_changes = deviations
        .windows(2)
        .filter(|w| (w[0] > 0) != (w[1] > 0))
        .count();

    if sign_changes < 2 {
        return None;
    }

    let total_time = log.last()?.0.saturating_sub(log.first()?.0);
    if total_time == 0 {
        return None;
    }

    let cycles = sign_changes as f64 / 2.0;
    let period_samples = total_time as f64 / cycles;
    let vibrato_hz = VGM_SAMPLE_RATE / period_samples;

    let product = (30.0 / vibrato_hz).max(1.0);
    let (speed, steps) = if product <= 2.0 {
        (1u8, 1u8)
    } else if product <= 6.0 {
        (1, product.round() as u8)
    } else if product <= 12.0 {
        (2, (product / 2.0).round() as u8)
    } else {
        (3, (product / 3.0).round().min(255.0) as u8)
    };

    let max_dev = deviations.iter().map(|d| d.unsigned_abs()).max().unwrap_or(0);
    let delta = if steps > 0 {
        ((max_dev / steps as u32) as u8).max(1)
    } else {
        1
    };

    Some(NoteModulation {
        wait: 1,
        speed: speed.max(1),
        delta,
        steps: steps.max(1),
    })
}

// --- Public API ---

#[derive(Debug)]
pub struct VgmImportData {
    pub title: String,
    pub song: Song,
    pub warnings: Vec<ImportWarning>,
}

pub fn import_vgm_data(data: &[u8]) -> Result<VgmImportData, String> {
    let data = decompress_if_gzip(data)?;
    let data = data.as_slice();
    let header = parse_vgm_header(data)?;

    if header.ym2612_clock == 0 && header.sn76489_clock == 0 {
        return Err(
            "VGM file has no YM2612 or SN76489 — not a Genesis/Mega Drive VGM".into(),
        );
    }

    let title = parse_gd3_title(data, header.gd3_offset).unwrap_or_else(|| "VGM Import".into());

    let mut state = ImportState::new();
    let end_offset = (0x04 + header.eof_offset as usize).min(data.len());
    let mut pos = header.data_offset;

    while pos < end_offset {
        if pos >= data.len() {
            break;
        }
        let cmd = data[pos];
        pos += 1;

        match cmd {
            0x52 => {
                if pos + 1 >= data.len() {
                    break;
                }
                let addr = data[pos];
                let val = data[pos + 1];
                pos += 2;
                state.process_ym2612(0, addr, val);
            }
            0x53 => {
                if pos + 1 >= data.len() {
                    break;
                }
                let addr = data[pos];
                let val = data[pos + 1];
                pos += 2;
                state.process_ym2612(1, addr, val);
            }
            0x50 => {
                if pos >= data.len() {
                    break;
                }
                let val = data[pos];
                pos += 1;
                state.process_psg(val);
            }
            0x61 => {
                if pos + 1 >= data.len() {
                    break;
                }
                let n = u16::from_le_bytes([data[pos], data[pos + 1]]);
                pos += 2;
                state.sample_pos += n as u64;
                state.check_dac_gap();
            }
            0x62 => {
                state.sample_pos += 735;
                state.check_dac_gap();
            }
            0x63 => {
                state.sample_pos += 882;
                state.check_dac_gap();
            }
            0x66 => break,
            0x67 => {
                if pos + 6 > data.len() {
                    break;
                }
                pos += 2;
                let size = read_u32_le(data, pos) as usize;
                pos += 4 + size;
            }
            0x70..=0x7F => {
                state.sample_pos += (cmd & 0x0F) as u64 + 1;
                state.check_dac_gap();
            }
            0x80..=0x8F => {
                state.record_dac_write();
                state.sample_pos += (cmd & 0x0F) as u64;
            }
            0xE0 => {
                if pos + 4 > data.len() {
                    break;
                }
                pos += 4;
            }
            _ => match cmd {
                0x30..=0x4F => {
                    pos += 1;
                }
                0x51 | 0x54..=0x5F => {
                    pos += 2;
                }
                0xA0..=0xBF => {
                    pos += 2;
                }
                0xC0..=0xDF => {
                    pos += 3;
                }
                0xE1..=0xFF => {
                    pos += 4;
                }
                _ => {}
            },
        }
    }

    state.close_open_notes();

    if state.dac_ever_enabled {
        let msg = if state.dac_had_writes {
            "DAC/sample playback detected \u{2014} placeholder notes created but samples cannot be reconstructed from VGM"
        } else {
            "DAC mode enabled but no sample data detected"
        };
        state.warnings.push(ImportWarning {
            channel: "DAC".into(),
            message: msg.into(),
        });
    }
    if header.ym2612_clock == 0 {
        state.warnings.push(ImportWarning {
            channel: "FM".into(),
            message: "No YM2612 chip detected \u{2014} FM channels will be empty".into(),
        });
    }
    if header.sn76489_clock == 0 {
        state.warnings.push(ImportWarning {
            channel: "PSG".into(),
            message: "No SN76489 chip detected \u{2014} PSG channels will be empty".into(),
        });
    }
    if state.ch3_special_ever {
        state.warnings.push(ImportWarning {
            channel: "FM3".into(),
            message:
                "Channel 3 special mode detected \u{2014} operator frequencies may not be fully captured"
                    .into(),
        });
    }
    if state.lfo_enabled {
        state.warnings.push(ImportWarning {
            channel: "FM".into(),
            message:
                "Hardware LFO detected \u{2014} vibrato approximated via software modulation"
                    .into(),
        });
    }

    let samples_per_tick =
        VGM_SAMPLE_RATE * 60.0 / (DEFAULT_BPM * DEFAULT_TICKS_PER_BEAT as f64);

    let metadata = SongMetadata {
        name: title.clone(),
        tempo: DEFAULT_BPM,
        time_signature: (4, 4),
        ticks_per_beat: DEFAULT_TICKS_PER_BEAT,
        driver_id: "flamedriver".into(),
    };

    let psg_instruments = build_psg_instruments(&state.psg);
    let tracks = build_tracks(
        &state.fm_channels,
        &state.psg,
        &state.notes,
        &state.instruments,
        &psg_instruments,
        samples_per_tick,
    );

    Ok(VgmImportData {
        title,
        song: Song {
            metadata,
            tracks,
            instruments: InstrumentBank {
                fm: state.instruments,
                psg: psg_instruments,
                dac: vec![],
            },
        },
        warnings: state.warnings,
    })
}

// --- Track building ---

fn build_psg_instruments(psg: &PsgState) -> Vec<PsgInstrument> {
    let mut out = Vec::new();
    for ch_idx in 0..3 {
        if psg.channels[ch_idx].ever_used {
            out.push(PsgInstrument {
                id: Uuid::new_v4(),
                name: format!("PSG {}", ch_idx + 1),
                volume_sequence: vec![15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
                loop_point: None,
                silence_on_end: true,
                noise_mode: None,
                smps_envelope_index: None,
                metadata: InstrumentMetadata::default(),
            });
        }
    }
    if psg.channels[3].ever_used {
        out.push(PsgInstrument {
            id: Uuid::new_v4(),
            name: "PSG Noise".into(),
            volume_sequence: vec![15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
            loop_point: None,
            silence_on_end: true,
            noise_mode: Some(NoiseMode::White(0)),
            smps_envelope_index: None,
            metadata: InstrumentMetadata::default(),
        });
    }
    out
}

fn build_tracks(
    fm_channels: &[FmChannelState; 6],
    psg: &PsgState,
    notes: &[RecordedNote],
    instruments: &[FmInstrument],
    psg_instruments: &[PsgInstrument],
    samples_per_tick: f64,
) -> Vec<Track> {
    let mut tracks = Vec::new();

    for ch_idx in 0..6 {
        if !fm_channels[ch_idx].ever_used {
            continue;
        }
        let ch_notes: Vec<Note> = notes
            .iter()
            .filter(|n| n.channel_type == ChannelType::Fm && n.hw_channel == ch_idx)
            .map(|n| sample_to_note(n, instruments, samples_per_tick))
            .collect();
        if ch_notes.is_empty() {
            continue;
        }
        let last_tick = ch_notes
            .iter()
            .map(|n| n.tick + n.duration_ticks)
            .max()
            .unwrap_or(0);
        let initial_instrument = ch_notes.first().and_then(|n| n.instrument_id);
        tracks.push(Track {
            id: Uuid::new_v4(),
            name: format!("FM{}", ch_idx + 1),
            channel: ChannelAssignment::Fm(ch_idx as u8),
            instrument_id: initial_instrument,
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: last_tick,
                notes: ch_notes,
                instrument_id: None,
            }],
            muted: false,
            solo: false,
            volume: 127,
            pan: Pan::Center,
            pitch_offset: 0,
            modulation: None,
        });
    }

    for ch_idx in 0..3 {
        if !psg.channels[ch_idx].ever_used {
            continue;
        }
        let psg_inst = psg_instruments
            .iter()
            .find(|p| p.name == format!("PSG {}", ch_idx + 1));
        let ch_notes: Vec<Note> = notes
            .iter()
            .filter(|n| n.channel_type == ChannelType::Psg && n.hw_channel == ch_idx)
            .map(|n| {
                let mut note = sample_to_note(n, &[], samples_per_tick);
                note.instrument_id = psg_inst.map(|p| p.id);
                note
            })
            .collect();
        if ch_notes.is_empty() {
            continue;
        }
        let last_tick = ch_notes
            .iter()
            .map(|n| n.tick + n.duration_ticks)
            .max()
            .unwrap_or(0);
        tracks.push(Track {
            id: Uuid::new_v4(),
            name: format!("PSG{}", ch_idx + 1),
            channel: ChannelAssignment::Psg(ch_idx as u8),
            instrument_id: psg_inst.map(|p| p.id),
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: last_tick,
                notes: ch_notes,
                instrument_id: None,
            }],
            muted: false,
            solo: false,
            volume: 127,
            pan: Pan::Center,
            pitch_offset: 0,
            modulation: None,
        });
    }

    if psg.channels[3].ever_used {
        let psg_inst = psg_instruments.iter().find(|p| p.name == "PSG Noise");
        let ch_notes: Vec<Note> = notes
            .iter()
            .filter(|n| n.channel_type == ChannelType::PsgNoise && n.hw_channel == 3)
            .map(|n| {
                let mut note = sample_to_note(n, &[], samples_per_tick);
                note.instrument_id = psg_inst.map(|p| p.id);
                note
            })
            .collect();
        if !ch_notes.is_empty() {
            let last_tick = ch_notes
                .iter()
                .map(|n| n.tick + n.duration_ticks)
                .max()
                .unwrap_or(0);
            tracks.push(Track {
                id: Uuid::new_v4(),
                name: "PSG Noise".into(),
                channel: ChannelAssignment::PsgNoise,
                instrument_id: psg_inst.map(|p| p.id),
                regions: vec![Region {
                    id: Uuid::new_v4(),
                    start_tick: 0,
                    duration_ticks: last_tick,
                    notes: ch_notes,
                    instrument_id: None,
                }],
                muted: false,
                solo: false,
                volume: 127,
                pan: Pan::Center,
                pitch_offset: 0,
                modulation: None,
            });
        }
    }

    let dac_notes: Vec<Note> = notes
        .iter()
        .filter(|n| n.channel_type == ChannelType::Dac)
        .map(|n| sample_to_note(n, &[], samples_per_tick))
        .collect();
    if !dac_notes.is_empty() {
        let last_tick = dac_notes
            .iter()
            .map(|n| n.tick + n.duration_ticks)
            .max()
            .unwrap_or(0);
        tracks.push(Track {
            id: Uuid::new_v4(),
            name: "DAC".into(),
            channel: ChannelAssignment::Dac(0),
            instrument_id: None,
            regions: vec![Region {
                id: Uuid::new_v4(),
                start_tick: 0,
                duration_ticks: last_tick,
                notes: dac_notes,
                instrument_id: None,
            }],
            muted: false,
            solo: false,
            volume: 127,
            pan: Pan::Center,
            pitch_offset: 0,
            modulation: None,
        });
    }

    tracks
}

fn sample_to_note(
    n: &RecordedNote,
    instruments: &[FmInstrument],
    samples_per_tick: f64,
) -> Note {
    let start_tick = (n.start_sample as f64 / samples_per_tick).round() as u64;
    let end_tick = (n.end_sample as f64 / samples_per_tick).round() as u64;
    let duration = end_tick.saturating_sub(start_tick).max(1);

    let pan_override = if n.pan == 0xC0 { None } else { Some(n.pan) };

    Note {
        tick: start_tick,
        pitch: n.midi_note,
        velocity: n.velocity,
        duration_ticks: duration,
        instrument_id: n
            .instrument_idx
            .and_then(|idx| instruments.get(idx).map(|i| i.id)),
        detune: n.detune.clamp(-128, 127) as i8,
        pan_override,
        modulation: n.modulation.clone(),
    }
}

// --- Disk I/O ---

pub fn import_vgm_file(
    vgm_path: &Path,
    parent_dir: &Path,
    recognition: &crate::import::RecognitionTable,
) -> Result<ImportResult, String> {
    let data = std::fs::read(vgm_path)
        .map_err(|e| format!("failed to read {}: {e}", vgm_path.display()))?;

    let import = import_vgm_data(&data)?;
    let mut song = import.song;
    crate::import::recognize_instruments(&mut song.instruments, recognition);

    let dir_name = vgm_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("VGM_Import");
    let project_dir = parent_dir.join(dir_name);

    std::fs::create_dir_all(&project_dir)
        .map_err(|e| format!("create dir {}: {e}", project_dir.display()))?;
    std::fs::create_dir_all(project_dir.join("instruments/fm")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/psg")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("instruments/dac")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(project_dir.join("exports")).map_err(|e| e.to_string())?;

    let version = serde_json::json!({ "version": "0.1.0" });
    std::fs::write(
        project_dir.join(".seraph"),
        serde_json::to_string_pretty(&version).unwrap(),
    )
    .map_err(|e| e.to_string())?;

    let project_file = ProjectFile {
        metadata: song.metadata.clone(),
        tracks: song.tracks.clone(),
    };
    let json = serde_json::to_string_pretty(&project_file).map_err(|e| e.to_string())?;
    std::fs::write(project_dir.join("project.json"), json).map_err(|e| e.to_string())?;

    let mut instrument_count = 0;
    for inst in &song.instruments.fm {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/fm/{}.json", inst.id)),
            json,
        )
        .map_err(|e| e.to_string())?;
        instrument_count += 1;
    }
    for inst in &song.instruments.psg {
        let json = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
        std::fs::write(
            project_dir.join(format!("instruments/psg/{}.json", inst.id)),
            json,
        )
        .map_err(|e| e.to_string())?;
        instrument_count += 1;
    }

    Ok(ImportResult {
        project_dir: project_dir.to_string_lossy().into_owned(),
        metadata: song.metadata,
        track_count: song.tracks.len(),
        instrument_count,
        warnings: import.warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_vgm_header(ym_clock: u32, psg_clock: u32) -> Vec<u8> {
        let mut data = vec![0u8; 0x40];
        data[0..4].copy_from_slice(b"Vgm ");
        data[8] = 0x50;
        data[9] = 0x01; // version 1.50
        data[0x0C..0x10].copy_from_slice(&psg_clock.to_le_bytes());
        data[0x2C..0x30].copy_from_slice(&ym_clock.to_le_bytes());
        data[0x34..0x38].copy_from_slice(&0x0Cu32.to_le_bytes()); // data at 0x40
        data
    }

    fn finalize_vgm(data: &mut Vec<u8>, total_samples: u32) {
        data.push(0x66); // end marker
        let eof = (data.len() - 4) as u32;
        data[4..8].copy_from_slice(&eof.to_le_bytes());
        data[0x18..0x1C].copy_from_slice(&total_samples.to_le_bytes());
    }

    fn ym_write(data: &mut Vec<u8>, port: u8, addr: u8, val: u8) {
        data.push(if port == 0 { 0x52 } else { 0x53 });
        data.push(addr);
        data.push(val);
    }

    fn ym_write_op_reg(data: &mut Vec<u8>, port: u8, base: u8, ch: u8, slot: u8, val: u8) {
        let addr = base + ch + slot * 4;
        ym_write(data, port, addr, val);
    }

    fn wait_samples(data: &mut Vec<u8>, samples: u16) {
        data.push(0x61);
        data.extend_from_slice(&samples.to_le_bytes());
    }

    fn program_fm_ch(data: &mut Vec<u8>, port: u8, ch: u8, tl: u8, fb_alg: u8) {
        for slot in 0..4u8 {
            ym_write_op_reg(data, port, 0x30, ch, slot, 0x71);
            ym_write_op_reg(data, port, 0x40, ch, slot, tl);
            ym_write_op_reg(data, port, 0x50, ch, slot, 0x1F);
            ym_write_op_reg(data, port, 0x60, ch, slot, 0x05);
            ym_write_op_reg(data, port, 0x70, ch, slot, 0x02);
            ym_write_op_reg(data, port, 0x80, ch, slot, 0x1A);
        }
        ym_write(data, port, 0xB0 + ch, fb_alg);
        ym_write(data, port, 0xB4 + ch, 0xC0);
    }

    fn make_fm_note_vgm() -> Vec<u8> {
        let mut data = make_vgm_header(7_670_454, 3_579_545);
        program_fm_ch(&mut data, 0, 0, 0x20, 0x32);
        ym_write(&mut data, 0, 0xA4, 0x24);
        ym_write(&mut data, 0, 0xA0, 0x3C);
        ym_write(&mut data, 0, 0x28, 0xF0);
        wait_samples(&mut data, 44100);
        ym_write(&mut data, 0, 0x28, 0x00);
        finalize_vgm(&mut data, 44100);
        data
    }

    #[test]
    fn test_vgm_header_parsing() {
        let data = make_fm_note_vgm();
        let header = parse_vgm_header(&data).unwrap();
        assert_eq!(header.ym2612_clock, 7_670_454);
        assert_eq!(header.sn76489_clock, 3_579_545);
        assert_eq!(header.data_offset, 0x40);
    }

    #[test]
    fn test_vgm_rejects_non_genesis() {
        let mut data = make_vgm_header(0, 0);
        finalize_vgm(&mut data, 0);
        let result = import_vgm_data(&data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not a Genesis"));
    }

    #[test]
    fn test_vgm_decompresses_vgz() {
        use flate2::write::GzEncoder;
        use std::io::Write;

        let raw = make_fm_note_vgm();
        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(&raw).unwrap();
        let compressed = encoder.finish().unwrap();

        let import = import_vgm_data(&compressed).unwrap();
        assert_eq!(import.song.tracks.len(), 1);
        assert_eq!(import.song.instruments.fm.len(), 1);
        assert_eq!(import.song.tracks[0].regions[0].notes[0].pitch, 69);
    }

    #[test]
    fn test_vgm_import_single_fm_note() {
        let data = make_fm_note_vgm();
        let import = import_vgm_data(&data).unwrap();

        assert_eq!(import.song.tracks.len(), 1);
        assert_eq!(import.song.instruments.fm.len(), 1);

        let track = &import.song.tracks[0];
        assert_eq!(track.name, "FM1");
        assert!(matches!(track.channel, ChannelAssignment::Fm(0)));
        assert_eq!(track.regions.len(), 1);
        assert_eq!(track.regions[0].notes.len(), 1);

        let note = &track.regions[0].notes[0];
        assert_eq!(note.pitch, 69); // A4
        assert_eq!(note.detune, 0);
        assert_eq!(note.duration_ticks, 1200);

        // ALG=2: carrier mask 0b0001, carrier is DAW op 0 (HW slot 3)
        // TL=0x20=32, velocity = 127 - 32 = 95
        assert_eq!(note.velocity, 95);

        let inst = &import.song.instruments.fm[0];
        assert_eq!(inst.algorithm, 2);
        assert_eq!(inst.feedback, 6);
    }

    #[test]
    fn test_vgm_fm_instrument_dedup() {
        let mut data = make_vgm_header(7_670_454, 0);

        for ch in 0..2u8 {
            program_fm_ch(&mut data, 0, ch, 0x20, 0x32);
            ym_write(&mut data, 0, 0xA4 + ch, 0x24);
            ym_write(&mut data, 0, 0xA0 + ch, 0x3C);
        }

        ym_write(&mut data, 0, 0x28, 0xF0);
        ym_write(&mut data, 0, 0x28, 0xF1);
        wait_samples(&mut data, 22050);
        ym_write(&mut data, 0, 0x28, 0x00);
        ym_write(&mut data, 0, 0x28, 0x01);
        finalize_vgm(&mut data, 22050);

        let import = import_vgm_data(&data).unwrap();
        assert_eq!(
            import.song.instruments.fm.len(),
            1,
            "identical patches should be deduplicated"
        );
        assert_eq!(import.song.tracks.len(), 2);
    }

    #[test]
    fn test_vgm_psg_note() {
        let mut data = make_vgm_header(0, 3_579_545);

        let period_low = (254 & 0x0F) as u8;
        let period_high = (254 >> 4) as u8;

        data.push(0x50);
        data.push(0x80 | period_low);
        data.push(0x50);
        data.push(period_high);
        data.push(0x50);
        data.push(0x90);

        wait_samples(&mut data, 22050);

        data.push(0x50);
        data.push(0x9F);

        finalize_vgm(&mut data, 22050);

        let import = import_vgm_data(&data).unwrap();
        assert_eq!(import.song.tracks.len(), 1);

        let track = &import.song.tracks[0];
        assert_eq!(track.name, "PSG1");
        assert!(matches!(track.channel, ChannelAssignment::Psg(0)));
        assert_eq!(track.regions[0].notes.len(), 1);

        let note = &track.regions[0].notes[0];
        assert_eq!(note.pitch, 69);
        assert_eq!(note.velocity, 127);
        assert_eq!(note.duration_ticks, 600);
    }

    #[test]
    fn test_vgm_gd3_title() {
        let mut data = make_vgm_header(7_670_454, 0);
        finalize_vgm(&mut data, 0);

        let gd3_offset = data.len();
        let gd3_rel = (gd3_offset - 0x14) as u32;
        data[0x14..0x18].copy_from_slice(&gd3_rel.to_le_bytes());

        data.extend_from_slice(b"Gd3 ");
        data.extend_from_slice(&0x0100u32.to_le_bytes());
        let title_utf16: Vec<u8> = "Test Song"
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let data_size = title_utf16.len() + 2;
        data.extend_from_slice(&(data_size as u32).to_le_bytes());
        data.extend_from_slice(&title_utf16);
        data.extend_from_slice(&[0, 0]);

        let title = parse_gd3_title(&data, gd3_rel);
        assert_eq!(title, Some("Test Song".to_string()));
    }

    #[test]
    fn test_vgm_import_file_creates_project() {
        let data = make_fm_note_vgm();
        let tmp = tempfile::tempdir().unwrap();
        let vgm_path = tmp.path().join("test_song.vgm");
        std::fs::write(&vgm_path, &data).unwrap();

        let result = import_vgm_file(&vgm_path, tmp.path(), &Default::default()).unwrap();
        let project_dir = std::path::PathBuf::from(&result.project_dir);

        assert!(project_dir.join("project.json").exists());
        assert!(project_dir.join(".seraph").exists());
        assert!(project_dir.join("instruments/fm").exists());
        assert_eq!(result.track_count, 1);
        assert_eq!(result.instrument_count, 1);
        assert!(result.project_dir.ends_with("test_song"));
    }

    #[test]
    fn test_vgm_port1_channels() {
        let mut data = make_vgm_header(7_670_454, 0);
        program_fm_ch(&mut data, 1, 0, 0x10, 0x00);
        ym_write(&mut data, 1, 0xA4, 0x24);
        ym_write(&mut data, 1, 0xA0, 0x3C);
        ym_write(&mut data, 0, 0x28, 0xF4);
        wait_samples(&mut data, 22050);
        ym_write(&mut data, 0, 0x28, 0x04);
        finalize_vgm(&mut data, 22050);

        let import = import_vgm_data(&data).unwrap();
        assert_eq!(import.song.tracks.len(), 1);
        assert_eq!(import.song.tracks[0].name, "FM4");
        assert!(matches!(
            import.song.tracks[0].channel,
            ChannelAssignment::Fm(3)
        ));
    }

    #[test]
    fn test_vgm_dac_warning() {
        let mut data = make_vgm_header(7_670_454, 0);
        ym_write(&mut data, 0, 0x2B, 0x80);
        finalize_vgm(&mut data, 0);

        let import = import_vgm_data(&data).unwrap();
        assert!(import.warnings.iter().any(|w| w.channel == "DAC"));
    }

    #[test]
    fn test_vgm_fm_velocity_from_carrier_tl() {
        let mut data = make_vgm_header(7_670_454, 0);
        // ALG=7: all 4 ops are carriers
        for slot in 0..4u8 {
            ym_write_op_reg(&mut data, 0, 0x30, 0, slot, 0x01);
            ym_write_op_reg(&mut data, 0, 0x40, 0, slot, slot * 10); // TL=0,10,20,30
            ym_write_op_reg(&mut data, 0, 0x50, 0, slot, 0x1F);
            ym_write_op_reg(&mut data, 0, 0x60, 0, slot, 0x00);
            ym_write_op_reg(&mut data, 0, 0x70, 0, slot, 0x00);
            ym_write_op_reg(&mut data, 0, 0x80, 0, slot, 0x0F);
        }
        ym_write(&mut data, 0, 0xB0, 0x07); // FB=0, ALG=7
        ym_write(&mut data, 0, 0xB4, 0xC0);
        ym_write(&mut data, 0, 0xA4, 0x24);
        ym_write(&mut data, 0, 0xA0, 0x3C);
        ym_write(&mut data, 0, 0x28, 0xF0);
        wait_samples(&mut data, 22050);
        ym_write(&mut data, 0, 0x28, 0x00);
        finalize_vgm(&mut data, 22050);

        let import = import_vgm_data(&data).unwrap();
        let note = &import.song.tracks[0].regions[0].notes[0];
        // min carrier TL = 0, velocity = 127-0 = 127
        assert_eq!(note.velocity, 127);
        // Carrier TLs in instrument should be [0, 10, 20, 30] (relative to min=0)
        let inst = &import.song.instruments.fm[0];
        assert_eq!(inst.operators[0].total_level, 30); // HW slot 3 → DAW op 0
        assert_eq!(inst.operators[1].total_level, 10); // HW slot 1 → DAW op 1
        assert_eq!(inst.operators[2].total_level, 20); // HW slot 2 → DAW op 2
        assert_eq!(inst.operators[3].total_level, 0);  // HW slot 0 → DAW op 3
    }

    #[test]
    fn test_vgm_per_note_pan() {
        let mut data = make_vgm_header(7_670_454, 0);
        program_fm_ch(&mut data, 0, 0, 0x00, 0x00);
        // Left pan for first note
        ym_write(&mut data, 0, 0xB4, 0x80);
        ym_write(&mut data, 0, 0xA4, 0x24);
        ym_write(&mut data, 0, 0xA0, 0x3C);
        ym_write(&mut data, 0, 0x28, 0xF0);
        wait_samples(&mut data, 11025);
        ym_write(&mut data, 0, 0x28, 0x00);
        // Right pan for second note
        ym_write(&mut data, 0, 0xB4, 0x40);
        ym_write(&mut data, 0, 0x28, 0xF0);
        wait_samples(&mut data, 11025);
        ym_write(&mut data, 0, 0x28, 0x00);
        finalize_vgm(&mut data, 22050);

        let import = import_vgm_data(&data).unwrap();
        let notes = &import.song.tracks[0].regions[0].notes;
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].pan_override, Some(0x80));
        assert_eq!(notes[1].pan_override, Some(0x40));
    }

    #[test]
    fn test_vgm_lfo_detection() {
        let mut data = make_vgm_header(7_670_454, 0);
        // LFO enable, speed=3
        ym_write(&mut data, 0, 0x22, 0x0B);
        program_fm_ch(&mut data, 0, 0, 0x00, 0x00);
        // center pan, FMS=5
        ym_write(&mut data, 0, 0xB4, 0xC5);
        ym_write(&mut data, 0, 0xA4, 0x24);
        ym_write(&mut data, 0, 0xA0, 0x3C);
        ym_write(&mut data, 0, 0x28, 0xF0);
        wait_samples(&mut data, 22050);
        ym_write(&mut data, 0, 0x28, 0x00);
        finalize_vgm(&mut data, 22050);

        let import = import_vgm_data(&data).unwrap();
        let note = &import.song.tracks[0].regions[0].notes[0];
        assert!(note.modulation.is_some(), "LFO should generate modulation");
        let m = note.modulation.as_ref().unwrap();
        assert!(m.delta > 0);
        assert!(m.steps > 0);
        // Pan includes FMS bits
        assert_eq!(note.pan_override, Some(0xC5));
        // LFO warning present
        assert!(import
            .warnings
            .iter()
            .any(|w| w.channel == "FM" && w.message.contains("LFO")));
    }

    #[test]
    fn test_vgm_dac_placeholder_notes() {
        let mut data = make_vgm_header(7_670_454, 0);
        ym_write(&mut data, 0, 0x2B, 0x80); // enable DAC
        // First hit: two writes close together
        ym_write(&mut data, 0, 0x2A, 0x80);
        wait_samples(&mut data, 1000);
        ym_write(&mut data, 0, 0x2A, 0x40);
        wait_samples(&mut data, 1000);
        // Gap > DAC_GAP_THRESHOLD
        wait_samples(&mut data, 5000);
        // Second hit
        ym_write(&mut data, 0, 0x2A, 0x80);
        wait_samples(&mut data, 1000);
        finalize_vgm(&mut data, 8000);

        let import = import_vgm_data(&data).unwrap();
        let dac_track = import
            .song
            .tracks
            .iter()
            .find(|t| t.name == "DAC");
        assert!(dac_track.is_some(), "should create DAC track");
        assert_eq!(dac_track.unwrap().regions[0].notes.len(), 2);
    }

    #[test]
    fn test_vgm_ch3_special_mode_warning() {
        let mut data = make_vgm_header(7_670_454, 0);
        // Enable CH3 special mode
        ym_write(&mut data, 0, 0x27, 0x40);
        program_fm_ch(&mut data, 0, 2, 0x00, 0x00);
        ym_write(&mut data, 0, 0xA6, 0x24);
        ym_write(&mut data, 0, 0xA2, 0x3C);
        ym_write(&mut data, 0, 0x28, 0xF2);
        wait_samples(&mut data, 11025);
        ym_write(&mut data, 0, 0x28, 0x02);
        finalize_vgm(&mut data, 11025);

        let import = import_vgm_data(&data).unwrap();
        assert!(import.warnings.iter().any(|w| w.channel == "FM3"));
    }

    #[test]
    fn test_vgm_mid_note_pitch_vibrato() {
        let mut data = make_vgm_header(7_670_454, 0);
        program_fm_ch(&mut data, 0, 0, 0x00, 0x00);
        // Base freq
        ym_write(&mut data, 0, 0xA4, 0x24);
        ym_write(&mut data, 0, 0xA0, 0x3C);
        ym_write(&mut data, 0, 0x28, 0xF0);
        // Simulate vibrato: oscillate freq around base (0x24<<8 | 0x3C)
        let base: u16 = 0x243C;
        for i in 0..8 {
            wait_samples(&mut data, 500);
            let offset: i16 = if i % 2 == 0 { 10 } else { -10 };
            let freq = (base as i16 + offset) as u16;
            ym_write(&mut data, 0, 0xA4, ((freq >> 8) & 0x3F) as u8);
            ym_write(&mut data, 0, 0xA0, (freq & 0xFF) as u8);
        }
        wait_samples(&mut data, 500);
        ym_write(&mut data, 0, 0x28, 0x00);
        finalize_vgm(&mut data, 4500);

        let import = import_vgm_data(&data).unwrap();
        let note = &import.song.tracks[0].regions[0].notes[0];
        assert!(
            note.modulation.is_some(),
            "oscillating pitch should produce modulation"
        );
    }
}
