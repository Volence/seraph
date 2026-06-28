#![allow(non_camel_case_types, non_snake_case, dead_code)]

use std::os::raw::c_uint;

/// Chip-type mode flags (ym3438_mode_* from ym3438.h).
/// OPN2_SetChipType is a global setter — pass one of these.
pub const YM3438_MODE_YM2612: u32 = 0x01;   // YM2612 emulation (MD1, MD2 VA2)
pub const YM3438_MODE_READMODE: u32 = 0x02; // Status read on any port

/// Opaque representation of ym3438_t.
///
/// Actual size on x86-64: 1252 bytes (verified by sizeof(ym3438_t) at build time).
/// We use a 2048-byte buffer to stay safe across alignment/platform variations.
/// Must be repr(C) and at least as large as the C struct.
#[repr(C)]
pub struct ym3438_t {
    _data: [u8; 2048],
}

impl ym3438_t {
    /// Create a zeroed, uninitialised instance. Call OPN2_Reset before use.
    pub fn zeroed() -> Self {
        ym3438_t { _data: [0u8; 2048] }
    }
}

extern "C" {
    /// Reset the chip to power-on state.
    pub fn OPN2_Reset(chip: *mut ym3438_t);

    /// Set the global chip emulation mode (YM2612 vs YM3438 quirks).
    /// NOTE: this is a global setter — there is no per-chip pointer argument.
    pub fn OPN2_SetChipType(chip_type: c_uint);

    /// Advance the emulator by one internal clock step, producing one stereo sample pair.
    /// buffer must point to two consecutive i16 slots: [left, right].
    pub fn OPN2_Clock(chip: *mut ym3438_t, buffer: *mut i16);

    /// Write a byte to the chip. port 0/1 = address/data bank A, port 2/3 = bank B.
    pub fn OPN2_Write(chip: *mut ym3438_t, port: c_uint, data: u8);

    /// Drive the /TEST pin (used for diagnostic/test purposes).
    pub fn OPN2_SetTestPin(chip: *mut ym3438_t, value: c_uint);

    /// Read back the current /TEST pin state.
    pub fn OPN2_ReadTestPin(chip: *mut ym3438_t) -> c_uint;

    /// Read the /IRQ pin state (timer overflow indicator).
    pub fn OPN2_ReadIRQPin(chip: *mut ym3438_t) -> c_uint;

    /// Read a byte from the chip (status register etc.).
    pub fn OPN2_Read(chip: *mut ym3438_t, port: c_uint) -> u8;

    /// Read summed per-channel outputs with panning (bypasses DAC time-multiplexing).
    pub fn OPN2_ReadChannels(chip: *mut ym3438_t, left: *mut i16, right: *mut i16);
}
