use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

pub const FFT_SIZE: usize = 1024;
pub const SPECTRUM_BINS: usize = FFT_SIZE / 2;
const HOP_SIZE: usize = 1024;

pub struct SpectrumBuffer {
    pub bins: Vec<AtomicU32>,
    pub generation: AtomicU64,
}

impl SpectrumBuffer {
    pub fn new() -> Self {
        let neg96_bits = (-96.0f32).to_bits();
        Self {
            bins: (0..SPECTRUM_BINS).map(|_| AtomicU32::new(neg96_bits)).collect(),
            generation: AtomicU64::new(0),
        }
    }

    pub fn read_bin(&self, i: usize) -> f32 {
        f32::from_bits(self.bins[i].load(Ordering::Relaxed))
    }
}

pub struct SpectrumAnalyzer {
    ring: [f32; FFT_SIZE],
    write_pos: usize,
    samples_since_fft: usize,
    window: [f32; FFT_SIZE],
    twiddle_re: Vec<f32>,
    twiddle_im: Vec<f32>,
    fft_re: Vec<f32>,
    fft_im: Vec<f32>,
    buffer: Arc<SpectrumBuffer>,
}

impl SpectrumAnalyzer {
    pub fn new() -> Self {
        let mut window = [0.0f32; FFT_SIZE];
        for i in 0..FFT_SIZE {
            window[i] = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / FFT_SIZE as f32).cos());
        }

        let half = FFT_SIZE / 2;
        let mut twiddle_re = vec![0.0f32; half];
        let mut twiddle_im = vec![0.0f32; half];
        for k in 0..half {
            let angle = -2.0 * std::f32::consts::PI * k as f32 / FFT_SIZE as f32;
            twiddle_re[k] = angle.cos();
            twiddle_im[k] = angle.sin();
        }

        Self {
            ring: [0.0; FFT_SIZE],
            write_pos: 0,
            samples_since_fft: 0,
            window,
            twiddle_re,
            twiddle_im,
            fft_re: vec![0.0; FFT_SIZE],
            fft_im: vec![0.0; FFT_SIZE],
            buffer: Arc::new(SpectrumBuffer::new()),
        }
    }

    pub fn spectrum_buffer(&self) -> Arc<SpectrumBuffer> {
        Arc::clone(&self.buffer)
    }

    #[inline]
    pub fn push_sample(&mut self, sample: f32) {
        self.ring[self.write_pos] = sample;
        self.write_pos = (self.write_pos + 1) % FFT_SIZE;
        self.samples_since_fft += 1;

        if self.samples_since_fft >= HOP_SIZE {
            self.samples_since_fft = 0;
            self.compute_fft();
        }
    }

    fn compute_fft(&mut self) {
        let n = FFT_SIZE;
        for i in 0..n {
            let idx = (self.write_pos + i) % n;
            self.fft_re[i] = self.ring[idx] * self.window[i];
            self.fft_im[i] = 0.0;
        }

        // Bit-reversal permutation
        let mut j = 0usize;
        for i in 0..n {
            if i < j {
                self.fft_re.swap(i, j);
                self.fft_im.swap(i, j);
            }
            let mut m = n >> 1;
            while m >= 1 && j >= m {
                j -= m;
                m >>= 1;
            }
            j += m;
        }

        // Cooley-Tukey butterfly
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let step = n / len;
            for start in (0..n).step_by(len) {
                for k in 0..half {
                    let tw_idx = k * step;
                    let wr = self.twiddle_re[tw_idx];
                    let wi = self.twiddle_im[tw_idx];

                    let a = start + k;
                    let b = start + k + half;

                    let tr = self.fft_re[b] * wr - self.fft_im[b] * wi;
                    let ti = self.fft_re[b] * wi + self.fft_im[b] * wr;

                    self.fft_re[b] = self.fft_re[a] - tr;
                    self.fft_im[b] = self.fft_im[a] - ti;
                    self.fft_re[a] += tr;
                    self.fft_im[a] += ti;
                }
            }
            len <<= 1;
        }

        let scale = 2.0 / n as f32;
        for k in 0..SPECTRUM_BINS {
            let re = self.fft_re[k];
            let im = self.fft_im[k];
            let mag = (re * re + im * im).sqrt() * scale;
            let db = (20.0 * (mag + 1e-10).log10()).max(-96.0).min(0.0);
            self.buffer.bins[k].store(db.to_bits(), Ordering::Relaxed);
        }
        self.buffer.generation.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_produces_peak_at_correct_bin() {
        let mut sa = SpectrumAnalyzer::new();
        let freq = 1000.0f32;
        let sr = 44100.0f32;
        for i in 0..2048 {
            let sample = (2.0 * std::f32::consts::PI * freq * i as f32 / sr).sin() * 0.5;
            sa.push_sample(sample);
        }

        let expected_bin = (freq / sr * FFT_SIZE as f32).round() as usize;
        let peak_db = sa.buffer.read_bin(expected_bin);
        assert!(peak_db > -20.0, "Expected peak at bin {expected_bin}, got {peak_db} dB");

        let far_bin = expected_bin * 3;
        if far_bin < SPECTRUM_BINS {
            let noise_db = sa.buffer.read_bin(far_bin);
            assert!(noise_db < peak_db - 30.0, "Far bin should be much quieter");
        }
    }
}
