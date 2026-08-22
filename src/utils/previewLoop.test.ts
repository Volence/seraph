import { describe, it, expect } from "vitest";
import { defaultPreviewLoop, dragPreviewLoop } from "./previewLoop";
import type { GridMeta } from "./grid";

// Deliberately non-4/4 so hardcoded 4-beats-per-bar math would fail:
// bar = 480 * 3 = 1440 ticks, beat = 480 ticks.
const m34: GridMeta = { ticksPerBeat: 480, timeSignature: [3, 4] };

describe("defaultPreviewLoop", () => {
  it("is one bar at the bar containing the seek cursor", () => {
    // Seek tick 1500 lies in bar 2 ([1440, 2880) in 3/4 at 480 tpb).
    expect(defaultPreviewLoop(1500, m34)).toEqual({ start: 1440, end: 2880 });
  });

  it("is bars 1 when the cursor was never moved", () => {
    expect(defaultPreviewLoop(0, m34)).toEqual({ start: 0, end: 1440 });
  });
});

describe("dragPreviewLoop", () => {
  it("bar snap: floors the low edge, ceils the high edge", () => {
    // Drag from tick 1500 to 3000: encloses bars 2-3 ([1440, 4320)).
    expect(dragPreviewLoop(1500, 3000, m34, "bar")).toEqual({
      start: 1440,
      end: 4320,
    });
  });

  it("is direction-agnostic (right-to-left drag gives the same range)", () => {
    expect(dragPreviewLoop(3000, 1500, m34, "bar")).toEqual(
      dragPreviewLoop(1500, 3000, m34, "bar"),
    );
  });

  it("bar snap: a zero-length drag still spans one full bar (min one bar)", () => {
    expect(dragPreviewLoop(1500, 1500, m34, "bar")).toEqual({
      start: 1440,
      end: 2880,
    });
  });

  it("beat snap: snaps edges to beats, min one beat", () => {
    // 1000..1000 lies in beat 3 ([960, 1440)).
    expect(dragPreviewLoop(1000, 1000, m34, "beat")).toEqual({
      start: 960,
      end: 1440,
    });
  });

  it("off: exact ticks, min one tick", () => {
    expect(dragPreviewLoop(1001, 1234, m34, "off")).toEqual({
      start: 1001,
      end: 1234,
    });
    expect(dragPreviewLoop(1001, 1001, m34, "off")).toEqual({
      start: 1001,
      end: 1002,
    });
  });

  it("clamps to tick 0 when the drag runs off the left edge", () => {
    expect(dragPreviewLoop(-500, 100, m34, "bar")).toEqual({
      start: 0,
      end: 1440,
    });
  });
});
