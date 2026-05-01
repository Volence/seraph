// SN76489 PSG emulator — pure Rust, no FFI
//
// Hardware reference:
//   3 tone channels: 10-bit period counters, square-wave output
//   1 noise channel: 16-bit LFSR, selectable white/periodic
//   4 volume registers: 4-bit attenuation (0 = max, 15 = silent)
//
// Clock rate: PSG clocked at ~3.58 MHz on Genesis (master / 15).
// Callers are responsible for driving clock() at the correct rate
// and sampling render_sample() at the desired output rate.

/// -3 dB per step volume table (linear amplitude, 15 = silence).
const VOLUME_TABLE: [i16; 16] = [
    8191, 6507, 5168, 4105, 3261, 2590, 2057, 1634,
    1298, 1031,  819,  650,  516,  410,  326,    0,
];

/// LFSR tap mask for white noise: taps at bits 0 and 3.
const NOISE_TAPPED: u16 = 0x0009;

pub struct Sn76489 {
    // Tone channels 0-2
    tone_period:  [u16; 3],
    tone_counter: [u16; 3],
    tone_output:  [bool; 3],

    // Shared volume (index 3 = noise channel)
    volume: [u8; 4],   // 0 = loudest, 15 = silent

    // Noise channel
    noise_shift:   u16,
    noise_period:  u8,   // 0-3: 0→$10, 1→$20, 2→$40, 3→tone-ch2
    noise_white:   bool,
    noise_counter: u16,

    // Write-register latch
    latched_channel: u8,   // 0-3
    latched_type:    bool, // false = tone/noise, true = volume
}

