use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use crate::audio::command::AudioCommand;
use crate::sequencer::{Sequencer, SequencerOutput};
use crate::ym2612::Ym2612;
use crate::sn76489::Sn76489;

const YM2612_MASTER_CLOCK: f64 = 7_670_453.0;
const SN76489_CLOCK_DIVIDER: f64 = 16.0;

struct PsgEnvelopePlayer {
    hw_ch: u8,
    envelope: Arc<Vec<u8>>,
    loop_point: Option<usize>,
    index: usize,
    volume_atten: u8,
}

struct FmModulationPlayer {
    port_base: u32,
    ch_offset: u8,
    base_freq: u16,
    accumulated: i16,
    wait_counter: u8,
    speed_counter: u8,
    speed_reset: u8,
    delta: i16,
    steps_counter: u8,
    steps_reset: u8,
}

struct PsgModulationPlayer {
    hw_ch: u8,
    base_period: u16,
    accumulated: i16,
    wait_counter: u8,
    speed_counter: u8,
    speed_reset: u8,
    delta: i16,
    steps_counter: u8,
    steps_reset: u8,
}

pub struct AudioEngine {
    ym2612: Ym2612,
    sn76489: Sn76489,
    sample_rate: f64,
    ym_cycles_per_sample: f64,
    psg_clocks_per_sample: f64,
    ym_cycle_accumulator: f64,
    psg_clock_accumulator: f64,
    dac_samples: Option<Arc<Vec<u8>>>,
    dac_position: f64,
    dac_step: f64,
    psg_preview_envelope: Option<Arc<Vec<u8>>>,
    psg_preview_loop: Option<usize>,
    psg_preview_channel: u8,
    psg_preview_index: usize,
    psg_preview_tick_acc: f64,
    psg_preview_samples_per_tick: f64,
    psg_envelopes: [Option<PsgEnvelopePlayer>; 4],
    fm_modulations: [Option<FmModulationPlayer>; 6],
    psg_modulations: [Option<PsgModulationPlayer>; 4],
    psg_env_tick_acc: f64,
    sequencer: Sequencer,
    position_tick: Arc<AtomicU64>,
    channel_levels: Arc<Vec<AtomicU8>>,
    level_decay_counter: u32,
    sequencer_output_buf: Vec<SequencerOutput>,
    ym_settle_cycles: u32,
}

