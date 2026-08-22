import { describe, it, expect } from "vitest";
import { ticksPerBar, type GridMeta } from "./grid";
import {
  loopHitZone,
  resizePreviewLoop,
  movePreviewLoop,
  LOOP_HANDLE_PX,
} from "./loopHandles";

// Non-4/4 so hardcoded 4-beats-per-bar snapping fails: bar = 1440.
const meta: GridMeta = { ticksPerBeat: 480, timeSignature: [3, 4] };
const BAR = ticksPerBar(meta);

describe("loopHitZone", () => {
  it("hits the start handle within the tolerance band", () => {
    expect(loopHitZone(100 - LOOP_HANDLE_PX, 100, 200)).toBe("start");
    expect(loopHitZone(100 + LOOP_HANDLE_PX, 100, 200)).toBe("start");
  });

  it("hits the end handle within the tolerance band", () => {
    expect(loopHitZone(200 - LOOP_HANDLE_PX, 100, 200)).toBe("end");
    expect(loopHitZone(200 + LOOP_HANDLE_PX, 100, 200)).toBe("end");
  });

  it("hits the body between the handles", () => {
    expect(loopHitZone(150, 100, 200)).toBe("body");
  });

  it("misses outside the band plus tolerance", () => {
    expect(loopHitZone(100 - LOOP_HANDLE_PX - 1, 100, 200)).toBe(null);
    expect(loopHitZone(200 + LOOP_HANDLE_PX + 1, 100, 200)).toBe(null);
  });

  it("prefers the nearest edge when the band is narrower than the handles", () => {
    expect(loopHitZone(101, 100, 104)).toBe("start");
    expect(loopHitZone(103, 100, 104)).toBe("end");
  });
});

describe("resizePreviewLoop", () => {
  const loop = { start: BAR, end: BAR * 3 };

  it("rounds the dragged start edge to the snap unit", () => {
    // BAR + 500 rounds down to BAR; BAR + 800 rounds up to 2*BAR.
    expect(resizePreviewLoop(loop, "start", BAR + 500, meta, "bar")).toEqual({ start: BAR, end: BAR * 3 });
    expect(resizePreviewLoop(loop, "start", BAR + 800, meta, "bar")).toEqual({ start: BAR * 2, end: BAR * 3 });
  });

  it("rounds the dragged end edge to the snap unit", () => {
    expect(resizePreviewLoop(loop, "end", BAR * 4 - 500, meta, "bar")).toEqual({ start: BAR, end: BAR * 4 });
  });

  it("keeps at least one snap unit when edges would cross", () => {
    expect(resizePreviewLoop(loop, "start", BAR * 5, meta, "bar")).toEqual({ start: BAR * 2, end: BAR * 3 });
    expect(resizePreviewLoop(loop, "end", 0, meta, "bar")).toEqual({ start: BAR, end: BAR * 2 });
  });

  it("clamps the start edge at zero", () => {
    expect(resizePreviewLoop(loop, "start", -5000, meta, "bar").start).toBe(0);
  });

  it("resizes at beat granularity in beat snap", () => {
    const beat = meta.ticksPerBeat;
    expect(resizePreviewLoop(loop, "end", BAR * 3 + beat - 10, meta, "beat")).toEqual({ start: BAR, end: BAR * 3 + beat });
  });
});

describe("movePreviewLoop", () => {
  const loop = { start: BAR, end: BAR * 3 };

  it("snaps the moved start to the unit and preserves the length", () => {
    // +0.55 bar rounds to +1 bar.
    const moved = movePreviewLoop(loop, BAR * 0.55, meta, "bar");
    expect(moved).toEqual({ start: BAR * 2, end: BAR * 4 });
  });

  it("a small nudge under half a unit does not move a snapped loop", () => {
    expect(movePreviewLoop(loop, BAR * 0.4, meta, "bar")).toEqual(loop);
  });

  it("clamps at zero without shrinking", () => {
    expect(movePreviewLoop(loop, -BAR * 10, meta, "bar")).toEqual({ start: 0, end: BAR * 2 });
  });

  it("moves freely with snap off", () => {
    expect(movePreviewLoop(loop, 7, meta, "off")).toEqual({ start: BAR + 7, end: BAR * 3 + 7 });
  });
});