impl Sn76489 {
    pub fn new() -> Self {
        Self {
            tone_period:  [0; 3],
            tone_counter: [0; 3],
            tone_output:  [false; 3],
            volume:       [0x0F; 4], // all channels muted
            noise_shift:  0x8000,
            noise_period: 0,
            noise_white:  false,
            noise_counter: 0,
            latched_channel: 0,
            latched_type:    false,
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Write a byte to the PSG register port.
    ///
    /// Latch byte (bit 7 = 1):
    ///   bits 6-5 = channel (0-3)
    ///   bit  4   = type (0 = tone/noise, 1 = volume)
    ///   bits 3-0 = data nibble
    ///
    /// Data byte (bit 7 = 0):
    ///   bits 5-0 = high 6 bits for tone period (combined with latched low 4)
    ///   bits 1-0 = noise period select, bit 2 = white/periodic flag
    pub fn write(&mut self, data: u8) {
        if data & 0x80 != 0 {
            // ---- Latch byte ----
            let channel  = (data >> 5) & 0x03;
            let is_vol   = (data >> 4) & 0x01 != 0;
            let nibble   = data & 0x0F;

            self.latched_channel = channel;
            self.latched_type    = is_vol;

            if is_vol {
                // Volume is fully specified by the latch byte itself.
                self.volume[channel as usize] = nibble;
            } else if channel == 3 {
                // Noise: low bits decoded immediately from latch byte.
                self.noise_period = nibble & 0x03;
                self.noise_white  = (nibble >> 2) & 0x01 != 0;
                // Reset the LFSR whenever the noise register is written.
                self.noise_shift = 0x8000;
            } else {
                // Tone: store low 4 bits; high bits arrive via data byte.
                let ch = channel as usize;
                self.tone_period[ch] = (self.tone_period[ch] & 0x03F0) | (nibble as u16);
            }
        } else {
            // ---- Data byte ----
            let channel = self.latched_channel as usize;
            let is_vol  = self.latched_type;

            if is_vol {
                // 4-bit volume update via data byte (low nibble).
                self.volume[channel] = data & 0x0F;
            } else if channel == 3 {
                // Noise: same decoding as latch byte (re-write full control bits).
                self.noise_period = data & 0x03;
                self.noise_white  = (data >> 2) & 0x01 != 0;
                self.noise_shift  = 0x8000;
            } else {
                // Tone: high 6 bits in bits 5-0 of data byte, preserve low 4.
                let low4 = self.tone_period[channel] & 0x000F;
                self.tone_period[channel] = ((data as u16 & 0x3F) << 4) | low4;
            }
        }
    }

    /// Advance the PSG by one PSG clock cycle.
    pub fn clock(&mut self) {
        // --- Tone channels ---
        for ch in 0..3 {
            if self.tone_counter[ch] > 0 {
                self.tone_counter[ch] -= 1;
            } else {
                // Reload from period (period = 0 treated as 0x400 for DC offset avoidance,
                // matching hardware behaviour where period 0 clocks at max rate).
                let period = if self.tone_period[ch] == 0 { 1 } else { self.tone_period[ch] };
                self.tone_counter[ch] = period;
                self.tone_output[ch] = !self.tone_output[ch];
            }
        }

        // --- Noise channel ---
        let noise_div: u16 = match self.noise_period {
            0 => 0x10,
            1 => 0x20,
            2 => 0x40,
            _ => {
                // Period 3: use tone channel 2's period.
                let p = self.tone_period[2];
                if p == 0 { 1 } else { p }
            }
        };

        if self.noise_counter > 0 {
            self.noise_counter -= 1;
        } else {
            self.noise_counter = noise_div;

            // Compute feedback bit.
            let feedback: u16 = if self.noise_white {
                // White noise: parity of tapped bits.
                let tapped = self.noise_shift & NOISE_TAPPED;
                (tapped.count_ones() & 1) as u16
            } else {
                // Periodic noise: bit 0 of shift register.
                self.noise_shift & 0x0001
            };

            // Shift right and insert feedback at bit 15.
            self.noise_shift = (self.noise_shift >> 1) | (feedback << 15);
        }
    }

    /// Mix all four channels and return a mono i16 sample.
    ///
    /// Tone output is high/low square wave; noise output is bit 0 of LFSR.
    /// Each channel contributes ±volume when active, 0 when low.
    /// We use a signed mix: +vol when output is high, 0 when low, matching
    /// common SN76489 implementations (single-sided output, no DC cancel needed).
    pub fn render_sample(&mut self) -> i16 {
        let mut sum: i32 = 0;

        // Tone channels: output high → add volume.
        for ch in 0..3 {
            if self.tone_output[ch] {
                sum += VOLUME_TABLE[self.volume[ch] as usize] as i32;
            }
        }

        // Noise channel: LFSR bit 0 high → add volume.
        if self.noise_shift & 0x0001 != 0 {
            sum += VOLUME_TABLE[self.volume[3] as usize] as i32;
        }

        // Clamp to i16 range.
        sum.clamp(i16::MIN as i32, i16::MAX as i32) as i16
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_is_silent() {
        let mut chip = Sn76489::new();
        // All volumes initialised to 0xF (silent) — sample must be 0.
        assert_eq!(chip.render_sample(), 0, "new chip must produce silence");
    }

    #[test]
    fn test_tone_produces_output() {
        let mut chip = Sn76489::new();
        // Set channel 0 volume to 0 (maximum).
        chip.write(0x90);         // latch: ch0, volume, value=0
        // Set channel 0 tone period: low nibble = 4.
        chip.write(0x80 | 4);     // latch: ch0, tone, low=4
        // Set high 6 bits = 6 → period = (6 << 4) | 4 = 100.
        chip.write(0x00 | 6);     // data byte: high=6

        // Clock 1000 cycles; collect samples.
        let mut has_nonzero = false;
        for _ in 0..1000 {
            chip.clock();
            if chip.render_sample() != 0 {
                has_nonzero = true;
            }
        }
        assert!(has_nonzero, "tone channel 0 at max volume must produce non-zero output");
    }

    #[test]
    fn test_volume_silence() {
        let mut chip = Sn76489::new();
        // Set channel 0 tone period (period=100, same as above).
        chip.write(0x80 | 4);     // latch: ch0, tone, low=4
        chip.write(0x00 | 6);     // data: high=6
        // Leave volume at 0xF (the default — silent).

        // Clock and verify every sample is zero.
        for _ in 0..1000 {
            chip.clock();
            assert_eq!(
                chip.render_sample(), 0,
                "channel with volume=0xF must be silent regardless of tone period"
            );
        }
    }
}
