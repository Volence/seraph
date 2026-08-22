import * as ipc from "../api/ipc";

/**
 * Coalescing wrapper around `ipc.reloadSequence()` for CONTINUOUS gestures.
 *
 * A knob drag (`Knob` fires `onChange` on every mousemove) or a volume ride
 * (a range input fires one change event per pixel) produces tens of commits a
 * second. Each `reloadSequence` rebuilds the whole snapshot on the backend —
 * every note of every track, with its instrument payload cloned — so firing
 * one per input event queues work faster than it can be done.
 *
 * The coalescer is self-clocked by the IPC round trip rather than by a timer:
 * the first call goes out immediately (so the gesture feels connected), any
 * calls made while that one is in flight collapse into a single trailing
 * reload issued when it returns. That adapts automatically to how expensive
 * the rebuild actually is — a small song reloads on nearly every event, a
 * large one throttles itself — with no magic interval to tune.
 *
 * The trailing reload always sees the final state: callers `await` their
 * mutation before scheduling, so the last scheduled reload is issued after
 * the last mutation has landed.
 *
 * Correctness note: this is only safe because a reload no longer disturbs
 * sounding notes (see `Sequencer::reload_snapshot`). Dropping a reload here
 * only ever costs a few milliseconds of staleness, never a stuck note.
 */

let inFlight: Promise<void> | null = null;
let trailing = false;

function run(): void {
  inFlight = ipc
    .reloadSequence()
    .catch((e) => {
      // A failed reload must not wedge the coalescer: the gesture continues
      // and the next schedule has to be able to fire.
      console.error("reloadSequence failed:", e);
    })
    .then(() => {
      inFlight = null;
      if (trailing) {
        trailing = false;
        run();
      }
    });
}

/**
 * Ask for the running sequence to pick up the latest project state. Safe to
 * call on every input event of a drag.
 */
export function scheduleReloadSequence(): void {
  if (inFlight) {
    trailing = true;
    return;
  }
  run();
}

/**
 * Resolves once no reload is in flight and none is queued. For tests and for
 * callers that need the backend to be current before reading it back.
 */
export async function whenReloadsSettled(): Promise<void> {
  while (inFlight) {
    await inFlight;
  }
}

/** Drop all coalescer state. Test-only: module state leaks between cases. */
export function resetLiveReloadForTests(): void {
  inFlight = null;
  trailing = false;
}
