//! Rendered-audio tests for LIVE parameter audibility (audit F3 + F13 and the
//! banked `reload_snapshot`/`silence_all` follow-up).
//!
//! The claim under test is "a parameter edit becomes audible in the RUNNING
//! stream", so every assertion here is made on produced samples from the real
//! chain (`Sequencer` -> `AudioEngine` -> Nuked-OPN2 / SN76489 -> filters) via
//! `rendered_rms`. A register-level check would pass on a chain that never
//! reaches the DAC; this repo has already been bitten by exactly that.
//!
//! Method: render a sustained note, drop an `AudioCommand::ReloadSequence`
//! into the stream mid-note (what `reload_sequence` does when a knob or the
//! volume slider commits during playback), and compare the window after the
//! edit against a CONTROL render — the same note played from the start with
//! the edited value. The control render is the derived expectation: a live
//! edit must land the running stream where a fresh play at the new value
//! would have been. Nothing here is transcribed from an observed number.

use crate::audio::command::AudioCommand;
use crate::audio::rendered_rms::{
    db_ratio, fm_instrument_data, load_pack_fm, one_note_snapshot, render_snapshot_with_edits,
    stats_window,
};
use crate::model::instrument::FmInstrument;
use crate::sequencer::{ChannelType, InstrumentData, SequencerEvent, SequencerSnapshot};
use std::sync::Arc;

/// Total render length and the point the edit is injected. The note is held
/// well past `TOTAL_SECS` so nothing else can end it.
const TOTAL_SECS: f64 = 1.0;
const EDIT_AT: f64 = 0.5;
/// Analysis window: starts after the edit plus a settle margin (register
/// writes take effect over a few hundred YM cycles), ends at the render end.
const WINDOW_FROM: f64 = 0.6;
const WINDOW_TO: f64 = 1.0;

/// A voice from the shipped library — a real patch, not a hand-rolled one, so
/// the test exercises the same bytes the DAW plays.
const VOICE: &str = "batman-robin/fm/animal-boss-voice-92.json";

fn fm_snapshot(inst: &FmInstrument, track_volume: u8) -> SequencerSnapshot {
    one_note_snapshot(
        ChannelType::Fm(0),
        fm_instrument_data(inst),
        60,
        127,
        track_volume,
        TOTAL_SECS * 4.0,
    )
}

/// Replace the NoteOn instrument of the single channel in `snap`.
fn with_instrument(mut snap: SequencerSnapshot, instrument: InstrumentData) -> SequencerSnapshot {
    for ev in snap.channels[0].events.iter_mut() {
        if let SequencerEvent::NoteOn { instrument: slot, .. } = ev {
            *slot = instrument.clone();
        }
    }
    snap
}

// --------------------------------------------------------------------------
// The banked follow-up: `reload_snapshot` must not cut sustained notes.
// --------------------------------------------------------------------------

/// A commit that changes NOTHING must be inaudible: reloading the identical
/// snapshot mid-note has to leave the running stream bit-for-bit where the
/// control render is. This is the banked 2026-08-21 follow-up ("any commit
/// mid-loop momentarily cuts sustained notes on other channels") stated as a
/// measurement instead of a suspicion.
#[test]
fn identical_reload_mid_note_is_inaudible() {
    let inst = load_pack_fm(VOICE);
    let control = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &[]);
    let reloaded = render_snapshot_with_edits(
        fm_snapshot(&inst, 127),
        TOTAL_SECS,
        &[(
            EDIT_AT,
            AudioCommand::ReloadSequence { snapshot: fm_snapshot(&inst, 127) },
        )],
    );

    let c = stats_window(&control, WINDOW_FROM, WINDOW_TO);
    let r = stats_window(&reloaded, WINDOW_FROM, WINDOW_TO);
    assert!(
        c.rms > 0.01,
        "control render is not audible (rms={:.5}) — harness broken, not a pass",
        c.rms
    );
    let db = db_ratio(r.rms, c.rms);
    assert!(
        db.abs() < 0.5,
        "a no-op reload changed the sustained note by {db:+.1} dB \
         (control rms={:.5}, after reload rms={:.5}) — the reload is cutting notes",
        c.rms,
        r.rms
    );
}

