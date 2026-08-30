const FM_FNUM_TABLE: [u16; 12] = [
    644,  // C
    683,  // C#
    723,  // D
    766,  // D#
    813,  // E
    860,  // F
    911,  // F#
    965,  // G
    1023, // G#
    1084, // A
    1148, // A#
    1216, // B
];

pub fn midi_to_fm_freq(midi_note: u8) -> (u8, u16) {
    let semitone = (midi_note % 12) as usize;
    let octave = midi_note / 12;
    let block = octave.saturating_sub(1).min(7);
    (block, FM_FNUM_TABLE[semitone])
}

// SMPS PSG period lookup — matches Flamedriver/S3K Z80 driver exactly.
// 84 entries for z80_index 0-83, mapped to MIDI 36-119 via (midi = z80_index + 36).
const PSG_PERIOD_TABLE: [u16; 84] = [
    0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3FF,0x3F7,0x3BE,0x388,
    0x356,0x326,0x2F9,0x2CE,0x2A5,0x280,0x25C,0x23A,0x21A,0x1FB,0x1DF,0x1C4,
    0x1AB,0x193,0x17D,0x167,0x153,0x140,0x12E,0x11D,0x10D,0x0FE,0x0EF,0x0E2,
    0x0D6,0x0C9,0x0BE,0x0B4,0x0A9,0x0A0,0x097,0x08F,0x087,0x07F,0x078,0x071,
    0x06B,0x065,0x05F,0x05A,0x055,0x050,0x04B,0x047,0x043,0x040,0x03C,0x039,
    0x036,0x033,0x030,0x02D,0x02B,0x028,0x026,0x024,0x022,0x020,0x01F,0x01D,
    0x01B,0x01A,0x018,0x017,0x016,0x015,0x013,0x012,0x011,0x010,0x000,0x000,
];

pub fn fm_freq_to_midi(block: u8, fnum: u16) -> (u8, i16) {
    let mut best_idx = 0;
    let mut best_diff = i32::MAX;
    for (i, &table_fnum) in FM_FNUM_TABLE.iter().enumerate() {
        let diff = (fnum as i32 - table_fnum as i32).abs();
        if diff < best_diff {
            best_diff = diff;
            best_idx = i;
        }
    }
    let midi = ((block as u16 + 1) * 12 + best_idx as u16).min(127) as u8;
    let detune = fnum as i16 - FM_FNUM_TABLE[best_idx] as i16;
    (midi, detune)
}

pub fn psg_period_to_midi(period: u16) -> u8 {
    if period == 0 {
        return 119;
    }
    let mut best_idx = 0;
    let mut best_diff = u32::MAX;
    for (i, &table_period) in PSG_PERIOD_TABLE.iter().enumerate() {
        if table_period == 0 {
            continue;
        }
        let diff = (period as i32 - table_period as i32).unsigned_abs();
        if diff < best_diff {
            best_diff = diff;
            best_idx = i;
        }
    }
    ((best_idx as u16) + 36).min(127) as u8
}

pub fn midi_to_psg_period(midi_note: u8) -> u16 {
    let idx = midi_note as i16 - 36;
    if idx >= 0 && idx < 84 {
        PSG_PERIOD_TABLE[idx as usize]
    } else if idx < 0 {
        1023
    } else {
        0
    }
}

/// The lowest and highest MIDI notes this driver's PSG table can represent.
/// Outside this range `midi_to_psg_period` CLAMPS: below `PSG_MIDI_LOW` every
/// note returns the same period as the bottom of the table, and above
/// `PSG_MIDI_HIGH` it returns the top. The note is not silenced -- it is
/// silently RETUNED, which is worse, because the app and the export agree
/// with each other while neither matches what the author wrote.
pub const PSG_MIDI_LOW: u8 = 36;
pub const PSG_MIDI_HIGH: u8 = 119;

/// Whether this pitch is representable on PSG without being retuned.
pub fn psg_pitch_is_representable(midi_note: u8) -> bool {
    (PSG_MIDI_LOW..=PSG_MIDI_HIGH).contains(&midi_note)
}

#[cfg(test)]
mod tests {
    /// Last-ditch location of the S3K disassembly, used only when git cannot
    /// answer. Relative to the process's working directory, which for
    /// `cargo test` is `src-tauri/` in the MAIN checkout — see
    /// `skdisasm_dir` for why that is not good enough on its own.
    const SKDISASM_FALLBACK: &str = "../../skdisasm";

