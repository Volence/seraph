//! Rendered-audio tests for LAST-NOTE-PRIORITY on a shared hardware channel.
//!
//! The Memra driver has no note-off event and no note identity: channel state
//! is one `sc_note` byte and one `SCF_KEYED` bit, and a note-on onto a
//! sounding channel force-keys-off first (`Fm_NoteOnFreqExact.do_keyon`,
//! aeon `1ee8f8e6`, `sound_fm.emp:1092-1099`). Overlapping notes on one
//! channel therefore resolve as last-note-priority: the effective duration of
//! a note is `min(authored, next onset on that channel)`.
//!
//! `build_snapshot` merges several author-side tracks onto one channel, so
//! the note that terminates another may come from a different track. These
//! tests go through the REAL path — `ProjectManager::build_snapshot` ->
//! `Sequencer` -> `AudioEngine` -> Nuked-OPN2 -> filters — and assert on
//! produced samples. Every expectation is a CONTROL render of a song that
//! states the hypothesis directly; nothing here is a transcribed number.

use crate::audio::rendered_rms::{
    db_ratio, load_pack_fm, render_snapshot_with_edits, stats_window, TICKS_PER_SEC,
};
use crate::model::song::ChannelAssignment;
use crate::project::ProjectManager;
use crate::sequencer::SequencerSnapshot;
use std::env;
use uuid::Uuid;

/// A SUSTAINING voice from the shipped library — real patch bytes, so the
/// render is the one the DAW plays. Used where the question is "is the note
/// still sounding?", which needs a level that does not decay away on its own.
const SUSTAIN_VOICE: &str = "batman-robin/fm/animal-boss-voice-92.json";
/// A strongly DECAYING voice, used for the re-attack test: a tie and an
/// envelope restart are only distinguishable on a voice whose level has
/// dropped by the overlap point. The test verifies that separation itself
/// before asserting anything about the render under test.
const DECAY_VOICE: &str = "sonic2/fm/ooz-voice-03.json";
/// One pitch throughout: the tie-vs-re-attack comparison must differ ONLY by
/// the envelope restart, never by frequency.
const PITCH: u8 = 60;

fn secs(ticks: u64) -> f64 {
    ticks as f64 / TICKS_PER_SEC
}

/// Build a real project whose FM1 channel carries `voiced` + `unvoiced` — one
/// note per AUTHOR-SIDE TRACK, every track assigned to `Fm(0)`. Tracks for
/// `voiced` notes carry the library voice; tracks for `unvoiced` notes carry
/// NO instrument, which is how a fresh project's seeded lanes start and what
/// an author has while drawing notes before picking a patch. Returns what the
/// playback path is actually handed.
///
/// One track per note is deliberate: the merge across tracks is the case the
/// suppression has to get right, and a per-track view cannot see it.
fn snapshot_mixed(
    tag: &str,
    voice: &str,
    voiced: &[(u64, u64)],
    unvoiced: &[(u64, u64)],
) -> SequencerSnapshot {
    let path = env::temp_dir().join(format!("seraph_test_overlap_{tag}_{}", Uuid::new_v4()));
    let mut mgr = ProjectManager::new(crate::driver::default_registry());
    mgr.create(&path, "Overlap", "flamedriver", 120.0, (4, 4))
        .expect("create project");
    let inst_id = mgr.add_fm_instrument(load_pack_fm(voice));

    let span = voiced
        .iter()
        .chain(unvoiced)
        .map(|(t, d)| t + d)
        .max()
        .unwrap_or(0)
        + 480;
    let all = voiced
        .iter()
        .map(|n| (*n, Some(inst_id)))
        .chain(unvoiced.iter().map(|n| (*n, None)));
    for (i, ((tick, duration_ticks), inst)) in all.enumerate() {
        let track_id = mgr.add_track(format!("lane{i}"), ChannelAssignment::Fm(0), inst);
        let region_id = mgr.add_region(track_id, 0, span).expect("add region");
        mgr.add_note(track_id, region_id, tick, PITCH, 127, duration_ticks, None)
            .expect("add note");
    }

    let snapshot = mgr.build_snapshot();
    let _ = std::fs::remove_dir_all(&path);
    snapshot
}

fn snapshot_for(tag: &str, voice: &str, notes: &[(u64, u64)]) -> SequencerSnapshot {
    snapshot_mixed(tag, voice, notes, &[])
}

fn render(tag: &str, voice: &str, notes: &[(u64, u64)], total_secs: f64) -> Vec<f32> {
    render_snapshot_with_edits(snapshot_for(tag, voice, notes), total_secs, &[])
}