/// A volume ride is many reloads in a row (`TrackHeader` fires one per input
/// event). Ten back-to-back reloads must still leave the note sounding at the
/// control level — the F13 "machine-guns the audio with silence/reprogram
/// cycles" hazard, measured.
#[test]
fn rapid_repeated_reloads_do_not_machine_gun_the_note() {
    let inst = load_pack_fm(VOICE);
    let control = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &[]);

    // One reload every 10 ms across the drag window — the rate a pointer drag
    // over a range input produces.
    let edits: Vec<(f64, AudioCommand)> = (0..10)
        .map(|i| {
            (
                0.30 + i as f64 * 0.01,
                AudioCommand::ReloadSequence { snapshot: fm_snapshot(&inst, 127) },
            )
        })
        .collect();
    let ridden = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &edits);

    let c = stats_window(&control, WINDOW_FROM, WINDOW_TO);
    let r = stats_window(&ridden, WINDOW_FROM, WINDOW_TO);
    let db = db_ratio(r.rms, c.rms);
    assert!(
        db.abs() < 0.5,
        "10 rapid reloads moved the sustained note {db:+.1} dB off the control \
         (control rms={:.5}, ridden rms={:.5})",
        c.rms,
        r.rms
    );
}

// --------------------------------------------------------------------------
// F13: track volume must be audible in the note that is already sounding.
// --------------------------------------------------------------------------

/// Lowering the track volume mid-note must move the running stream to exactly
/// where a fresh play at that volume would be. Expectation is derived: the
/// control is the same note rendered from the start at the new volume, so the
/// test never encodes an observed dB figure.
#[test]
fn track_volume_change_is_audible_in_the_sustained_note() {
    let inst = load_pack_fm(VOICE);
    const LOUD: u8 = 127;
    // Carrier vol_offset is `127 - volume + 127 - velocity` TL steps, so 100
    // is ~27 TL steps down — a clearly audible cut that still renders well
    // above the noise floor (63 lands at rms ~2e-4, too close to nothing to
    // distinguish a working live update from a silenced note).
    const QUIET: u8 = 100;

    let control_loud = render_snapshot_with_edits(fm_snapshot(&inst, LOUD), TOTAL_SECS, &[]);
    let control_quiet = render_snapshot_with_edits(fm_snapshot(&inst, QUIET), TOTAL_SECS, &[]);
    let edited = render_snapshot_with_edits(
        fm_snapshot(&inst, LOUD),
        TOTAL_SECS,
        &[(
            EDIT_AT,
            AudioCommand::ReloadSequence { snapshot: fm_snapshot(&inst, QUIET) },
        )],
    );

    let loud = stats_window(&control_loud, WINDOW_FROM, WINDOW_TO);
    let quiet = stats_window(&control_quiet, WINDOW_FROM, WINDOW_TO);
    let after = stats_window(&edited, WINDOW_FROM, WINDOW_TO);

    // Sanity: the two control levels must actually differ, else the test
    // proves nothing. Derived from the sequencer's carrier vol_offset
    // (`127 - volume + 127 - velocity`, applied to carrier TL).
    let spread = db_ratio(quiet.rms, loud.rms);
    assert!(
        spread < -3.0,
        "volume {LOUD} vs {QUIET} only differ by {spread:+.1} dB — pick a bigger \
         spread, this test cannot distinguish anything"
    );
    assert!(
        quiet.rms > 0.001,
        "the quiet control (rms={:.5}) is at the noise floor — a silenced note would \
         pass by accident",
        quiet.rms
    );

    let db = db_ratio(after.rms, quiet.rms);
    assert!(
        db.abs() < 1.0,
        "after a mid-note volume change the stream sits {db:+.1} dB from a fresh \
         play at volume {QUIET} (quiet control rms={:.5}, after edit rms={:.5}, \
         loud control rms={:.5})",
        quiet.rms,
        after.rms,
        loud.rms
    );
}

// --------------------------------------------------------------------------
// F3: an FM patch edit must be audible in the note that is already sounding.
// --------------------------------------------------------------------------

