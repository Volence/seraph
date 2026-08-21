import { describe, it, expect } from "vitest";
import { ticksPerBar, type GridMeta } from "./grid";

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
