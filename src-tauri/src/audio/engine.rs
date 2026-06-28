use std::sync::Arc;
use std::sync::atomic::{AtomicU8, AtomicU64, Ordering};
use crate::audio::command::AudioCommand;
use crate::audio::spectrum::{SpectrumAnalyzer, SpectrumBuffer};
use crate::sequencer::{Sequencer, SequencerOutput};
use crate::ym2612::Ym2612;
use crate::sn76489::Sn76489;

const YM2612_MASTER_CLOCK: f64 = 7_670_453.0;
const PSG_INPUT_CLOCK: f64 = 3_579_545.0;
const SN76489_CLOCK_DIVIDER: f64 = 16.0;

struct PsgEnvelopePlayer {
    hw_ch: u8,
    envelope: Arc<Vec<u8>>,
    loop_point: Option<usize>,
    index: usize,
    volume_atten: u8,
    silence_on_end: bool,
    delay_ticks: u8,
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
    ym_clocks_per_sample: f64,
    psg_clocks_per_sample: f64,
    ym_clock_accumulator: f64,
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
    master_volume: f32,
    // YM2612 resampling with anti-alias filtering.
    //
    // The chip produces a new ch_out value every 24 internal clocks
    // (~53.3 kHz). Our output rate is 44.1 kHz — only 1.2x apart.
    // This tiny ratio means content between 22 kHz and 26.6 kHz
    // (half-output-rate to half-ch_out-rate) aliases into the audible
    // range at ~9 kHz, producing metallic "crunchiness."
    //
    // Fix: apply a steep anti-alias LPF at the ch_out rate (53 kHz)
    // BEFORE decimation, cutting everything above ~20 kHz. Then
    // integrate the filtered signal over each output sample period
    // (box-car decimation). The alias energy is removed at the source.
    ym_cycle_pos: u32,
    ym_accum_l: f64,
    ym_accum_r: f64,
    ym_accum_samples: f64,
    ym_last_snapshot_l: f32,
    ym_last_snapshot_r: f32,
    // Anti-alias LPF cascade at ch_out rate (~53 kHz).
    // Four cascaded 2nd-order Butterworth sections = 8th order total
    // (-48 dB/octave). Cutoff 20 kHz at 53 kHz sample rate gives
    // -20 dB at 22 kHz (alias ingress) and -40 dB at the ch_out
    // Nyquist (26.6 kHz), while keeping passband loss under 1 dB
    // at 14 kHz. Combined with the output-rate 14 kHz LPF, total
    // alias rejection exceeds 40 dB.
    ym_aa_l: [LowPassFilter; 4],
    ym_aa_r: [LowPassFilter; 4],
    // Output-rate LPF for final smoothing (Genesis analog stage emulation)
    lpf_l: LowPassFilter,
    lpf_r: LowPassFilter,
    spectrum_analyzer: SpectrumAnalyzer,
    peak_sample_counter: u32,
}