/// Raising every carrier's total level mid-note must attenuate the sustained
/// note. TL is a post-envelope attenuation on the YM2612, so the expected
/// level is exactly the level of a fresh play with the same TLs — the control
/// render. (A carrier TL edit is what an `FmEditor` TL slider drag produces.)
#[test]
fn fm_carrier_tl_edit_is_audible_in_the_sustained_note() {
    let inst = load_pack_fm(VOICE);
    const TL_DELTA: u8 = 16;

    // Build the edited patch from the packed bytes so the derivation is the
    // production one: b[20+i] is `total_level | 0x80 if carrier`.
    let base = fm_instrument_data(&inst);
    let edited_data = match &base {
        InstrumentData::FmPatch { bytes, ssg_eg } => {
            let mut b = *bytes;
            for i in 0..4 {
                let carrier_flag = b[20 + i] & 0x80;
                let tl = (b[20 + i] & 0x7F).saturating_add(TL_DELTA).min(127);
                b[20 + i] = tl | carrier_flag;
            }
            InstrumentData::FmPatch { bytes: b, ssg_eg: *ssg_eg }
        }
        _ => unreachable!("library FM entry must pack to an FmPatch"),
    };

    let control_before = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &[]);
    let control_after = render_snapshot_with_edits(
        with_instrument(fm_snapshot(&inst, 127), edited_data.clone()),
        TOTAL_SECS,
        &[],
    );
    let edited = render_snapshot_with_edits(
        fm_snapshot(&inst, 127),
        TOTAL_SECS,
        &[(
            EDIT_AT,
            AudioCommand::ReloadSequence {
                snapshot: with_instrument(fm_snapshot(&inst, 127), edited_data),
            },
        )],
    );

    let before = stats_window(&control_before, WINDOW_FROM, WINDOW_TO);
    let target = stats_window(&control_after, WINDOW_FROM, WINDOW_TO);
    let after = stats_window(&edited, WINDOW_FROM, WINDOW_TO);

    let spread = db_ratio(target.rms, before.rms);
    assert!(
        spread < -3.0,
        "+{TL_DELTA} TL on every carrier only moved the level {spread:+.1} dB — \
         the test cannot distinguish the edit from no edit"
    );
    assert!(
        target.rms > 0.001,
        "the edited-patch control (rms={:.5}) is at the noise floor — a silenced note \
         would pass by accident",
        target.rms
    );
    let db = db_ratio(after.rms, target.rms);
    assert!(
        db.abs() < 1.5,
        "after a mid-note carrier-TL edit the stream sits {db:+.1} dB from a fresh \
         play of the edited patch (edited control rms={:.5}, after edit rms={:.5}, \
         unedited control rms={:.5})",
        target.rms,
        after.rms,
        before.rms
    );
}

/// A patch edit that is NOT a level change (operator MUL — the FmEditor "MUL"
/// knob) must also reach the sounding note: the running stream has to stop
/// matching the unedited control. Envelope state carries across a live patch
/// write, so this asserts audible divergence rather than an exact target.
#[test]
fn fm_timbre_edit_reaches_the_sustained_note() {
    let inst = load_pack_fm(VOICE);

    let base = fm_instrument_data(&inst);
    let edited_data = match &base {
        InstrumentData::FmPatch { bytes, ssg_eg } => {
            let mut b = *bytes;
            // b[i] = (detune << 4) | multiple. Force every operator to MUL=8,
            // keeping detune — a drastic, purely timbral change.
            for i in 0..4 {
                b[i] = (b[i] & 0xF0) | 8;
            }
            InstrumentData::FmPatch { bytes: b, ssg_eg: *ssg_eg }
        }
        _ => unreachable!("library FM entry must pack to an FmPatch"),
    };

    let control = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &[]);
    let edited = render_snapshot_with_edits(
        fm_snapshot(&inst, 127),
        TOTAL_SECS,
        &[(
            EDIT_AT,
            AudioCommand::ReloadSequence {
                snapshot: with_instrument(fm_snapshot(&inst, 127), edited_data),
            },
        )],
    );

    let c = stats_window(&control, WINDOW_FROM, WINDOW_TO);
    let e = stats_window(&edited, WINDOW_FROM, WINDOW_TO);
    assert!(
        e.rms > 0.001,
        "the sustained note died after the timbre edit (rms={:.5}) — a patch edit \
         must not silence the running note",
        e.rms
    );
    // Waveform divergence, not just level: compare sample-by-sample against
    // the control over the same window. Identical streams give 0.
    let lo = (WINDOW_FROM * crate::audio::rendered_rms::SAMPLE_RATE as f64) as usize;
    let hi = (WINDOW_TO * crate::audio::rendered_rms::SAMPLE_RATE as f64) as usize;
    let diff_rms = {
        let n = hi - lo;
        let sum: f64 = (lo..hi)
            .map(|i| {
                let d = (edited[i] - control[i]) as f64;
                d * d
            })
            .sum();
        (sum / n as f64).sqrt() as f32
    };
    let db = db_ratio(diff_rms, c.rms);
    assert!(
        db > -20.0,
        "the running stream barely moved after a MUL edit: difference is {db:+.1} dB \
         below the control (control rms={:.5}, diff rms={diff_rms:.5}) — the edit did \
         not reach the sounding note",
        c.rms
    );
}

