import { describe, it, expect } from "vitest";
import { ticksPerBar, snapUnit, snapTick, type GridMeta } from "./grid";

function meta(ticksPerBeat: number, timeSignature: [number, number]): GridMeta {
  return { ticksPerBeat, timeSignature };
}

describe("ticksPerBar", () => {
  it("is ticks-per-beat times beats-per-bar in 4/4", () => {
    const m = meta(480, [4, 4]);
    expect(ticksPerBar(m)).toBe(m.ticksPerBeat * m.timeSignature[0]);
  });

  it("follows the time-signature numerator, not a hardcoded 4", () => {
    const m = meta(480, [3, 4]);
    expect(ticksPerBar(m)).toBe(m.ticksPerBeat * 3);
  });

  it("follows ticksPerBeat, not a hardcoded 480", () => {
    const m = meta(96, [4, 4]);
    expect(ticksPerBar(m)).toBe(96 * 4);
  });
});

// Deliberately non-4/4 so a hardcoded 4-beats-per-bar unit would fail:
// bar = 480 * 3 = 1440 ticks, beat = 480 ticks.
const m34 = meta(480, [3, 4]);

describe("snapUnit", () => {
  it("bar mode is one bar (ticksPerBeat * numerator)", () => {
    expect(snapUnit(m34, "bar")).toBe(1440);
  });

  it("beat mode is ticksPerBeat", () => {
    expect(snapUnit(m34, "beat")).toBe(480);
  });

  it("off mode is a single tick", () => {
    expect(snapUnit(m34, "off")).toBe(1);
  });
});

describe("snapTick", () => {
  it("bar mode floors to the enclosing bar", () => {
    // Tick 1500 lies in bar 2 (bar = 1440 ticks in 3/4 at 480 tpb).
    expect(snapTick(1500, m34, "bar")).toBe(1440);
  });

  it("beat mode floors to the enclosing beat", () => {
    // Tick 1500 lies in beat 4 (beat = 480 ticks).
    expect(snapTick(1500, m34, "beat")).toBe(1440);
    expect(snapTick(1000, m34, "beat")).toBe(960);
  });

  it("off mode leaves the tick alone", () => {
    expect(snapTick(1501, m34, "off")).toBe(1501);
  });

  it("matches the old snapToBar behavior in bar mode", () => {
    // Parity with the hardcoded snapToBar this replaces:
    // Math.floor(tick / ticksPerBar) * ticksPerBar.
    const bar = ticksPerBar(m34);
    for (const t of [0, 1, 1439, 1440, 2879, 4321]) {
      expect(snapTick(t, m34, "bar")).toBe(Math.floor(t / bar) * bar);
    }
  });
});