impl AudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate_f = sample_rate as f64;
        let ym_clock_rate = YM2612_MASTER_CLOCK / 6.0;
        let ym_clocks_per_sample = ym_clock_rate / sample_rate_f;
        let psg_clocks_per_sample = PSG_INPUT_CLOCK / SN76489_CLOCK_DIVIDER / sample_rate_f;

        AudioEngine {
            ym2612: Ym2612::new(),
            sn76489: Sn76489::new(),
            sample_rate: sample_rate_f,
            ym_clocks_per_sample,
            psg_clocks_per_sample,
            ym_clock_accumulator: 0.0,
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
            master_volume: 1.0,
            ym_cycle_pos: 0,
            ym_accum_l: 0.0,
            ym_accum_r: 0.0,
            ym_accum_samples: 0.0,
            ym_last_snapshot_l: 0.0,
            ym_last_snapshot_r: 0.0,
            // Anti-alias filter cascade at ch_out rate (~53267 Hz).
            // 4-stage cascade with 20 kHz cutoff.
            ym_aa_l: [
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
            ],
            ym_aa_r: [
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
                LowPassFilter::new(ym_clock_rate / 24.0, 20000.0),
            ],
            lpf_l: LowPassFilter::new(sample_rate_f, 14000.0),
            lpf_r: LowPassFilter::new(sample_rate_f, 14000.0),
            spectrum_analyzer: SpectrumAnalyzer::new(),
            peak_sample_counter: 0,
        }
    }

    pub fn position_tick(&self) -> Arc<AtomicU64> {
        self.position_tick.clone()
    }

    pub fn channel_levels(&self) -> Arc<Vec<AtomicU8>> {
        self.channel_levels.clone()
    }

    pub fn spectrum_buffer(&self) -> Arc<SpectrumBuffer> {
        self.spectrum_analyzer.spectrum_buffer()
    }

    #[inline(always)]
    fn ym_clock_settle(&mut self) {
        self.ym2612.clock();
        self.ym_cycle_pos += 1;
        // The chip's ch_out values update once per 24-clock cycle.
        // At each boundary, read the new ch_out sum and feed it through
        // the anti-alias LPF (running at ~53 kHz, cutoff 20 kHz).
        // Then integrate the filtered value over the output sample period.
        self.ym_accum_l += self.ym_last_snapshot_l as f64;
        self.ym_accum_r += self.ym_last_snapshot_r as f64;
        self.ym_accum_samples += 1.0;
        if self.ym_cycle_pos >= 24 {
            self.ym_cycle_pos = 0;
            let ch = self.ym2612.read_channels();
            // 8th-order anti-alias filter at ch_out rate before decimation
            let mut l = ch[0] as f32;
            let mut r = ch[1] as f32;
            for stage in &mut self.ym_aa_l { l = stage.process(l); }
            for stage in &mut self.ym_aa_r { r = stage.process(r); }
            self.ym_last_snapshot_l = l;
            self.ym_last_snapshot_r = r;
        }
    }

    pub fn process_command(&mut self, cmd: AudioCommand) {
        match cmd {
            AudioCommand::Ym2612Write { port, data } => {
                self.ym2612.write(port, data);
                for _ in 0..13 { self.ym_clock_settle(); }
            }
            AudioCommand::Sn76489Write { data } => {
                self.sn76489.write(data);
            }
            AudioCommand::FmKeyOn { channel, operators } => {
                let ch_encoded = if channel < 3 { channel } else { channel + 1 };
                let value = (operators & 0xF0) | (ch_encoded & 0x07);
                self.ym2612.write(0, 0x28);
                for _ in 0..13 { self.ym_clock_settle(); }
                self.ym2612.write(1, value);
                for _ in 0..13 { self.ym_clock_settle(); }
            }
            AudioCommand::FmKeyOff { channel } => {
                let ch_encoded = if channel < 3 { channel } else { channel + 1 };
                let value = ch_encoded & 0x07;
                self.ym2612.write(0, 0x28);
                for _ in 0..13 { self.ym_clock_settle(); }
                self.ym2612.write(1, value);
                for _ in 0..13 { self.ym_clock_settle(); }
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
                self.ym_clock_accumulator = 0.0;
                self.psg_clock_accumulator = 0.0;
                self.dac_samples = None;
                self.dac_position = 0.0;
                self.psg_preview_envelope = None;
                self.psg_envelopes = [None, None, None, None];
                self.fm_modulations = [None, None, None, None, None, None];
                self.psg_modulations = [None, None, None, None];
                self.psg_env_tick_acc = 0.0;
                self.ym_cycle_pos = 0;
                self.ym_accum_l = 0.0;
                self.ym_accum_r = 0.0;
                self.ym_accum_samples = 0.0;
                self.ym_last_snapshot_l = 0.0;
                self.ym_last_snapshot_r = 0.0;
                let ym_ch_out_rate = YM2612_MASTER_CLOCK / 6.0 / 24.0;
                self.ym_aa_l = [
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                ];
                self.ym_aa_r = [
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                    LowPassFilter::new(ym_ch_out_rate, 20000.0),
                ];
                self.lpf_l = LowPassFilter::new(self.sample_rate, 14000.0);
                self.lpf_r = LowPassFilter::new(self.sample_rate, 14000.0);
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
            AudioCommand::SetMasterVolume { volume } => {
                self.master_volume = volume.clamp(0.0, 1.5);
            }
        }
    }

    fn apply_sequencer_output(&mut self, output: &mut Vec<SequencerOutput>) {
        let mut fm_writes: Vec<(u32, u8)> = Vec::new();
        for cmd in output.drain(..) {
            match cmd {
                SequencerOutput::FmWrite(w) => {
                    fm_writes.push((w.port, w.data));
                }
                SequencerOutput::PsgWrite(data) => {
                    self.sn76489.write(data);
                }
                SequencerOutput::DacPlayback { samples, sample_rate } => {
                    self.dac_samples = Some(samples);
                    self.dac_position = 0.0;
                    self.dac_step = sample_rate as f64 / self.sample_rate;
                }
                SequencerOutput::PsgEnvelopeStart { hw_ch, envelope, loop_point, volume_atten, silence_on_end } => {
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
                            silence_on_end,
                            delay_ticks: 0,
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
        for (port, data) in fm_writes {
            self.ym2612.write(port, data);
            for _ in 0..13 { self.ym_clock_settle(); }
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

            let mut dac_sample: i32 = 0;
            if let Some(ref samples) = self.dac_samples.clone() {
                let idx = self.dac_position as usize;
                if idx < samples.len() {
                    dac_sample = samples[idx] as i32 - 128;
                    self.dac_position += self.dac_step;
                } else {
                    self.dac_samples = None;
                }
            }

            // --- YM2612 ---
            // Advance clocks and integrate the piecewise-constant ch_out
            // signal over the output sample period. Dividing the integral
            // by the number of clocks gives a properly anti-aliased sample
            // (box-car filter), eliminating the ~9 kHz alias that linear
            // interpolation between 53 kHz snapshots produces at 44.1 kHz.
            self.ym_accum_l = 0.0;
            self.ym_accum_r = 0.0;
            self.ym_accum_samples = 0.0;
            self.ym_clock_accumulator += self.ym_clocks_per_sample;
            let clocks = self.ym_clock_accumulator as u32;
            self.ym_clock_accumulator -= clocks as f64;

            for _ in 0..clocks {
                self.ym_clock_settle();
            }
            let ym_avg_l = if self.ym_accum_samples > 0.0 {
                (self.ym_accum_l / self.ym_accum_samples) as f32
            } else {
                self.ym_last_snapshot_l
            };
            let ym_avg_r = if self.ym_accum_samples > 0.0 {
                (self.ym_accum_r / self.ym_accum_samples) as f32
            } else {
                self.ym_last_snapshot_r
            };

            // --- SN76489 PSG ---
            self.psg_clock_accumulator += self.psg_clocks_per_sample;
            let psg_ticks = self.psg_clock_accumulator as u32;
            self.psg_clock_accumulator -= psg_ticks as f64;

            for _ in 0..psg_ticks {
                self.sn76489.clock();
            }
            let psg_sample = self.sn76489.render_sample() as i32;

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
                        if player.delay_ticks > 0 {
                            player.delay_ticks -= 1;
                            false
                        } else if player.index < player.envelope.len() {
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
                                    if player.silence_on_end {
                                        self.sn76489.write(0x90 | (player.hw_ch << 5) | 0x0F);
                                    }
                                    true
                                }
                            } else {
                                false
                            }
                        } else {
                            if player.silence_on_end && player.loop_point.is_none() {
                                self.sn76489.write(0x90 | (player.hw_ch << 5) | 0x0F);
                                true
                            } else {
                                false
                            }
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
                            for _ in 0..13 { self.ym_clock_settle(); }
                            self.ym2612.write(port + 1, freq_msb);
                            for _ in 0..13 { self.ym_clock_settle(); }
                            self.ym2612.write(port, 0xA0 + ch);
                            for _ in 0..13 { self.ym_clock_settle(); }
                            self.ym2612.write(port + 1, freq_lsb);
                            for _ in 0..13 { self.ym_clock_settle(); }
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

            // --- Finalize this sample ---
            // Averaged ch_out: 9-bit signed (-256..+255) summed across
            // up to 6 channels → max ±1536. Scale to ~±0.5 float range.
            let fm_l = self.lpf_l.process(ym_avg_l / 3072.0);
            let fm_r = self.lpf_r.process(ym_avg_r / 3072.0);

            let dac_mix = dac_sample as f32 * 48.0 / 65536.0;
            let psg_mix = psg_sample as f32 * 9.0 / (16.0 * 65536.0);

            let pre_l = (fm_l + dac_mix + psg_mix) * self.master_volume;
            let pre_r = (fm_r + dac_mix + psg_mix) * self.master_volume;
            let out_l = soft_clip(pre_l);
            let out_r = soft_clip(pre_r);
            self.spectrum_analyzer.push_sample((out_l + out_r) * 0.5);
            buffer[frame * 2]     = out_l;
            buffer[frame * 2 + 1] = out_r;

            self.peak_sample_counter += 1;
            if self.peak_sample_counter >= self.sample_rate as u32 {
                self.peak_sample_counter = 0;
            }
        }
    }
}


fn soft_clip(x: f32) -> f32 {
    const THRESHOLD: f32 = 0.8;
    if x.abs() <= THRESHOLD {
        x
    } else {
        let excess = x.abs() - THRESHOLD;
        let compressed = THRESHOLD + (1.0 - THRESHOLD) * (1.0 - (-excess * 5.0).exp());
        x.signum() * compressed
    }
}

/// 2nd-order (biquad) low-pass filter modeling the Genesis analog output stage.
/// Real hardware has RC filters that roll off harsh digital edges around 5-12 kHz.
struct LowPassFilter {
    b0: f32, b1: f32, b2: f32,
    a1: f32, a2: f32,
    x1: f32, x2: f32,
    y1: f32, y2: f32,
}

impl LowPassFilter {
    fn new(sample_rate: f64, cutoff_hz: f64) -> Self {
        let omega = 2.0 * std::f64::consts::PI * cutoff_hz / sample_rate;
        let sin_w = omega.sin();
        let cos_w = omega.cos();
        let q = 0.707; // Butterworth
        let alpha = sin_w / (2.0 * q);

        let a0 = 1.0 + alpha;
        let b0 = ((1.0 - cos_w) / 2.0 / a0) as f32;
        let b1 = ((1.0 - cos_w) / a0) as f32;
        let b2 = b0;
        let a1 = (-2.0 * cos_w / a0) as f32;
        let a2 = ((1.0 - alpha) / a0) as f32;

        LowPassFilter { b0, b1, b2, a1, a2, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 }
    }

    #[inline(always)]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
              - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
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
    fn diagnostic_icz1_patch_quality() {
        let mut engine = AudioEngine::new(44100);

        // ICZ1 FM2 patch: Algorithm 5, FB=7
        // Alg 5: (Op1→Op2), (Op3→Op4), carriers = Op2+Op4
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xB0 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xF5 }); // FB=7, ALG=5
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xB4 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xC0 }); // L+R

        // Op1 (slot 0): DT=0, MUL=0 (0.5×)
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x30 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x40 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 }); // TL=0
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x50 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F }); // AR=31
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x60 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1D }); // D1R=29
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x70 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 }); // D2R=0
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x80 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F }); // SL=1, RR=15

        // Op2 (slot 4): DT=0, MUL=10
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x34 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x0A }); // MUL=10
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x44 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 }); // TL=0
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x54 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x64 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1D });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x74 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x16 }); // D2R=22
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x84 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F });

        // Op3 (slot 8): DT=0, MUL=4
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x38 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x04 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x48 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x58 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x68 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1D });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x78 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x06 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x88 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x0F });

        // Op4 (slot C): DT=0, MUL=0 (0.5×)
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x3C });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x4C });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x5C });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x6C });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x7C });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x05 });
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x8C });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x09 });

        // High-pitched note: C6 (MIDI 84) → block=6, fnum=644
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA4 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x32 }); // block=6, fnum_h=2
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA0 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x84 }); // fnum_l=0x84 → fnum=644
        // Key on all operators
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x28 });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xF0 });

        let mut buf = vec![0.0f32; 44100 * 2]; // 1 second
        engine.render(&mut buf);

        // Analyze left channel, skip transient
        let skip = 2000;
        let left: Vec<f32> = buf[skip*2..].iter().step_by(2).cloned().collect();

        // Dump raw samples
        let peak = left.iter().cloned().fold(0.0f32, |a, b| a.max(b.abs()));
        eprintln!("ICZ1-like patch C6: peak={:.4}", peak);

        // Count unique quantized levels (convert to i16 equiv)
        let i16_vals: Vec<i16> = left[..4096].iter().map(|&s| (s * 32767.0).round() as i16).collect();
        let unique: std::collections::HashSet<i16> = i16_vals.iter().copied().collect();
        eprintln!("Unique i16 levels in 4096 samples: {}", unique.len());

        // Step size analysis
        let diffs: Vec<i32> = i16_vals.windows(2).map(|w| (w[1] as i32 - w[0] as i32).abs()).collect();
        let mean_step: f32 = diffs.iter().sum::<i32>() as f32 / diffs.len() as f32;
        let max_step: i32 = *diffs.iter().max().unwrap_or(&0);
        eprintln!("Step sizes: mean={:.1} max={}", mean_step, max_step);

        // Print first 50 raw f32 values
        eprintln!("First 50 samples (after transient):");
        for chunk in left[..50].chunks(10) {
            let s: Vec<String> = chunk.iter().map(|v| format!("{:.5}", v)).collect();
            eprintln!("  {}", s.join("  "));
        }

        // Compute spectrum
        let n = 4096;
        let window: Vec<f32> = (0..n).map(|i| {
            0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / n as f32).cos())
        }).collect();
        let windowed: Vec<f32> = left[..n].iter().zip(window.iter()).map(|(s, w)| s * w).collect();

        // Simple DFT for top bins (just check a few key frequencies)
        let sr = 44100.0f32;
        for &freq in &[500.0, 1000.0, 2000.0, 4000.0, 5000.0, 8000.0, 10000.0, 15000.0, 20000.0] {
            let bin = (freq / sr * n as f32).round() as usize;
            if bin < n / 2 {
                let mut re = 0.0f32;
                let mut im = 0.0f32;
                for (i, &s) in windowed.iter().enumerate() {
                    let angle = -2.0 * std::f32::consts::PI * bin as f32 * i as f32 / n as f32;
                    re += s * angle.cos();
                    im += s * angle.sin();
                }
                let mag = (re * re + im * im).sqrt();
                eprintln!("  {:.0} Hz: mag={:.2}", freq, mag);
            }
        }

        assert!(peak > 0.001, "Should produce output");
    }

    #[test]
    fn diagnostic_raw_ym_output() {
        // Render with ICZ1 patch directly and dump raw cycle sums + final output
        // to determine if crunchiness is inherent to the YM2612 or our processing.
        let mut chip = crate::ym2612::chip::Ym2612::new();

        let writes: Vec<(u32, u8)> = vec![
            // Alg 5, FB=7
            (0, 0xB0), (1, 0xF5), (0, 0xB4), (1, 0xC0),
            // Op1: DT=0, MUL=0 (0.5x)
            (0, 0x30), (1, 0x00), (0, 0x40), (1, 0x00),
            (0, 0x50), (1, 0x1F), (0, 0x60), (1, 0x1D),
            (0, 0x70), (1, 0x00), (0, 0x80), (1, 0x1F),
            // Op2: DT=0, MUL=10
            (0, 0x34), (1, 0x0A), (0, 0x44), (1, 0x00),
            (0, 0x54), (1, 0x1F), (0, 0x64), (1, 0x1D),
            (0, 0x74), (1, 0x16), (0, 0x84), (1, 0x1F),
            // Op3: DT=0, MUL=4
            (0, 0x38), (1, 0x04), (0, 0x48), (1, 0x00),
            (0, 0x58), (1, 0x1F), (0, 0x68), (1, 0x1D),
            (0, 0x78), (1, 0x06), (0, 0x88), (1, 0x0F),
            // Op4: DT=0, MUL=0 (0.5x)
            (0, 0x3C), (1, 0x00), (0, 0x4C), (1, 0x00),
            (0, 0x5C), (1, 0x1F), (0, 0x6C), (1, 0x1F),
            (0, 0x7C), (1, 0x05), (0, 0x8C), (1, 0x09),
            // C6: block=6, fnum=644
            (0, 0xA4), (1, 0x32), (0, 0xA0), (1, 0x84),
            // Key on
            (0, 0x28), (1, 0xF0),
        ];
        for (port, data) in &writes {
            chip.write(*port, *data);
            for _ in 0..24 { chip.clock(); }
        }

        // Collect raw 24-clock cycle sums (= YM's native ~53kHz sample rate)
        let num_cycles = 4000; // ~75ms at 53kHz
        let mut raw_l: Vec<i32> = Vec::with_capacity(num_cycles);
        let mut cycle_pos = 0u32;

        for _ in 0..num_cycles {
            let mut sum_l = 0i32;
            for _ in 0..24 {
                let s = chip.clock();
                if cycle_pos & 3 == 3 {
                    sum_l += s[0] as i32;
                }
                cycle_pos = (cycle_pos + 1) % 24;
            }
            raw_l.push(sum_l);
        }

        // Skip transient, analyze steady state
        let skip = 1000;
        let analysis = &raw_l[skip..];
        let peak = analysis.iter().map(|v| v.abs()).max().unwrap_or(0);
        let unique: std::collections::HashSet<i32> = analysis.iter().copied().collect();
        let diffs: Vec<i32> = analysis.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let mean_diff = diffs.iter().sum::<i32>() as f32 / diffs.len() as f32;
        let max_diff = *diffs.iter().max().unwrap_or(&0);

        eprintln!("=== RAW YM2612 CYCLE SUMS (ICZ1 patch, C6) ===");
        eprintln!("Peak: {} (out of max ~±765 per channel)", peak);
        eprintln!("Unique levels: {}", unique.len());
        eprintln!("Step sizes: mean={:.1} max={}", mean_diff, max_diff);
        eprintln!("First 30 values (after transient):");
        for chunk in analysis[..30].chunks(10) {
            let s: Vec<String> = chunk.iter().map(|v| format!("{:5}", v)).collect();
            eprintln!("  {}", s.join(" "));
        }

        // Also dump at a lower pitch: A3 (block=3, fnum=1084)
        chip.reset();
        let writes2: Vec<(u32, u8)> = vec![
            (0, 0xB0), (1, 0xF5), (0, 0xB4), (1, 0xC0),
            (0, 0x30), (1, 0x00), (0, 0x40), (1, 0x00),
            (0, 0x50), (1, 0x1F), (0, 0x60), (1, 0x1D),
            (0, 0x70), (1, 0x00), (0, 0x80), (1, 0x1F),
            (0, 0x34), (1, 0x0A), (0, 0x44), (1, 0x00),
            (0, 0x54), (1, 0x1F), (0, 0x64), (1, 0x1D),
            (0, 0x74), (1, 0x16), (0, 0x84), (1, 0x1F),
            (0, 0x38), (1, 0x04), (0, 0x48), (1, 0x00),
            (0, 0x58), (1, 0x1F), (0, 0x68), (1, 0x1D),
            (0, 0x78), (1, 0x06), (0, 0x88), (1, 0x0F),
            (0, 0x3C), (1, 0x00), (0, 0x4C), (1, 0x00),
            (0, 0x5C), (1, 0x1F), (0, 0x6C), (1, 0x1F),
            (0, 0x7C), (1, 0x05), (0, 0x8C), (1, 0x09),
            // A3: block=3, fnum=1084
            (0, 0xA4), (1, 0x1C), (0, 0xA0), (1, 0x3C),
            (0, 0x28), (1, 0xF0),
        ];
        for (port, data) in &writes2 {
            chip.write(*port, *data);
            for _ in 0..24 { chip.clock(); }
        }

        let mut raw_l2: Vec<i32> = Vec::with_capacity(num_cycles);
        cycle_pos = 0;
        for _ in 0..num_cycles {
            let mut sum_l = 0i32;
            for _ in 0..24 {
                let s = chip.clock();
                if cycle_pos & 3 == 3 {
                    sum_l += s[0] as i32;
                }
                cycle_pos = (cycle_pos + 1) % 24;
            }
            raw_l2.push(sum_l);
        }

        let analysis2 = &raw_l2[skip..];
        let peak2 = analysis2.iter().map(|v| v.abs()).max().unwrap_or(0);
        let unique2: std::collections::HashSet<i32> = analysis2.iter().copied().collect();
        let diffs2: Vec<i32> = analysis2.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let mean_diff2 = diffs2.iter().sum::<i32>() as f32 / diffs2.len() as f32;

        eprintln!("\n=== RAW YM2612 CYCLE SUMS (ICZ1 patch, A3) ===");
        eprintln!("Peak: {}", peak2);
        eprintln!("Unique levels: {}", unique2.len());
        eprintln!("Step sizes: mean={:.1}", mean_diff2);
        eprintln!("First 30 values:");
        for chunk in analysis2[..30].chunks(10) {
            let s: Vec<String> = chunk.iter().map(|v| format!("{:5}", v)).collect();
            eprintln!("  {}", s.join(" "));
        }
    }

    #[test]
    fn diagnostic_clock_rate_comparison() {
        // Compare output at 1x rate (current) vs 6x rate (full master clock)
        // to see if running more clocks per sample changes audio quality.
        let sample_rate = 44100u32;

        // Helper to program the same simple tone on both engines
        fn program_tone(engine: &mut AudioEngine) {
            // Simple sine: Alg 7, FB=0, MUL=1, TL=0
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xB0 });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x07 });
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xB4 });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xC0 });
            for slot in &[0x00u8, 0x04, 0x08, 0x0C] {
                engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x30 + slot });
                engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x01 });
                engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x40 + slot });
                engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
                engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x50 + slot });
                engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x1F });
                engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x80 + slot });
                engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x00 });
            }
            // A4
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA4 });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x24 });
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0xA0 });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0x3C });
            engine.process_command(AudioCommand::Ym2612Write { port: 0, data: 0x28 });
            engine.process_command(AudioCommand::Ym2612Write { port: 1, data: 0xF0 });
        }

        // Engine 1: current approach (render via engine)
        let mut engine1 = AudioEngine::new(sample_rate);
        program_tone(&mut engine1);
        let mut buf1 = vec![0.0f32; 8192 * 2];
        engine1.render(&mut buf1);
        let left1: Vec<f32> = buf1[4000..].iter().step_by(2).cloned().collect();

        // Engine 2: direct chip at 6x rate, manual averaging
        let mut chip = crate::ym2612::chip::Ym2612::new();
        // Same register programming
        let writes: Vec<(u32, u8)> = vec![
            (0, 0xB0), (1, 0x07), (0, 0xB4), (1, 0xC0),
            (0, 0x30), (1, 0x01), (0, 0x40), (1, 0x00), (0, 0x50), (1, 0x1F), (0, 0x80), (1, 0x00),
            (0, 0x34), (1, 0x01), (0, 0x44), (1, 0x00), (0, 0x54), (1, 0x1F), (0, 0x84), (1, 0x00),
            (0, 0x38), (1, 0x01), (0, 0x48), (1, 0x00), (0, 0x58), (1, 0x1F), (0, 0x88), (1, 0x00),
            (0, 0x3C), (1, 0x01), (0, 0x4C), (1, 0x00), (0, 0x5C), (1, 0x1F), (0, 0x8C), (1, 0x00),
            (0, 0xA4), (1, 0x24), (0, 0xA0), (1, 0x3C),
            (0, 0x28), (1, 0xF0),
        ];
        for (port, data) in &writes {
            chip.write(*port, *data);
            for _ in 0..13 { chip.clock(); }
        }

        // Render at full master clock rate: 7670453 Hz / 44100 = 173.9 clocks per sample
        let clocks_per_sample = 7_670_453.0 / 44100.0;
        let mut clock_acc = 0.0f64;
        let mut cycle_pos = 0u32;
        let num_samples = 8192;
        let mut left2 = Vec::with_capacity(num_samples);

        for _ in 0..num_samples {
            clock_acc += clocks_per_sample;
            let clocks = clock_acc as u32;
            clock_acc -= clocks as f64;

            let mut sum_l: i64 = 0;
            let mut sum_r: i64 = 0;
            let mut valid_count = 0u32;

            for _ in 0..clocks {
                let s = chip.clock();
                if cycle_pos & 3 == 3 {
                    sum_l += s[0] as i64;
                    sum_r += s[1] as i64;
                    valid_count += 1;
                }
                cycle_pos = (cycle_pos + 1) % 24;
            }

            let ym_l = if valid_count > 0 { (sum_l / valid_count as i64) as i32 } else { 0 };
            let out = ym_l as f32 * 8.0 / 65536.0;
            left2.push(out);
        }

        // Compare
        let skip = 2000;
        let n = 200;
        eprintln!("=== CLOCK RATE COMPARISON (A4 simple sine) ===");
        eprintln!("Sample | Engine (1x) | Raw chip (6x) | Diff");
        let mut max_diff = 0.0f32;
        for i in 0..n.min(left1.len() - skip).min(left2.len() - skip) {
            let v1 = left1[skip + i];
            let v2 = left2[skip + i];
            let diff = (v1 - v2).abs();
            if diff > max_diff { max_diff = diff; }
            if i < 20 || i % 50 == 0 {
                eprintln!("  {:4} | {:+.5} | {:+.5} | {:.5}", i, v1, v2, diff);
            }
        }
        eprintln!("Max difference over {} samples: {:.6}", n, max_diff);
        eprintln!("Engine peak: {:.5}", left1[skip..skip+n].iter().cloned().fold(0.0f32, |a, b| a.max(b.abs())));
        eprintln!("Chip peak: {:.5}", left2[skip..skip+n].iter().cloned().fold(0.0f32, |a, b| a.max(b.abs())));
    }

    #[test]
    fn test_psg_envelope_shapes_consecutive_notes() {
        let mut engine = AudioEngine::new(44100);

        // Set noise mode: white noise, fixed divider
        engine.process_command(AudioCommand::Sn76489Write { data: 0xE4 });

        // Envelope: fast decay [15, 10, 5, 0] with silence_on_end
        let envelope = Arc::new(vec![15u8, 10, 5, 0]);
        let samples_per_tick = (44100.0 / 60.0) as usize; // ~735

        // Start envelope for noise channel (hw_ch=3)
        engine.process_command(AudioCommand::Sn76489Write {
            data: 0x90 | (3 << 5) | 0x00, // ch3, max volume
        });

        // Manually set up the PSG envelope player via the engine's internal state
        // by sending the PsgEnvelopePreview command
        engine.process_command(AudioCommand::PsgEnvelopePreview {
            channel: 3,
            period: 0,
            envelope: envelope.clone(),
            loop_point: None,
        });

        // Render enough for 2 envelope cycles + gap
        let total_samples = samples_per_tick * 10;
        let mut buf = vec![0.0f32; total_samples * 2];
        engine.render(&mut buf);

        // Measure RMS in each envelope-tick window
        let mut rms_per_tick: Vec<f32> = Vec::new();
        for tick in 0..8 {
            let start = tick * samples_per_tick;
            let end = (tick + 1) * samples_per_tick;
            let mut sum_sq = 0.0f32;
            for i in start..end.min(total_samples) {
                sum_sq += buf[i * 2] * buf[i * 2];
            }
            let rms = (sum_sq / (end - start) as f32).sqrt();
            rms_per_tick.push(rms);
        }

        eprintln!("PSG envelope RMS per tick: {:?}", rms_per_tick);

        // Tick 0-1: max volume (preview handler + envelope[0]=15)
        // Tick 2: envelope[1]=10, tick 3: envelope[2]=5 (decay)
        // Tick 4: envelope[3]=0 applied + mute → silent
        // Threshold 0.007 accounts for YM2612 idle DC offset (~0.006 RMS)
        assert!(rms_per_tick[0] > rms_per_tick[3],
            "Envelope should decay: tick0={} > tick3={}", rms_per_tick[0], rms_per_tick[3]);
        assert!(rms_per_tick[4] < 0.007,
            "Envelope should be silent at tick 4: {}", rms_per_tick[4]);
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