// --------------------------------------------------------------------------
// F3/F13 on PSG.
// --------------------------------------------------------------------------

fn psg_snapshot(envelope: Vec<u8>, track_volume: u8) -> SequencerSnapshot {
    one_note_snapshot(
        ChannelType::Psg(0),
        InstrumentData::PsgEnvelope {
            period: 0,
            envelope: Arc::new(envelope),
            // Loop so the envelope holds a steady level for the whole render.
            loop_point: Some(0),
            silence_on_end: false,
        },
        60,
        127,
        track_volume,
        TOTAL_SECS * 4.0,
    )
}

/// PSG track volume must reach the sounding note too. Steady (looped) envelope
/// so the only thing moving is the volume attenuation the sequencer derives
/// (`(127-volume)*15/127 + (127-velocity)*15/127`, clamped to 15).
#[test]
fn psg_volume_change_is_audible_in_the_sustained_note() {
    const ENV: [u8; 2] = [15, 15];
    const LOUD: u8 = 127;
    const QUIET: u8 = 63;

    let control_loud =
        render_snapshot_with_edits(psg_snapshot(ENV.to_vec(), LOUD), TOTAL_SECS, &[]);
    let control_quiet =
        render_snapshot_with_edits(psg_snapshot(ENV.to_vec(), QUIET), TOTAL_SECS, &[]);
    let edited = render_snapshot_with_edits(
        psg_snapshot(ENV.to_vec(), LOUD),
        TOTAL_SECS,
        &[(
            EDIT_AT,
            AudioCommand::ReloadSequence { snapshot: psg_snapshot(ENV.to_vec(), QUIET) },
        )],
    );

    let loud = stats_window(&control_loud, WINDOW_FROM, WINDOW_TO);
    let quiet = stats_window(&control_quiet, WINDOW_FROM, WINDOW_TO);
    let after = stats_window(&edited, WINDOW_FROM, WINDOW_TO);

    let spread = db_ratio(quiet.rms, loud.rms);
    assert!(
        spread < -3.0,
        "PSG volume {LOUD} vs {QUIET} only differ by {spread:+.1} dB — test cannot \
         distinguish anything"
    );
    let db = db_ratio(after.rms, quiet.rms);
    assert!(
        db.abs() < 1.0,
        "after a mid-note PSG volume change the stream sits {db:+.1} dB from a fresh \
         play at volume {QUIET} (quiet control rms={:.5}, after edit rms={:.5}, loud \
         control rms={:.5})",
        quiet.rms,
        after.rms,
        loud.rms
    );
}

