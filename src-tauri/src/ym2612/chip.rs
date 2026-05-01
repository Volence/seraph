use super::bindings::*;

pub struct Ym2612 {
    chip: Box<ym3438_t>,
}

impl Ym2612 {
    pub fn new() -> Self {
        unsafe {
            OPN2_SetChipType(YM3438_MODE_YM2612);
        }
        let mut instance = Ym2612 {
            chip: Box::new(ym3438_t::zeroed()),
        };
        instance.reset();
        instance
    }

    pub fn reset(&mut self) {
        unsafe {
            OPN2_Reset(self.chip.as_mut());
        }
    }

    pub fn write(&mut self, port: u32, data: u8) {
        unsafe {
            OPN2_Write(self.chip.as_mut(), port, data);
        }
    }

    pub fn clock(&mut self) -> [i16; 2] {
        let mut buf = [0i16; 2];
        unsafe {
            OPN2_Clock(self.chip.as_mut(), buf.as_mut_ptr());
        }
        buf
    }

    pub fn read_status(&mut self) -> u8 {
        unsafe { OPN2_Read(self.chip.as_mut(), 0) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_reset() {
        let mut chip = Ym2612::new();
        chip.reset();
        let status = chip.read_status();
        assert_eq!(status, 0);
    }

    #[test]
    fn test_fm_tone_produces_output() {
        let mut chip = Ym2612::new();

        // Algorithm 7, feedback 0
        chip.write(0, 0xB0); chip.write(1, 0x07);
        // Op1 TL = 0 (max volume)
        chip.write(0, 0x40); chip.write(1, 0x00);
        // Op1 AR = 31 (fastest attack)
        chip.write(0, 0x50); chip.write(1, 0x1F);
        // Op1 SL=0, RR=0 (sustain forever)
        chip.write(0, 0x80); chip.write(1, 0x00);
        // Op1 DT=0, MUL=1
        chip.write(0, 0x30); chip.write(1, 0x01);
        // Frequency ~440Hz (block 4, F-num)
        chip.write(0, 0xA4); chip.write(1, 0x22);
        chip.write(0, 0xA0); chip.write(1, 0x8D);
        // Key on: all operators, channel 0
        chip.write(0, 0x28); chip.write(1, 0xF0);

        // Clock 10000 times and collect output
        let mut any_nonzero = false;
        for _ in 0..10000 {
            let sample = chip.clock();
            if sample[0] != 0 || sample[1] != 0 {
                any_nonzero = true;
            }
        }

        assert!(any_nonzero, "Expected non-zero audio output after FM tone programming and key-on");
    }
}
