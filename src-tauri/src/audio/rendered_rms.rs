//! Rendered-audio loudness harness.
//!
//! Renders a fixed note through the REAL sequencer playback path
//! (SequencerSnapshot -> Sequencer -> AudioEngine -> Nuked-OPN2 -> filters)
//! and measures peak / RMS amplitude of the produced samples. This is the
//! repo's ground truth for loudness questions: register-level checks have
//! already lied to us once (the OP_REG_OFFSETS / PACKED_OP_SLOTS split), so
//! any "voice X is too quiet / too loud" claim must be backed by numbers
//! from this harness, not by TL values.
//!
//! The `render_library_voice` helper loads a shipped library pack entry by
//! its repo-relative path (e.g. "batman-robin/fm/animal-boss-voice-151.json")
//! and renders it exactly as the DAW would play a note placed on an FM track.

use crate::audio::engine::AudioEngine;
use crate::model::instrument::FmInstrument;
use crate::sequencer::{
    ChannelSequence, ChannelType, InstrumentData, SequencerEvent, SequencerSnapshot,
};
use crate::audio::command::AudioCommand;

pub const SAMPLE_RATE: u32 = 44100;

#[derive(Debug, Clone, Copy)]
pub struct RenderStats {
    pub peak: f32,
    pub rms: f32,
}

/// Load the `instrument` payload of a library pack JSON entry.
/// `rel` is relative to the repo's `library/` directory.
pub fn load_pack_fm(rel: &str) -> FmInstrument {
    let path = format!("{}/../library/{}", env!("CARGO_MANIFEST_DIR"), rel);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read library entry {path}: {e}"));
    let v: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("parse {path}: {e}"));
    serde_json::from_value(v["instrument"].clone())
        .unwrap_or_else(|e| panic!("parse instrument in {path}: {e}"))
}

/// Render `render_secs` of one FM note through the actual playback path.
///
/// Mirrors production exactly: patch bytes come from `pack_patch()` (identical
/// to `FlamedriverProfile::fm_to_bytes`, carrier flags included), and the
/// sequencer applies its carrier vol_offset from `track_volume`/`velocity`
/// the same way `ProjectManager::build_sequencer_snapshot` playback does.
/// Returns the left-channel samples.
pub fn render_fm_note(
    inst: &FmInstrument,
    pitch: u8,
    velocity: u8,
    track_volume: u8,
    render_secs: f64,
) -> Vec<f32> {
    let bytes = inst.pack_patch();
    let ssg_eg = [
        inst.operators[0].ssg_eg,
        inst.operators[1].ssg_eg,
        inst.operators[2].ssg_eg,
        inst.operators[3].ssg_eg,
    ];
    let instrument = InstrumentData::FmPatch { bytes, ssg_eg };

    // 120 BPM, 480 tpb -> 960 ticks/sec. Hold the note for the whole render.
    let duration_ticks = (render_secs * 2.0 * 960.0) as u64;
    let snapshot = SequencerSnapshot {
        tempo_bpm: 120.0,
        ticks_per_beat: 480,
        loop_start: None,
        loop_end: None,
        channels: vec![ChannelSequence {
            channel_type: ChannelType::Fm(0),
            volume: track_volume,
            pan: 0xC0,
            modulation: None,
            noise_reg: 0xE4,
            events: vec![
                SequencerEvent::NoteOn {
                    tick: 0,
                    pitch,
                    velocity,
                    detune: 0,
                    duration_ticks,
                    instrument,
                    modulation: None,
                    pan_override: None,
                },
                SequencerEvent::NoteOff { tick: duration_ticks, pitch },
            ],
            overlaps: vec![],
        }],
    };

    let mut engine = AudioEngine::new(SAMPLE_RATE);
    engine.process_command(AudioCommand::LoadSequence { snapshot });
    engine.process_command(AudioCommand::TransportPlay);

    let frames = (render_secs * SAMPLE_RATE as f64) as usize;
    let mut buf = vec![0.0f32; frames * 2];
    engine.render(&mut buf);
    buf.iter().step_by(2).copied().collect()
}

/// Render `render_secs` of one FM note through the library-audition path:
/// the same raw register programming `fm_preview_writes` / `do_preview_fm`
/// performs (patch TLs written verbatim — no velocity or track-volume
/// attenuation), keyed on channel 0. Returns the left-channel samples.
pub fn render_fm_audition(inst: &FmInstrument, pitch: u8, render_secs: f64) -> Vec<f32> {
    use crate::audio::frequency::midi_to_fm_freq;
    use crate::model::instrument::PACKED_OP_SLOTS;

    let mut engine = AudioEngine::new(SAMPLE_RATE);
    let mut w = |addr: u8, data: u8| {
        engine.process_command(AudioCommand::Ym2612Write { port: 0, data: addr });
        engine.process_command(AudioCommand::Ym2612Write { port: 1, data });
    };
    w(0xB0, (inst.feedback << 3) | inst.algorithm);
    w(0xB4, 0xC0);
    for (i, op) in inst.operators.iter().enumerate() {
        let slot = PACKED_OP_SLOTS[i];
        w(0x30 + slot, (op.detune << 4) | op.multiple);
        w(0x40 + slot, op.total_level);
        w(0x50 + slot, (op.rate_scale << 6) | op.attack_rate);
        w(0x60 + slot, ((op.amp_mod as u8) << 7) | op.d1r);
        w(0x70 + slot, op.d2r);
        w(0x80 + slot, (op.sustain_level << 4) | op.release_rate);
    }
    let (block, fnum) = midi_to_fm_freq(pitch);
    w(0xA4, (block << 3) | ((fnum >> 8) as u8 & 0x07));
    w(0xA0, (fnum & 0xFF) as u8);
    w(0x28, 0xF0);

    let frames = (render_secs * SAMPLE_RATE as f64) as usize;
    let mut buf = vec![0.0f32; frames * 2];
    engine.render(&mut buf);
    buf.iter().step_by(2).copied().collect()
}

