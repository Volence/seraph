use std::sync::Arc;
use crate::audio::command::AudioCommand;
use crate::ym2612::Ym2612;
use crate::sn76489::Sn76489;

const YM2612_MASTER_CLOCK: f64 = 7_670_453.0;
const SN76489_CLOCK_DIVIDER: f64 = 16.0;

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
}

impl AudioEngine {
    pub fn new(sample_rate: u32) -> Self {
        let sample_rate_f = sample_rate as f64;
        let ym_clocks_per_sample = YM2612_MASTER_CLOCK / sample_rate_f;
        let psg_clocks_per_sample = YM2612_MASTER_CLOCK / SN76489_CLOCK_DIVIDER / sample_rate_f;

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
        }
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
                self.ym_clock_accumulator = 0.0;
                self.psg_clock_accumulator = 0.0;
                self.dac_samples = None;
                self.dac_position = 0.0;
                self.psg_preview_envelope = None;
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
            // --- YM2612 ---
            // Accumulate fractional YM clocks and drain whole ticks.
            self.ym_clock_accumulator += self.ym_clocks_per_sample;
            let ym_ticks = self.ym_clock_accumulator as u32;
            self.ym_clock_accumulator -= ym_ticks as f64;

            let mut ym_l: i32 = 0;
            let mut ym_r: i32 = 0;

            for _ in 0..ym_ticks {
                let s = self.ym2612.clock();
                // Use the last sample from this render window rather than averaging.
                // The nuked-opm output is already a filtered DAC output; averaging
                // consecutive samples at master-clock rate causes destructive interference
                // on FM fundamentals. Nearest-neighbour decimation preserves signal level.
                ym_l = s[0] as i32;
                ym_r = s[1] as i32;
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
            let fm_scale: i32 = 32;
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

            // --- PSG envelope stepping ---
            if let Some(ref envelope) = self.psg_preview_envelope.clone() {
                self.psg_preview_tick_acc += 1.0;
                if self.psg_preview_tick_acc >= self.psg_preview_samples_per_tick {
                    self.psg_preview_tick_acc -= self.psg_preview_samples_per_tick;
                    if self.psg_preview_index < envelope.len() {
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
            }

            // --- Mix and normalize ---
            let scaled_l = ym_l * fm_scale + psg_sample + dac_sample;
            let scaled_r = ym_r * fm_scale + psg_sample + dac_sample;

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
        // The nuked-opm emulator has a small constant bias (~3) at idle — this is
        // accurate hardware behaviour. After FM scaling (×32), the idle level is
        // ~96/32768 ≈ 0.003. We check that no significant audio is produced.
        for &s in &buf {
            assert!(
                s.abs() < 0.01,
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
        let tail_signal = tail.iter().any(|s| s.abs() > 0.01);
        assert!(!tail_signal, "DAC should stop after samples exhausted");
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

        let any_loud_after = buf_after.iter().any(|s| s.abs() >= 0.01);
        assert!(
            !any_loud_after,
            "after Panic, all output should be near-silent (< 0.01); loudest was {:.6}",
            buf_after.iter().cloned().fold(0.0f32, f32::max)
        );
    }
}