    /// Where to look for the S3K disassembly, overridable with
    /// `SERAPH_SKDISASM_DIR`.
    ///
    /// `skdisasm/` is a SIBLING of this repo, not part of it, so reaching it
    /// means leaving the checkout. The old default counted `..` hops
    /// (`../../skdisasm` from `src-tauri/`), which is right from the main
    /// checkout at `<parent>/seraph/src-tauri` and WRONG from an agent
    /// worktree at `<parent>/seraph/.claude/worktrees/<agent>/src-tauri`,
    /// where the same two hops land in `.claude/worktrees/` (audit F39). The
    /// hop count is a property of where the checkout happens to sit, so it
    /// cannot be a constant.
    ///
    /// `git rev-parse --show-toplevel` does NOT fix this: inside a linked
    /// worktree that reports the *worktree's* root, which is the wrong
    /// directory again. `--git-common-dir` is the one thing every worktree
    /// shares — it resolves to `<main checkout>/.git` from the main checkout
    /// and from every worktree alike — so its parent is the repo and its
    /// grandparent is the directory the sibling disassembly lives in.
    ///
    /// Returns `None` rather than guessing when git is unavailable or the
    /// layout is unexpected; the caller then falls back and, failing that,
    /// panics with instructions. An unreachable source must never be a pass.
    fn skdisasm_dir() -> Option<std::path::PathBuf> {
        // Anchored at the crate, not at the process's cwd, so the answer does
        // not depend on where the test binary was launched from.
        const CRATE_DIR: &str = env!("CARGO_MANIFEST_DIR");
        let out = std::process::Command::new("git")
            .args(["rev-parse", "--git-common-dir"])
            .current_dir(CRATE_DIR)
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let raw = String::from_utf8(out.stdout).ok()?;
        let raw = std::path::Path::new(raw.trim());
        // Older gits print a path relative to the cwd we handed them.
        let common = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            std::path::Path::new(CRATE_DIR).join(raw)
        };
        let common = std::fs::canonicalize(common).ok()?;
        Some(common.parent()?.parent()?.join("skdisasm"))
    }

    /// Recomputes `PSG_PERIOD_TABLE` from the DRIVER'S OWN SOURCE and fails on
    /// any disagreement.
    ///
    /// The table is a transcription -- its comment says "matches
    /// Flamedriver/S3K Z80 driver exactly" -- and a transcription can drift
    /// from the thing it claims to match without anything failing. Nothing
    /// checked that claim until this test.
    ///
    /// The driver does not store periods at all: it stores FREQUENCIES IN HZ
    /// and computes periods at assembly time with
    /// `zMakePSGFrequency = min(3FFh, round(PSG_Sample_Rate / (frequency*2)))`,
    /// where `PSG_Sample_Rate = Z80_Clock/16` and `Z80_Clock = Master_Clock/15`.
    /// So this test parses the Hz list and the clock constants out of the
    /// disassembly and applies the driver's own formula, rather than comparing
    /// one copied table against another copied table -- which would prove
    /// nothing.
    ///
    /// Follows F32's rule: a test that cannot reach its input FAILS with
    /// instructions rather than passing silently. Skipping needs
    /// `SERAPH_SKIP_ROM_TESTS` set deliberately, the same switch the ROM tests
    /// use, so there is one knob rather than two.
    #[test]
    fn psg_table_still_matches_the_driver_it_claims_to_match() {
        let dir = match std::env::var("SERAPH_SKDISASM_DIR") {
            Ok(explicit) => std::path::PathBuf::from(explicit),
            Err(_) => skdisasm_dir()
                .unwrap_or_else(|| std::path::PathBuf::from(SKDISASM_FALLBACK)),
        };
        let driver = dir.join("Sound/Z80 Sound Driver.asm");
        let consts = dir.join("sonic3k.constants.asm");
        if !driver.exists() || !consts.exists() {
            if std::env::var("SERAPH_SKIP_ROM_TESTS").is_ok() {
                eprintln!("SERAPH_SKIP_ROM_TESTS set; skipping PSG drift check ({})", dir.display());
                return;
            }
            panic!(
                "S3K disassembly not found at {}.\n\
                 PSG_PERIOD_TABLE claims to match that driver and this test is the only \
                 thing that checks it, so it FAILS rather than passing silently.\n\
                 Set SERAPH_SKDISASM_DIR, or SERAPH_SKIP_ROM_TESTS=1 to skip deliberately.",
                dir.display(),
            );
        }

        let consts_src = std::fs::read_to_string(&consts).expect("read constants");
        let master: u64 = consts_src
            .lines()
            .find_map(|l| l.trim().strip_prefix("Master_Clock")?.split('=').nth(1))
            .and_then(|v| v.split(';').next())
            .and_then(|v| v.trim().parse().ok())
            .expect("Master_Clock not found in the disassembly's constants");
        // Integer division, matching the assembler's own expressions.
        let psg_sample_rate = (master / 15) / 16;

        let driver_src = std::fs::read_to_string(&driver).expect("read driver");
        let block = driver_src
            .split("zPSGFrequencies:")
            .nth(1)
            .expect("zPSGFrequencies label not found")
            .split("; ---")
            .next()
            .unwrap();
        let freqs: Vec<f64> = block
            .lines()
            .filter_map(|l| l.trim().strip_prefix("zMakePSGFrequencies"))
            .flat_map(|args| args.split(','))
            .filter_map(|t| t.trim().parse::<f64>().ok())
            .collect();

        assert_eq!(
            freqs.len(),
            PSG_PERIOD_TABLE.len(),
            "parsed {} frequencies from the driver but the table has {} entries; \
             the parse is wrong or the driver's table changed shape",
            freqs.len(),
            PSG_PERIOD_TABLE.len(),
        );

        let mut mismatches = Vec::new();
        for (i, (&f, &ours)) in freqs.iter().zip(PSG_PERIOD_TABLE.iter()).enumerate() {
            // AS `roundFloatToInteger` is round-half-away-from-zero.
            let computed = ((psg_sample_rate as f64 / (f * 2.0)) + 0.5).floor() as u32;
            let expected = computed.min(0x3FF) as u16;
            if expected != ours {
                mismatches.push(format!("index {i}: driver {expected:#05x} vs table {ours:#05x}"));
            }
        }
        assert!(
            mismatches.is_empty(),
            "PSG_PERIOD_TABLE has DRIFTED from the driver it claims to match \
             ({} of {} entries): {:?}",
            mismatches.len(),
            PSG_PERIOD_TABLE.len(),
            mismatches,
        );
    }

    use super::*;

    #[test]
    fn test_midi_60_is_block_4() {
        let (block, _) = midi_to_fm_freq(60);
        assert_eq!(block, 4);
    }

    #[test]
    fn test_midi_48_is_block_3() {
        let (block, _) = midi_to_fm_freq(48);
        assert_eq!(block, 3);
    }

    #[test]
    fn test_midi_72_is_block_5() {
        let (block, _) = midi_to_fm_freq(72);
        assert_eq!(block, 5);
    }

    #[test]
    fn test_fm_a4_fnum_is_1084() {
        let (_, fnum) = midi_to_fm_freq(69);
        assert_eq!(fnum, 1084);
    }

    #[test]
    fn test_psg_a4_period_exact() {
        // MIDI 69 = A4, z80_index 33, table entry 0x0FE = 254
        assert_eq!(midi_to_psg_period(69), 0x0FE);
    }

    #[test]
    fn test_psg_below_table_clamps_to_1023() {
        assert_eq!(midi_to_psg_period(12), 1023);
        assert_eq!(midi_to_psg_period(35), 1023);
    }

    #[test]
    fn test_psg_table_boundaries() {
        assert_eq!(midi_to_psg_period(36), 0x3FF); // z80_index 0
        assert_eq!(midi_to_psg_period(119), 0x000); // z80_index 83
    }

    #[test]
    fn test_psg_period_decreases_with_pitch() {
        let p_low = midi_to_psg_period(48);
        let p_high = midi_to_psg_period(72);
        assert!(p_low > p_high);
    }

    #[test]
    fn test_fm_freq_to_midi_a4() {
        let (midi, detune) = fm_freq_to_midi(4, 1084);
        assert_eq!(midi, 69);
        assert_eq!(detune, 0);
    }

    #[test]
    fn test_fm_freq_to_midi_c5() {
        let (midi, detune) = fm_freq_to_midi(4, 644);
        assert_eq!(midi, 60);
        assert_eq!(detune, 0);
    }

    #[test]
    fn test_fm_freq_to_midi_detuned() {
        let (midi, detune) = fm_freq_to_midi(4, 650);
        assert_eq!(midi, 60);
        assert_eq!(detune, 6);
    }

    #[test]
    fn test_psg_period_to_midi_a4() {
        assert_eq!(psg_period_to_midi(0x0FE), 69);
    }

    #[test]
    fn test_psg_period_to_midi_zero() {
        assert_eq!(psg_period_to_midi(0), 119);
    }

    #[test]
    fn test_fm_freq_roundtrip() {
        for midi_note in 12..=107 {
            let (block, fnum) = midi_to_fm_freq(midi_note);
            let (recovered, detune) = fm_freq_to_midi(block, fnum);
            assert_eq!(recovered, midi_note, "roundtrip failed for MIDI {}", midi_note);
            assert_eq!(detune, 0, "roundtrip detune nonzero for MIDI {}", midi_note);
        }
    }
}
