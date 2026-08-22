/**
 * Frontend-side transport memory for the Space stop semantics (G37, owner
 * ruling): Space pauses in place; Space again while stopped within
 * STOP_DOUBLE_TAP_MS returns the playhead to where the last playback started
 * (without starting playback); otherwise Space plays from the current
 * position.
 *
 * "Where playback started" is recorded at each transportPlay call site.
 * Module-level (not React state) because both App's Space handler and the
 * TransportControls buttons feed it, and it is read at event time.
 */

/** Space-after-stop within this window returns to the last play start. */
export const STOP_DOUBLE_TAP_MS = 400;

let lastPlayStartTick = 0;
let lastStopAt: number | null = null;

/** Call just before starting playback, with the transport's current tick. */
export function recordPlayStart(tick: number): void {
  lastPlayStartTick = tick;
  // A new play invalidates any pending stop window.
  lastStopAt = null;
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

/** Test helper: clear all recorded state. */
export function resetTransportMemory(): void {
  lastPlayStartTick = 0;
  lastStopAt = null;
}