impl AudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate_f = sample_rate as f64;
        let ym_cycles_per_sample = YM2612_MASTER_CLOCK / 144.0 / sample_rate_f;
        let psg_clocks_per_sample = YM2612_MASTER_CLOCK / SN76489_CLOCK_DIVIDER / sample_rate_f;

        AudioEngine {
            ym2612: Ym2612::new(),
            sn76489: Sn76489::new(),
            sample_rate: sample_rate_f,
            ym_cycles_per_sample,
            psg_clocks_per_sample,
            ym_cycle_accumulator: 0.0,
            psg_clock_accumulator: 0.0,
            dac_samples: None,
            dac_position: 0.0,
            dac_step: 0.0,
            psg_preview_envelope: None,
            psg_preview_loop: None,
            psg_preview_channel: 0,
            psg_preview_index: 0,
            psg_preview_tick_acc: 0.0,
            psg_preview_samples_per_tick: sample_rate_f / 60.0,
            psg_envelopes: [None, None, None, None],
            fm_modulations: [None, None, None, None, None, None],
            psg_modulations: [None, None, None, None],
            psg_env_tick_acc: 0.0,
            sequencer: Sequencer::new(sample_rate),
            position_tick: Arc::new(AtomicU64::new(0)),
            channel_levels: Arc::new((0..16).map(|_| AtomicU8::new(0)).collect()),
            level_decay_counter: 0,
            sequencer_output_buf: Vec::new(),
            ym_settle_cycles: 0,
        }
    }

    pub fn position_tick(&self) -> Arc<AtomicU64> {
        self.position_tick.clone()
    }

    pub fn channel_levels(&self) -> Arc<Vec<AtomicU8>> {
        self.channel_levels.clone()
    }

    pub fn process_command(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::Ym2612Write { port, data } => {
                self.ym2612.write(port, data);
                // nuked-opm emulates the real YM2612 bus timing faithfully.
                // The chip's internal state machine runs at master clock rate;
                // register writes only propagate to the synthesis state when the
                // time-division cycle reaches the matching operator slot (~24 cycles).
                // Genesis game code always waits 8-12 M68K cycles (≈40-60 YM clocks)
                // between writes for the same reason. We clock 24 times here to
                // guarantee the write settles before the next command is processed.
                for _ in 0..24 {
                    self.ym2612.clock();
                }
            }
            AudioCommand::Sn76489Write { data } => {
                self.sn76489.write(data);
            }
            AudioCommand::FmKeyOn { channel, operators } => {
                // Register 0x28: key on/off.
                // Bits 6-4 = operator enable mask, bits 2-0 = channel.
                // Channel encoding: 0-2 = Part I ch 0-2, 4-6 = Part II ch 0-2.
                // Gap at 3 (no channel), so channels 3-5 map to 4-6.
                let ch_encoded = if channel < 3 { channel } else { channel + 1 };
                let value = (operators & 0xF0) | (ch_encoded & 0x07);
                self.ym2612.write(0, 0x28);
                for _ in 0..24 { self.ym2612.clock(); }
                self.ym2612.write(1, value);
                for _ in 0..24 { self.ym2612.clock(); }
            }
            AudioCommand::FmKeyOff { channel } => {
                // Key off: operator bitmask = 0 (all operators released).
                let ch_encoded = if channel < 3 { channel } else { channel + 1 };
                let value = ch_encoded & 0x07;
                self.ym2612.write(0, 0x28);
                for _ in 0..24 { self.ym2612.clock(); }
                self.ym2612.write(1, value);
                for _ in 0..24 { self.ym2612.clock(); }
            }
            AudioCommand::DacPlayback { samples, sample_rate } => {
                self.dac_samples = Some(samples);
                self.dac_position = 0.0;
                self.dac_step = sample_rate as f64 / self.sample_rate;
            }
            AudioCommand::PsgEnvelopePreview { channel, period, envelope, loop_point } => {
                let low_nibble = (period & 0x0F) as u8;
                let high_bits = ((period >> 4) & 0x3F) as u8;
                self.sn76489.write(0x80 | (channel << 5) | low_nibble);
                self.sn76489.write(high_bits);
                self.sn76489.write(0x90 | (channel << 5));
                self.psg_preview_envelope = Some(envelope);
                self.psg_preview_loop = loop_point;
                self.psg_preview_channel = channel;
                self.psg_preview_index = 0;
                self.psg_preview_tick_acc = 0.0;
            }
            AudioCommand::StopPreview => {
                self.dac_samples = None;
                self.dac_position = 0.0;
                if self.psg_preview_envelope.take().is_some() {
                    self.sn76489.write(0x90 | (self.psg_preview_channel << 5) | 0x0F);
                }
            }
            AudioCommand::Panic => {
                self.ym2612.reset();
                self.sn76489.reset();
                self.ym_cycle_accumulator = 0.0;
                self.ym_settle_cycles = 0;
                self.psg_clock_accumulator = 0.0;
                self.dac_samples = None;
                self.dac_position = 0.0;
                self.psg_preview_envelope = None;
                self.psg_envelopes = [None, None, None, None];
                self.fm_modulations = [None, None, None, None, None, None];
                self.psg_modulations = [None, None, None, None];
                self.psg_env_tick_acc = 0.0;
            }
            AudioCommand::TransportPlay => {
                self.sequencer.play();
            }
            AudioCommand::TransportStop => {
                let mut output = Vec::new();
                self.sequencer.stop(&mut output);
                self.apply_sequencer_output(&mut output);
            }
            AudioCommand::TransportSeek { tick } => {
                let mut output = Vec::new();
                self.sequencer.seek(tick, &mut output);
                self.apply_sequencer_output(&mut output);
                self.position_tick.store(tick, Ordering::Relaxed);
            }
            AudioCommand::TransportSetLoop { start_tick, end_tick } => {
                self.sequencer.set_loop(start_tick, end_tick);
            }
            AudioCommand::TransportClearLoop => {
                self.sequencer.clear_loop();
            }
            AudioCommand::LoadSequence { snapshot } => {
                self.sequencer.load_snapshot(snapshot);
            }
            AudioCommand::ReloadSequence { snapshot } => {
                let mut output = Vec::new();
                self.sequencer.reload_snapshot(snapshot, &mut output);
                self.apply_sequencer_output(&mut output);
            }
        }
    }

    fn apply_sequencer_output(&mut self, output: &mut Vec<SequencerOutput>) {
        for cmd in output.drain(..) {
            match cmd {
                SequencerOutput::FmWrite(w) => {
                    self.ym2612.write(w.port, w.data);
                    for _ in 0..24 { self.ym2612.clock(); }
                    self.ym_settle_cycles += 1;
                }
                SequencerOutput::PsgWrite(data) => {
                    self.sn76489.write(data);
                }
                SequencerOutput::DacPlayback { samples, sample_rate } => {
                    self.dac_samples = Some(samples);
                    self.dac_position = 0.0;
                    self.dac_step = sample_rate as f64 / self.sample_rate;
                }
                SequencerOutput::PsgEnvelopeStart { hw_ch, envelope, loop_point, volume_atten } => {
                    if (hw_ch as usize) < 4 && !envelope.is_empty() {
                        let vol = envelope[0];
                        let final_vol = vol.saturating_sub(volume_atten);
                        let attenuation = 15u8.saturating_sub(final_vol);
                        self.sn76489.write(0x90 | (hw_ch << 5) | (attenuation & 0x0F));
                        self.psg_envelopes[hw_ch as usize] = Some(PsgEnvelopePlayer {
                            hw_ch,
                            envelope,
                            loop_point,
                            index: 1,
                            volume_atten,
                        });
                    }
                }
                SequencerOutput::PsgEnvelopeStop { hw_ch } => {
                    if (hw_ch as usize) < 4 {
                        self.psg_envelopes[hw_ch as usize] = None;
                    }
                }
                SequencerOutput::FmModulationStart { hw_ch, wait, speed, delta, steps, base_freq } => {
                    if (hw_ch as usize) < 6 {
                        let (port_base, ch_offset) = if hw_ch < 3 { (0u32, hw_ch) } else { (2u32, hw_ch - 3) };
                        self.fm_modulations[hw_ch as usize] = Some(FmModulationPlayer {
                            port_base,
                            ch_offset,
                            base_freq,
                            accumulated: 0,
                            wait_counter: wait,
                            speed_counter: speed,
                            speed_reset: speed,
                            delta: delta as i16,
                            steps_counter: steps / 2,
                            steps_reset: steps,
                        });
                    }
                }
                SequencerOutput::FmModulationStop { hw_ch } => {
                    if (hw_ch as usize) < 6 {
                        self.fm_modulations[hw_ch as usize] = None;
                    }
                }
                SequencerOutput::PsgModulationStart { hw_ch, wait, speed, delta, steps, base_period } => {
                    if (hw_ch as usize) < 4 {
                        self.psg_modulations[hw_ch as usize] = Some(PsgModulationPlayer {
                            hw_ch,
                            base_period,
                            accumulated: 0,
                            wait_counter: wait,
                            speed_counter: speed,
                            speed_reset: speed,
                            delta: delta as i16,
                            steps_counter: steps / 2,
                            steps_reset: steps,
                        });
                    }
                }
                SequencerOutput::PsgModulationStop { hw_ch } => {
                    if (hw_ch as usize) < 4 {
                        self.psg_modulations[hw_ch as usize] = None;
                    }
                }
            }
        }
    }

    fn update_channel_levels(&self) {
        use crate::sequencer::ChannelType;
        let activity = self.sequencer.channel_activity();
        for atom in self.channel_levels.iter() {
            let cur = atom.load(Ordering::Relaxed);
            if cur > 0 {
                atom.store(cur.saturating_sub(8), Ordering::Relaxed);
            }
        }
        for (ch_type, active, volume) in &activity {
            if !active { continue; }
            let idx = match ch_type {
                ChannelType::Fm(n) => *n as usize,
                ChannelType::Psg(n) => 6 + *n as usize,
                ChannelType::PsgNoise => 9,
                ChannelType::Dac(_) => 10,
            };
            if idx < self.channel_levels.len() {
                self.channel_levels[idx].store(*volume, Ordering::Relaxed);
            }
        }
    }

    /// Render interleaved stereo f32 samples into `buffer`.
    ///
    /// Buffer layout: [L0, R0, L1, R1, ...] — must have even length.
    pub fn render(&mut self, buffer: &mut [f32]) {
        debug_assert!(buffer.len() % 2 == 0, "render buffer must have even length");

        let frame_count = buffer.len() / 2;

        for frame in 0..frame_count {
            // --- Sequencer ---
            self.sequencer_output_buf.clear();
            self.sequencer.advance(&mut self.sequencer_output_buf);
            if !self.sequencer_output_buf.is_empty() {
                let mut buf = std::mem::take(&mut self.sequencer_output_buf);
                self.apply_sequencer_output(&mut buf);
                self.sequencer_output_buf = buf;
            }
            if self.sequencer.is_playing() {
                self.position_tick.store(self.sequencer.current_tick_u64(), Ordering::Relaxed);
            }

            self.level_decay_counter += 1;
            if self.level_decay_counter >= 512 {
                self.level_decay_counter = 0;
                self.update_channel_levels();
            }

            // --- YM2612 (24-clock cycle-based rendering) ---
            // Nuked OPN2 is time-multiplexed: each 24-clock cycle processes all
            // 6 channels. Only every 4th clock produces a valid sample (the other
            // 3 are sign-extension noise). We render in complete 24-clock cycles
            // and accumulate only the 6 valid outputs per cycle.
            self.ym_cycle_accumulator += self.ym_cycles_per_sample;
            let cycles_raw = self.ym_cycle_accumulator as u32;
            self.ym_cycle_accumulator -= cycles_raw as f64;

            let cycles = cycles_raw.saturating_sub(self.ym_settle_cycles);
            self.ym_settle_cycles = self.ym_settle_cycles.saturating_sub(cycles_raw);

            let mut ym_l: i32 = 0;
            let mut ym_r: i32 = 0;
            let mut valid_count: u32 = 0;

            for _ in 0..cycles {
                for clk in 0..24u32 {
                    let s = self.ym2612.clock();
                    if clk & 3 == 3 {
                        ym_l += s[0] as i32;
                        ym_r += s[1] as i32;
                        valid_count += 1;
                    }
                }
            }
            if valid_count > 0 {
                ym_l /= valid_count as i32;
                ym_r /= valid_count as i32;
            }

            // --- SN76489 PSG ---
            // PSG runs at master_clock / 16.
            self.psg_clock_accumulator += self.psg_clocks_per_sample;
            let psg_ticks = self.psg_clock_accumulator as u32;
            self.psg_clock_accumulator -= psg_ticks as f64;

            for _ in 0..psg_ticks {
                self.sn76489.clock();
            }
            let psg_sample = self.sn76489.render_sample() as i32;

            // --- DAC ---
            let fm_scale: i32 = 48;
            let dac_sample = if let Some(ref samples) = self.dac_samples {
                let idx = self.dac_position as usize;
                if idx < samples.len() {
                    let raw = samples[idx] as i32 - 128;
                    self.dac_position += self.dac_step;
                    raw * fm_scale
                } else {
                    self.dac_samples = None;
                    0
                }
            } else {
                0
            };

            // --- PSG envelope stepping (60 Hz) ---
            self.psg_env_tick_acc += 1.0;
            let step_envelopes = self.psg_env_tick_acc >= self.psg_preview_samples_per_tick;
            if step_envelopes {
                self.psg_env_tick_acc -= self.psg_preview_samples_per_tick;
            }

            // Preview envelope (instrument editor)
            if let Some(ref envelope) = self.psg_preview_envelope.clone() {
                if step_envelopes && self.psg_preview_index < envelope.len() {
                    let vol = envelope[self.psg_preview_index];
                    let attenuation = 15u8.saturating_sub(vol);
                    self.sn76489.write(0x90 | (self.psg_preview_channel << 5) | attenuation);
                    self.psg_preview_index += 1;
                    if self.psg_preview_index >= envelope.len() {
                        if let Some(lp) = self.psg_preview_loop {
                            self.psg_preview_index = lp;
                        } else {
                            self.sn76489.write(0x90 | (self.psg_preview_channel << 5) | 0x0F);
                            self.psg_preview_envelope = None;
                        }
                    }
                }
            }

            // Sequencer PSG envelopes (up to 4 channels)
            if step_envelopes {
                for slot in 0..4 {
                    let finished = if let Some(ref mut player) = self.psg_envelopes[slot] {
                        if player.index < player.envelope.len() {
                            let vol = player.envelope[player.index];
                            let final_vol = vol.saturating_sub(player.volume_atten);
                            let attenuation = 15u8.saturating_sub(final_vol);
                            self.sn76489.write(0x90 | (player.hw_ch << 5) | (attenuation & 0x0F));
                            player.index += 1;
                            if player.index >= player.envelope.len() {
                                if let Some(lp) = player.loop_point {
                                    player.index = lp;
                                    false
                                } else {
                                    self.sn76489.write(0x90 | (player.hw_ch << 5) | 0x0F);
                                    true
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        }
                    } else {
                        false
                    };
                    if finished {
                        self.psg_envelopes[slot] = None;
                    }
                }
            }

            // --- FM modulation stepping (60 Hz, same as PSG envelopes) ---
            if step_envelopes {
                for slot in 0..6 {
                    let needs_write = if let Some(ref mut player) = self.fm_modulations[slot] {
                        if player.wait_counter > 0 {
                            player.wait_counter -= 1;
                            false
                        } else {
                            player.speed_counter = player.speed_counter.saturating_sub(1);
                            if player.speed_counter == 0 {
                                player.speed_counter = player.speed_reset;
                                player.accumulated = player.accumulated.wrapping_add(player.delta);
                                player.steps_counter = player.steps_counter.saturating_sub(1);
                                if player.steps_counter == 0 {
                                    player.steps_counter = player.steps_reset;
                                    player.delta = -player.delta;
                                }
                            }
                            true
                        }
                    } else {
                        false
                    };
                    if needs_write {
                        if let Some(ref player) = self.fm_modulations[slot] {
                            let freq = (player.base_freq as i32 + player.accumulated as i32).clamp(0, 0x3FFF) as u16;
                            let freq_msb = (freq >> 8) as u8;
                            let freq_lsb = (freq & 0xFF) as u8;
                            let port = player.port_base;
                            let ch = player.ch_offset;
                            self.ym2612.write(port, 0xA4 + ch);
                            for _ in 0..24 { self.ym2612.clock(); }
                            self.ym2612.write(port + 1, freq_msb);
                            for _ in 0..24 { self.ym2612.clock(); }
                            self.ym2612.write(port, 0xA0 + ch);
                            for _ in 0..24 { self.ym2612.clock(); }
                            self.ym2612.write(port + 1, freq_lsb);
                            for _ in 0..24 { self.ym2612.clock(); }
                        }
                    }
                }

                // --- PSG modulation stepping (same 60 Hz tick) ---
                for slot in 0..4 {
                    let needs_write = if let Some(ref mut player) = self.psg_modulations[slot] {
                        if player.wait_counter > 0 {
                            player.wait_counter -= 1;
                            false
                        } else {
                            player.speed_counter = player.speed_counter.saturating_sub(1);
                            if player.speed_counter == 0 {
                                player.speed_counter = player.speed_reset;
                                player.accumulated = player.accumulated.wrapping_add(player.delta);
                                player.steps_counter = player.steps_counter.saturating_sub(1);
                                if player.steps_counter == 0 {
                                    player.steps_counter = player.steps_reset;
                                    player.delta = -player.delta;
                                }
                            }
                            true
                        }
                    } else {
                        false
                    };
                    if needs_write {
                        if let Some(ref player) = self.psg_modulations[slot] {
                            let period = (player.base_period as i32 + player.accumulated as i32).clamp(0, 1023) as u16;
                            let low_nibble = (period & 0x0F) as u8;
                            let high_bits = ((period >> 4) & 0x3F) as u8;
                            self.sn76489.write(0x80 | (player.hw_ch << 5) | low_nibble);
                            self.sn76489.write(high_bits);
                        }
                    }
                }
            }

            // --- Mix and normalize ---
            // PSG VOLUME_TABLE max per channel = 8191, 4 channels = ~32k.
            // YM2612 raw ~250 * fm_scale(48) = ~12k per channel.
            // On real hardware PSG is roughly 1/4 to 1/5 FM amplitude.
            let scaled_l = ym_l * fm_scale + psg_sample / 4 + dac_sample;
            let scaled_r = ym_r * fm_scale + psg_sample / 4 + dac_sample;

            buffer[frame * 2]     = (scaled_l as f32 / 32768.0).clamp(-1.0, 1.0);
            buffer[frame * 2 + 1] = (scaled_r as f32 / 32768.0).clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: write the FM channel 0 tone patch used across tests.
    /// Algorithm 7 (all-carrier), Op1 max volume, fastest AR, ~265 Hz, key-on.
    fn program_fm_tone(engine: &mut AudioEngine) {
        // Algorithm 7 (all-carrier), feedback 0
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xB0 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x07 });
        // Op1 TL = 0 (max volume)
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x40 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
        // Op1 AR = 31 (fastest attack)
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x50 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F });
        // Op1 SL=0, RR=0 (sustain forever)
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x80 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
        // Op1 DT=0, MUL=1
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x30 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x01 });
        // Frequency block 4, F-num (≈265 Hz)
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA4 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x22 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA0 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x8D });
        // Key on: all operators, channel 0
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x28 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xF0 });
    }

    #[test]
    fn test_renders_silence_by_default() {
        let mut engine = AudioEngine::new(44100);
        let mut buf = [0.0f32; 128];
        engine.render(&mut buf);
        for &s in &buf {
            assert!(
                s.abs() < 0.05,
                "new engine with no commands should produce near-silence, got {s}"
            );
        }
    }

    #[test]
    fn test_fm_tone_through_engine() {
        let mut engine = AudioEngine::new(44100);
        program_fm_tone(&mut engine);

        let mut buf = [0.0f32; 4096];
        engine.render(&mut buf);

        let has_signal = buf.iter().any(|s| s.abs() > 0.001);
        assert!(has_signal, "FM tone through engine should produce non-zero audio output");
    }

    #[test]
    fn test_dac_playback_produces_audio() {
        let mut engine = AudioEngine::new(44100);
        let samples: Vec<u8> = (0..1000).map(|i| if i % 2 == 0 { 200 } else { 56 }).collect();
        engine.process_command(AudioCommand::DacPlayback {
            samples: Arc::new(samples),
            sample_rate: 16000,
        });
        let mut buf = [0.0f32; 4096];
        engine.render(&mut buf);
        let has_signal = buf.iter().any(|s| s.abs() > 0.01);
        assert!(has_signal, "DAC playback should produce audible output");
    }

    #[test]
    fn test_dac_stops_after_samples_exhausted() {
        let mut engine = AudioEngine::new(44100);
        let samples = vec![200u8; 10];
        engine.process_command(AudioCommand::DacPlayback {
            samples: Arc::new(samples),
            sample_rate: 44100,
        });
        let mut buf = [0.0f32; 4096];
        engine.render(&mut buf);
        let tail = &buf[100..];
        let tail_signal = tail.iter().any(|s| s.abs() > 0.05);
        assert!(!tail_signal, "DAC should stop after samples exhausted");
    }

    #[test]
    fn test_fm_a4_pitch_accuracy() {
        let sample_rate = 44100u32;
        let mut engine = AudioEngine::new(sample_rate);

        // Algorithm 7 (all carriers), feedback 0
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xB0 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x07 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xB4 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xC0 });
        for slot in &[0x00u8, 0x04, 0x08, 0x0C] {
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x30 + slot });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x01 }); // DT=0, MUL=1
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x40 + slot });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 }); // TL=0
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x50 + slot });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F }); // AR=31
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x80 + slot });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 }); // SL=0 RR=0
        }
        // A4: block=4, fnum=1084 → reg A4=$24, A0=$3C
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA4 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x24 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA0 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x3C });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x28 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xF0 });

        let num_frames = sample_rate as usize;
        let mut buf = vec![0.0f32; num_frames * 2];
        engine.render(&mut buf);

        // Left channel, skip attack transient
        let left: Vec<f32> = buf.iter().skip(4000).step_by(2).cloned().collect();
        let n = left.len().min(8192);
        let analysis = &left[..n];
        let mean: f32 = analysis.iter().sum::<f32>() / n as f32;
        let centered: Vec<f32> = analysis.iter().map(|&s| s - mean).collect();

        // Autocorrelation pitch detection (search 200-880 Hz)
        let min_lag = (sample_rate as f32 / 880.0) as usize;
        let max_lag = (sample_rate as f32 / 200.0) as usize;
        let mut best_lag = min_lag;
        let mut best_corr: f32 = f32::MIN;
        for lag in min_lag..=max_lag.min(n / 2) {
            let mut corr: f32 = 0.0;
            for i in 0..(n - lag) {
                corr += centered[i] * centered[i + lag];
            }
            if corr > best_corr {
                best_corr = corr;
                best_lag = lag;
            }
        }

        let detected_freq = sample_rate as f32 / best_lag as f32;
        let error_cents = 1200.0 * (detected_freq / 440.0).ln() / 2.0f32.ln();
        eprintln!("FM A4 pitch: detected={:.1} Hz, expected=440 Hz, error={:.1} cents",
                  detected_freq, error_cents);

        assert!(
            error_cents.abs() < 50.0,
            "FM A4 should be within 50 cents of 440 Hz, got {:.1} Hz ({:.1} cents)",
            detected_freq, error_cents
        );
    }

    #[test]
    fn test_panic_silences_everything() {
        let mut engine = AudioEngine::new(44100);
        program_fm_tone(&mut engine);

        // Render enough frames to confirm the tone is audible.
        let mut buf_before = [0.0f32; 2048];
        engine.render(&mut buf_before);
        let has_signal_before = buf_before.iter().any(|s| s.abs() > 0.001);
        assert!(has_signal_before, "should have signal before panic");

        // Panic: reset all chips.
        engine.process_command(AudioCommand::Panic);

        // Render tail — should drop back to the idle noise floor (<0.001).
        let mut buf_after = [0.0f32; 2048];
        engine.render(&mut buf_after);

        let any_loud_after = buf_after.iter().any(|s| s.abs() >= 0.05);
        assert!(
            !any_loud_after,
            "after Panic, all output should be near-silent (< 0.05); loudest was {:.6}",
            buf_after.iter().cloned().fold(0.0f32, f32::max)
        );
    }
}