/// A PSG envelope edit (the `PsgEditor` step-graph) must reach the sounding
/// note: the running level has to move to the edited envelope's level rather
/// than stay on the old one or go silent.
#[test]
fn psg_envelope_edit_is_audible_in_the_sustained_note() {
    const LOUD_ENV: [u8; 2] = [15, 15];
    const QUIET_ENV: [u8; 2] = [8, 8];

    let control_loud =
        render_snapshot_with_edits(psg_snapshot(LOUD_ENV.to_vec(), 127), TOTAL_SECS, &[]);
    let control_quiet =
        render_snapshot_with_edits(psg_snapshot(QUIET_ENV.to_vec(), 127), TOTAL_SECS, &[]);
    let edited = render_snapshot_with_edits(
        psg_snapshot(LOUD_ENV.to_vec(), 127),
        TOTAL_SECS,
        &[(
            EDIT_AT,
            AudioCommand::ReloadSequence { snapshot: psg_snapshot(QUIET_ENV.to_vec(), 127) },
        )],
    );

    let loud = stats_window(&control_loud, WINDOW_FROM, WINDOW_TO);
    let quiet = stats_window(&control_quiet, WINDOW_FROM, WINDOW_TO);
    let after = stats_window(&edited, WINDOW_FROM, WINDOW_TO);

    let spread = db_ratio(quiet.rms, loud.rms);
    assert!(
        spread < -3.0,
        "envelope {LOUD_ENV:?} vs {QUIET_ENV:?} only differ by {spread:+.1} dB"
    );
    let db = db_ratio(after.rms, quiet.rms);
    assert!(
        db.abs() < 1.0,
        "after a mid-note PSG envelope edit the stream sits {db:+.1} dB from a fresh \
         play of the edited envelope (edited control rms={:.5}, after edit rms={:.5}, \
         unedited control rms={:.5})",
        quiet.rms,
        after.rms,
        loud.rms
    );
}

// --------------------------------------------------------------------------
// The safety net `silence_all` used to provide. Dropping the global silence
// must NOT leave notes ringing when the edit removed them.
// --------------------------------------------------------------------------

/// The RMS of the whole render's tail, used by the "must go silent" tests.
/// A key-off is a release, not an instant cut, so the window starts well
/// after the edit.
const RELEASE_FROM: f64 = 0.8;

/// Deleting the sounding note (the reloaded snapshot has no events at all)
/// must key the channel off. Without a global silence this is the orphan
/// path: no matching note in the new snapshot -> targeted key-off.
#[test]
fn deleting_the_sounding_note_still_silences_it() {
    let inst = load_pack_fm(VOICE);
    let mut emptied = fm_snapshot(&inst, 127);
    emptied.channels[0].events.clear();

    let control = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &[]);
    let deleted = render_snapshot_with_edits(
        fm_snapshot(&inst, 127),
        TOTAL_SECS,
        &[(EDIT_AT, AudioCommand::ReloadSequence { snapshot: emptied })],
    );

    let c = stats_window(&control, RELEASE_FROM, WINDOW_TO);
    let d = stats_window(&deleted, RELEASE_FROM, WINDOW_TO);
    let db = db_ratio(d.rms, c.rms);
    assert!(
        db < -40.0,
        "after deleting the sounding note the channel is still ringing at {db:+.1} dB \
         relative to the note that was left in place (control rms={:.5}, after delete \
         rms={:.5})",
        c.rms,
        d.rms
    );
}

/// Muting the track removes its channel from the snapshot entirely (the
/// snapshot only carries non-muted tracks). The sounding note must be keyed
/// off through the OLD channel type — nothing in the new snapshot names it.
#[test]
fn muting_the_track_still_silences_the_sounding_note() {
    let inst = load_pack_fm(VOICE);
    let mut muted = fm_snapshot(&inst, 127);
    muted.channels.clear();

    let control = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &[]);
    let after = render_snapshot_with_edits(
        fm_snapshot(&inst, 127),
        TOTAL_SECS,
        &[(EDIT_AT, AudioCommand::ReloadSequence { snapshot: muted })],
    );

    let c = stats_window(&control, RELEASE_FROM, WINDOW_TO);
    let m = stats_window(&after, RELEASE_FROM, WINDOW_TO);
    let db = db_ratio(m.rms, c.rms);
    assert!(
        db < -40.0,
        "after muting the track the note is still ringing at {db:+.1} dB relative to \
         the unmuted control (control rms={:.5}, after mute rms={:.5})",
        c.rms,
        m.rms
    );
}