/// Peak and RMS over the sustained portion of a rendered note
/// (skips the first `skip_secs` to exclude the attack transient).
pub fn stats(samples: &[f32], skip_secs: f64) -> RenderStats {
    let skip = (skip_secs * SAMPLE_RATE as f64) as usize;
    let body = &samples[skip.min(samples.len())..];
    assert!(!body.is_empty(), "analysis window is empty");
    let peak = body.iter().fold(0.0f32, |a, &s| a.max(s.abs()));
    let rms = (body.iter().map(|&s| (s as f64) * (s as f64)).sum::<f64>()
        / body.len() as f64)
        .sqrt() as f32;
    RenderStats { peak, rms }
}

/// One-call convenience: library entry -> rendered stats.
pub fn render_library_voice(
    rel: &str,
    pitch: u8,
    velocity: u8,
    track_volume: u8,
) -> RenderStats {
    let inst = load_pack_fm(rel);
    let samples = render_fm_note(&inst, pitch, velocity, track_volume, 1.0);
    stats(&samples, 0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VOICES: &[&str] = &[
        "batman-robin/fm/animal-boss-voice-151.json",
        "batman-robin/fm/animal-boss-voice-92.json",
        "batman-robin/fm/animal-boss-voice-189.json",
        "batman-robin/fm/extreme-boss-voice-152.json",
        "batman-robin/fm/gotham-by-night-voice-132.json",
    ];

    /// Diagnostic table: rendered peak/RMS for voice 151 and comparator
    /// voices from the same pack, at DAW defaults (velocity 100, track
    /// volume 100) and at full level (127/127), across three pitches.
    /// Run with `cargo test rendered_loudness_table -- --nocapture`.
    #[test]
    fn rendered_loudness_table() {
        for &(vel, vol, label) in &[(100u8, 100u8, "defaults"), (127u8, 127u8, "full")] {
            eprintln!("=== velocity={vel} track_volume={vol} ({label}) ===");
            for &pitch in &[48u8, 60, 72] {
                for rel in VOICES {
                    let s = render_library_voice(rel, pitch, vel, vol);
                    let name = rel.rsplit('/').next().unwrap();
                    eprintln!(
                        "  pitch={pitch:3}  {name:35}  peak={:.5}  rms={:.5}",
                        s.peak, s.rms
                    );
                }
                eprintln!();
            }
        }
    }

    /// REGRESSION (rendered audio, not registers): sequencer playback at the
    /// default track volume (127, driver-faithful full) with full note
    /// velocity must be level-identical to the library audition path. The
    /// old defaults (track volume 100 + note velocity 100, both silently
    /// interpreted as TL attenuation) rendered voice 151 34.6 dB below its
    /// audition — the "library instrument is extremely quiet in playback"
    /// bug. Guards the whole FM gain chain by measuring produced samples.
    #[test]
    fn playback_at_full_defaults_matches_audition_level() {
        for rel in [
            "batman-robin/fm/animal-boss-voice-151.json",
            "batman-robin/fm/animal-boss-voice-92.json",
        ] {
            let inst = load_pack_fm(rel);
            let audition = stats(&render_fm_audition(&inst, 60, 1.0), 0.1);
            let playback = stats(&render_fm_note(&inst, 60, 127, 127, 1.0), 0.1);
            let db = 20.0 * (playback.rms / audition.rms).log10();
            assert!(
                db.abs() < 0.5,
                "{rel}: playback at full defaults is {db:+.1} dB vs audition \
                 (audition rms={:.5}, playback rms={:.5})",
                audition.rms, playback.rms
            );
            assert!(
                playback.rms > 0.01,
                "{rel}: playback at full defaults must be audibly loud, rms={:.5}",
                playback.rms
            );
        }
    }

    /// Diagnostic: audition path (raw TLs, no attenuation) vs sequencer
    /// playback at UI-default velocity 100 / track volume 100, voice 151.
    #[test]
    fn audition_vs_default_playback() {
        let inst = load_pack_fm("batman-robin/fm/animal-boss-voice-151.json");
        let audition = stats(&render_fm_audition(&inst, 60, 1.0), 0.1);
        let defaults = stats(&render_fm_note(&inst, 60, 100, 100, 1.0), 0.1);
        let full = stats(&render_fm_note(&inst, 60, 127, 127, 1.0), 0.1);
        let db = |a: f32, b: f32| 20.0 * (a / b).log10();
        eprintln!("voice 151 @ pitch 60:");
        eprintln!("  audition            peak={:.5} rms={:.5}", audition.peak, audition.rms);
        eprintln!("  playback vel/vol127 peak={:.5} rms={:.5} ({:+.1} dB vs audition)",
            full.peak, full.rms, db(full.rms, audition.rms));
        eprintln!("  playback vel/vol100 peak={:.5} rms={:.5} ({:+.1} dB vs audition)",
            defaults.peak, defaults.rms, db(defaults.rms, audition.rms));
    }
}
