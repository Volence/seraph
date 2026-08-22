/**
 * Frontend-side transport memory for the Space stop semantics (G37, owner
 * ruling): Space pauses in place; Space again while stopped within
 * STOP_DOUBLE_TAP_MS returns the playhead to where the last playback started
 * (without starting playback); otherwise Space plays from the current
 * position.
 *
 * Owner ruling 2026-08-21: pausing/resuming must never move that return
 * point. The launch point is recorded only when the user establishes a NEW
 * one — the first play of a session, or the first play after an explicit
 * seek (ruler click, Home, the double-Space return-jump itself). A plain
 * pause -> resume does not re-record, so the original launch point survives
 * arbitrarily many pause/resume cycles. The record decision lives HERE, not
 * at the call sites, so App and TransportControls cannot diverge: both call
 * recordPlayStart unconditionally before every transportPlay, and every
 * explicit-seek path calls noteSeek.
 *
 * Module-level (not React state) because both App's Space handler and the
 * TransportControls buttons feed it, and it is read at event time.
 */

/** Space-after-stop within this window returns to the last play start. */
export const STOP_DOUBLE_TAP_MS = 400;

let lastPlayStartTick = 0;
let lastStopAt: number | null = null;
/** When true, the next recordPlayStart establishes a new launch point.
 *  Starts true (first play of a session records); set again by noteSeek. */
let launchPointStale = true;

/**
 * Call just before starting playback, with the transport's current tick.
 * Records a new launch point only when none exists yet or an explicit seek
 * intervened (noteSeek); a plain resume after a pause keeps the original.
 * Always invalidates any pending stop-double-tap window.
 */
export function recordPlayStart(tick: number): void {
  if (launchPointStale) {
    lastPlayStartTick = tick;
    launchPointStale = false;
  }
  // A new play invalidates any pending stop window.
  lastStopAt = null;
}

/**
 * Call from every explicit-seek path (ruler/mouse seek, Home, the
 * double-Space return-jump): the next play records its position as the new
 * launch point.
 */
export function noteSeek(): void {
  launchPointStale = true;
}

/** Call when the user stops playback. */
export function recordStop(now: number = performance.now()): void {
  lastStopAt = now;
}

/**
 * If called within STOP_DOUBLE_TAP_MS of the recorded stop, returns the tick
 * where the last playback started and consumes the window (a further tap
 * plays instead of re-returning); otherwise returns null.
 */
export function consumeStopDoubleTap(now: number = performance.now()): number | null {
  if (lastStopAt !== null && now - lastStopAt <= STOP_DOUBLE_TAP_MS) {
    lastStopAt = null;
    return lastPlayStartTick;
  }
  return null;
}

/**
 * Clear all recorded state. Called at project boundaries (open/create/
 * import) so a stale launch point never leaks across projects; also used by
 * tests.
 */
export function resetTransportMemory(): void {
  lastPlayStartTick = 0;
  lastStopAt = null;
  launchPointStale = true;
}
