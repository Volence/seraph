import { describe, it, expect, beforeEach } from "vitest";
import {
  STOP_DOUBLE_TAP_MS,
  recordPlayStart,
  recordStop,
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