/// Same for PSG: a removed note must not be left holding its envelope, which
/// is what the blanket per-channel attenuation writes in `silence_all` used
/// to guarantee.
#[test]
fn deleting_a_sounding_psg_note_still_silences_it() {
    let mut emptied = psg_snapshot(vec![15, 15], 127);
    emptied.channels[0].events.clear();

    let control = render_snapshot_with_edits(psg_snapshot(vec![15, 15], 127), TOTAL_SECS, &[]);
    let deleted = render_snapshot_with_edits(
        psg_snapshot(vec![15, 15], 127),
        TOTAL_SECS,
        &[(EDIT_AT, AudioCommand::ReloadSequence { snapshot: emptied })],
    );

    let c = stats_window(&control, RELEASE_FROM, WINDOW_TO);
    let d = stats_window(&deleted, RELEASE_FROM, WINDOW_TO);
    let db = db_ratio(d.rms, c.rms);
    assert!(
        db < -40.0,
        "after deleting the sounding PSG note the channel is still ringing at \
         {db:+.1} dB (control rms={:.5}, after delete rms={:.5})",
        c.rms,
        d.rms
    );
}

/// Transposing the sounding note (its pitch no longer matches) is not a
/// survivor: the old pitch must stop rather than keep ringing at the wrong
/// frequency until the next event.
#[test]
fn retuning_the_sounding_note_stops_the_old_pitch() {
    let inst = load_pack_fm(VOICE);
    let mut retuned = fm_snapshot(&inst, 127);
    for ev in retuned.channels[0].events.iter_mut() {
        match ev {
            SequencerEvent::NoteOn { pitch, .. } => *pitch = 67,
            SequencerEvent::NoteOff { pitch, .. } => *pitch = 67,
        }
    }

    let control = render_snapshot_with_edits(fm_snapshot(&inst, 127), TOTAL_SECS, &[]);
    let after = render_snapshot_with_edits(
        fm_snapshot(&inst, 127),
        TOTAL_SECS,
        &[(EDIT_AT, AudioCommand::ReloadSequence { snapshot: retuned })],
    );

    let c = stats_window(&control, RELEASE_FROM, WINDOW_TO);
    let a = stats_window(&after, RELEASE_FROM, WINDOW_TO);
    let db = db_ratio(a.rms, c.rms);
    assert!(
        db < -40.0,
        "after retuning the sounding note the old pitch is still ringing at {db:+.1} dB \
         (control rms={:.5}, after retune rms={:.5})",
        c.rms,
        a.rms
    );
}

// --------------------------------------------------------------------------
// Cross-channel: the banked follow-up's actual complaint.
// --------------------------------------------------------------------------

/// Editing ONE channel must not disturb another channel's sustained note.
/// Two FM voices sound; the reload changes only channel 1's track volume.
/// Channel 0's contribution is isolated by rendering it alone as the control
/// and subtracting nothing — instead the whole mix is compared against a mix
/// whose channel-1 volume was low from the start.
#[test]
fn editing_one_channel_does_not_cut_another_channels_note() {
    let inst = load_pack_fm(VOICE);

    let two_channel = |ch1_volume: u8| -> SequencerSnapshot {
        let mut snap = fm_snapshot(&inst, 127);
        let mut second = fm_snapshot(&inst, ch1_volume).channels.pop().unwrap();
        second.channel_type = ChannelType::Fm(1);
        // Different pitch so the two channels are distinguishable.
        for ev in second.events.iter_mut() {
            match ev {
                SequencerEvent::NoteOn { pitch, .. } => *pitch = 67,
                SequencerEvent::NoteOff { pitch, .. } => *pitch = 67,
            }
        }
        snap.channels.push(second);
        snap
    };

    let control = render_snapshot_with_edits(two_channel(63), TOTAL_SECS, &[]);
    let edited = render_snapshot_with_edits(
        two_channel(127),
        TOTAL_SECS,
        &[(EDIT_AT, AudioCommand::ReloadSequence { snapshot: two_channel(63) })],
    );

    let c = stats_window(&control, WINDOW_FROM, WINDOW_TO);
    let e = stats_window(&edited, WINDOW_FROM, WINDOW_TO);
    let db = db_ratio(e.rms, c.rms);
    assert!(
        db.abs() < 1.0,
        "a reload that only lowers channel 1 left the two-channel mix {db:+.1} dB off \
         a fresh play of the same state (control rms={:.5}, after edit rms={:.5}) — \
         channel 0's sustained note was disturbed",
        c.rms,
        e.rms
    );
}