// The scenario, in ticks (960 ticks/sec at the project's 120 BPM / 480 tpb):
// A runs [0, 480), B runs [240, 720). Pre-fix the merged event list carried a
// stale `NoteOff{tick: 480}` from A, which keyed OFF the sounding B.
const A_START: u64 = 0;
const A_DUR: u64 = 480;
const B_START: u64 = 240;
const B_DUR: u64 = 480;
const B_END: u64 = B_START + B_DUR;
const OVERLAP: &[(u64, u64)] = &[(A_START, A_DUR), (B_START, B_DUR)];

// --------------------------------------------------------------------------
// 1. The fix: B sounds for its full authored duration.
// --------------------------------------------------------------------------

/// The window that used to be truncated: from A's authored end to B's
/// authored end. B alone is the control — the same song with A deleted, which
/// is exactly the state the hardware occupies from B's onset onward.
#[test]
fn an_overlapped_note_sounds_for_its_full_authored_duration() {
    // Settle margin off A's end so the measurement is about B sustaining, not
    // about the sample on which the stale key-off landed.
    let from = secs(A_DUR) + 0.05;
    let to = secs(B_END);
    let total = to + 1.0;

    let both = render("both", SUSTAIN_VOICE, OVERLAP, total);
    let b_alone = render("b_alone", SUSTAIN_VOICE, &[(B_START, B_DUR)], total);

    // The window has to be one where B is genuinely audible, or "not silent"
    // proves nothing. Derive that from the control's own late silence rather
    // than from a transcribed level.
    let control = stats_window(&b_alone, from, to);
    let control_silence = stats_window(&b_alone, to + 0.5, total);
    let headroom = db_ratio(control.rms, control_silence.rms);
    assert!(
        headroom > 20.0,
        "the control's own window is only {headroom:+.1} dB above its post-note silence \
         (window rms={:.5}, silence rms={:.5}) — the measurement cannot distinguish \
         sounding from truncated",
        control.rms,
        control_silence.rms
    );

    let merged = stats_window(&both, from, to);
    let db = db_ratio(merged.rms, control.rms);
    assert!(
        db.abs() < 1.0,
        "B is {db:+.1} dB off a control render of B alone over [{from:.3}s, {to:.3}s) \
         (control rms={:.5}, overlapped rms={:.5}) — the overlapped note did not sound \
         for its full authored duration",
        control.rms,
        merged.rms
    );
}

// --------------------------------------------------------------------------
// 2. The re-attack: B keys on again, it does not tie into A.
// --------------------------------------------------------------------------

/// A note-on onto a sounding channel re-attacks (the driver keys off first,
/// then keys on). The opposite divergence — treating the overlap as a tie and
/// letting A's envelope run on — is what this catches.
///
/// Both hypotheses are rendered as songs at the SAME pitch, so they differ
/// only by the envelope restart:
///   * tie      — one note spanning [A_START, B_END): no second key-on.
///   * re-attack — B alone starting at B_START: a fresh key-on there.
/// The test first proves the window separates the two, then places the real
/// overlapped render on the re-attack side.
#[test]
fn the_second_note_re_attacks_rather_than_tying() {
    // Window right after B's onset: the attack transient itself.
    let from = secs(B_START);
    let to = secs(B_START) + 0.06;
    let total = secs(B_END) + 1.0;

    let both = render("attack_both", DECAY_VOICE, OVERLAP, total);
    let tie = render("attack_tie", DECAY_VOICE, &[(A_START, B_END - A_START)], total);
    let fresh = render("attack_fresh", DECAY_VOICE, &[(B_START, B_DUR)], total);

    let t = stats_window(&tie, from, to);
    let f = stats_window(&fresh, from, to);
    let separation = db_ratio(f.rms, t.rms);
    assert!(
        separation.abs() > 6.0,
        "a fresh key-on and a tie are only {separation:+.1} dB apart over \
         [{from:.3}s, {to:.3}s) (tie rms={:.5}, fresh rms={:.5}) — this window cannot \
         tell a re-attack from a tie, so the test proves nothing",
        t.rms,
        f.rms
    );

    let b = stats_window(&both, from, to);
    let vs_fresh = db_ratio(b.rms, f.rms);
    let vs_tie = db_ratio(b.rms, t.rms);
    assert!(
        vs_fresh.abs() < 1.0,
        "at the overlap point the render is {vs_fresh:+.1} dB off a fresh key-on and \
         {vs_tie:+.1} dB off a tie (fresh rms={:.5}, tie rms={:.5}, actual rms={:.5}) — \
         the second note did not re-attack",
        f.rms,
        t.rms,
        b.rms
    );
    assert!(
        vs_tie.abs() > 6.0,
        "at the overlap point the render sits {vs_tie:+.1} dB from a tie \
         (tie rms={:.5}, actual rms={:.5}) — the envelope was not restarted",
        t.rms,
        b.rms
    );
}

