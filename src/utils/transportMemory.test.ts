import { describe, it, expect, beforeEach } from "vitest";
import {
  STOP_DOUBLE_TAP_MS,
  recordPlayStart,
  recordStop,
  noteSeek,
  consumeStopDoubleTap,
  resetTransportMemory,
} from "./transportMemory";

// Owner ruling G37: Space while stopped, within STOP_DOUBLE_TAP_MS of the
// stop, returns the playhead to where the last playback started (without
// starting playback); otherwise Space plays from the current position.

beforeEach(() => {
  resetTransportMemory();
});

describe("transport stop double-tap memory", () => {
  it("names the window constant at ~400ms", () => {
    expect(STOP_DOUBLE_TAP_MS).toBe(400);
  });

  it("within the window after a stop, yields the last play-start tick", () => {
    recordPlayStart(960);
    recordStop(1000);
    expect(consumeStopDoubleTap(1000 + STOP_DOUBLE_TAP_MS)).toBe(960);
  });

  it("outside the window, yields null (play from current position)", () => {
    recordPlayStart(960);
    recordStop(1000);
    expect(consumeStopDoubleTap(1000 + STOP_DOUBLE_TAP_MS + 1)).toBeNull();
  });

  it("consumes the window: a second tap no longer returns", () => {
    recordPlayStart(960);
    recordStop(1000);
    expect(consumeStopDoubleTap(1100)).toBe(960);
    expect(consumeStopDoubleTap(1150)).toBeNull();
  });

  it("without any stop recorded, yields null", () => {
    recordPlayStart(960);
    expect(consumeStopDoubleTap(1000)).toBeNull();
  });

  it("a new play clears a pending stop window", () => {
    recordPlayStart(0);
    recordStop(1000);
    recordPlayStart(500); // played again right after the stop
    expect(consumeStopDoubleTap(1100)).toBeNull();
  });
});

// Owner ruling 2026-08-21: pausing/resuming must never move the double-Space
// return point. The launch point updates only when the user establishes a NEW
// one: the first play of a session, or the first play after an explicit seek
// (ruler click, Home, or the double-Space return-jump itself).
describe("launch-point memory across pause/resume (owner ruling 2026-08-21)", () => {
  it("first play of a session records the launch point", () => {
    recordPlayStart(960);
    recordStop(1000);
    expect(consumeStopDoubleTap(1100)).toBe(960);
  });

  it("pause -> resume does not re-record: the original launch point survives", () => {
    recordPlayStart(960); // launch
    recordStop(1000); // pause mid-song at, say, tick 2400
    recordPlayStart(2400); // resume in place (no seek in between)
    recordStop(5000); // pause again, later
    expect(consumeStopDoubleTap(5100)).toBe(960);
  });

  it("the original launch point survives arbitrarily many pause/resume cycles", () => {
    recordPlayStart(960);
    for (let i = 1; i <= 5; i++) {
      recordStop(i * 10_000);
      recordPlayStart(960 + i * 1000); // each resume is further along
    }
    recordStop(100_000);
    expect(consumeStopDoubleTap(100_100)).toBe(960);
  });

  it("an explicit seek means the next play records the new launch point", () => {
    recordPlayStart(960);
    recordStop(1000); // pause
    noteSeek(); // user clicks the ruler elsewhere
    recordPlayStart(4800); // play from the clicked position
    recordStop(9000);
    expect(consumeStopDoubleTap(9100)).toBe(4800);
  });

  it("a seek re-arms recording even across a pause/resume in between", () => {
    recordPlayStart(960);
    noteSeek(); // seek while playing
    recordStop(2000); // pause
    recordPlayStart(4800); // resume: seek intervened, so this records
    recordStop(9000);
    expect(consumeStopDoubleTap(9100)).toBe(4800);
  });

  it("pause -> resume -> pause -> double-Space returns the ORIGINAL launch point", () => {
    recordPlayStart(960); // launch
    recordStop(1000); // pause
    recordPlayStart(2400); // resume (outside the 400ms window semantics irrelevant here)
    recordStop(5000); // pause again
    // Double-Space within the window: return to the ORIGINAL launch point.
    expect(consumeStopDoubleTap(5000 + STOP_DOUBLE_TAP_MS)).toBe(960);
  });

  it("the double-Space return-jump itself counts as a seek: the next play re-records", () => {
    recordPlayStart(960);
    recordStop(1000);
    expect(consumeStopDoubleTap(1100)).toBe(960); // jump back to 960
    noteSeek(); // the return-jump routes through the seek path
    recordPlayStart(960); // play from the returned position
    recordStop(3000);
    expect(consumeStopDoubleTap(3100)).toBe(960);
  });

  it("resetTransportMemory re-arms: first play after a project switch records", () => {
    recordPlayStart(960);
    resetTransportMemory(); // project boundary
    recordPlayStart(120); // first play in the new project (no seek noted)
    recordStop(1000);
    expect(consumeStopDoubleTap(1100)).toBe(120);
  });
});