// --------------------------------------------------------------------------
// 3. Non-regression: a note-off that no successor replaces still happens.
// --------------------------------------------------------------------------

/// Suppression applies only where a successor note-on takes over. Two notes
/// with a GAP between them must still key off at the first note's end and go
/// quiet until the second — otherwise the fix has become "never key off".
#[test]
fn a_non_overlapping_pair_still_keys_off_between_the_notes() {
    const GAP_START: u64 = A_START + A_DUR;
    const SECOND_START: u64 = 960;
    let notes = &[(A_START, A_DUR), (SECOND_START, B_DUR)];
    let total = secs(SECOND_START + B_DUR) + 0.5;

    let rendered = render("gap", SUSTAIN_VOICE, notes, total);

    // Body of the first note (past its attack), and the tail of the gap
    // (past the release), both from THIS render.
    let body = stats_window(&rendered, secs(A_START) + 0.05, secs(A_START + A_DUR));
    let gap = stats_window(&rendered, secs(GAP_START) + 0.2, secs(SECOND_START));
    let db = db_ratio(gap.rms, body.rms);
    assert!(
        db < -40.0,
        "between two non-overlapping notes the channel is still ringing at {db:+.1} dB \
         relative to the first note's own body (body rms={:.5}, gap rms={:.5}) — the \
         note-off was suppressed where no successor replaces it",
        body.rms,
        gap.rms
    );

    // And the second note must still arrive: a gap that is silent because
    // nothing plays at all would satisfy the assertion above.
    let second = stats_window(&rendered, secs(SECOND_START) + 0.05, secs(SECOND_START + B_DUR));
    let second_db = db_ratio(second.rms, body.rms);
    assert!(
        second_db.abs() < 3.0,
        "the second note renders {second_db:+.1} dB from the first (first rms={:.5}, \
         second rms={:.5}) — the two-note render is not what the test assumes",
        body.rms,
        second.rms
    );
}

// --------------------------------------------------------------------------
// 4. A note with no resolvable instrument contributes NO events.
// --------------------------------------------------------------------------

/// A note whose instrument does not resolve keys nothing on. It must not key
/// anything OFF either: a bare key-off is pitch-blind, so it silences whatever
/// the channel happens to be sounding — the very divergence last-note-priority
/// exists to remove, reached through a different door.
///
/// This is ordinary DAW state, not a corner case: `create` seeds one
/// instrument-less track per channel, so any notes drawn before a patch is
/// picked land on exactly such a track.
///
/// The unvoiced note is placed so its authored end falls STRICTLY INSIDE the
/// sounding note — the only arrangement in which its stray key-off is
/// reachable. (If it ended at or after the sounding note's end it would be
/// last in the merged order and its off would be indistinguishable from the
/// real one; if a voiced successor followed, suppression would have removed
/// it already.)
#[test]
fn a_note_with_no_instrument_does_not_key_off_a_sounding_note() {
    const VOICED_DUR: u64 = 960;
    const UNVOICED: (u64, u64) = (240, 240); // ends at 480, inside [0, 960)
    let from = secs(UNVOICED.0 + UNVOICED.1) + 0.05;
    let to = secs(VOICED_DUR);
    let total = to + 1.0;

    let voiced = [(0u64, VOICED_DUR)];
    let mixed = render_snapshot_with_edits(
        snapshot_mixed("unvoiced_mixed", SUSTAIN_VOICE, &voiced, &[UNVOICED]),
        total,
        &[],
    );
    // Control: the same song WITHOUT the unvoiced note. An unvoiced note
    // produces no driver events at all, so the two must render identically.
    let alone = render_snapshot_with_edits(
        snapshot_mixed("unvoiced_alone", SUSTAIN_VOICE, &voiced, &[]),
        total,
        &[],
    );

    let control = stats_window(&alone, from, to);
    let control_silence = stats_window(&alone, to + 0.5, total);
    let headroom = db_ratio(control.rms, control_silence.rms);
    assert!(
        headroom > 20.0,
        "the control's own window is only {headroom:+.1} dB above its post-note silence \
         (window rms={:.5}, silence rms={:.5}) — the measurement cannot distinguish \
         sounding from cut short",
        control.rms,
        control_silence.rms
    );

    let m = stats_window(&mixed, from, to);
    let db = db_ratio(m.rms, control.rms);
    assert!(
        db.abs() < 1.0,
        "with an instrument-less note overlapping it, the sounding note is {db:+.1} dB \
         off a control render without that note over [{from:.3}s, {to:.3}s) (control \
         rms={:.5}, mixed rms={:.5}) — a note that keys nothing on still keyed \
         something off",
        control.rms,
        m.rms
    );
}
