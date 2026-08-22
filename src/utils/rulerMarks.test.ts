import { describe, it, expect } from "vitest";
import { ticksPerBar, type GridMeta } from "./grid";
import { barLabelStep, rulerMarks } from "./rulerMarks";

// Non-4/4 so any hardcoded 4-beats-per-bar math fails loudly: bar = 1440.
const meta: GridMeta = { ticksPerBeat: 480, timeSignature: [3, 4] };
const BAR = ticksPerBar(meta);

describe("barLabelStep", () => {
  it("labels every bar when bars are wide enough", () => {
    expect(barLabelStep(100)).toBe(1);
    expect(barLabelStep(40)).toBe(1);
  });

  it("doubles the step until labels fit the minimum width", () => {
    // 30px bars, 40px minimum: step 2 gives 60px >= 40.
    expect(barLabelStep(30)).toBe(2);
    // 7px bars: step 8 gives 56px >= 40 (step 4 = 28 does not).
    expect(barLabelStep(7)).toBe(8);
  });

  it("respects a custom minimum label width", () => {
    expect(barLabelStep(30, 25)).toBe(1);
    expect(barLabelStep(30, 61)).toBe(4);
  });
});

describe("rulerMarks", () => {
  it("emits bar marks at every derived bar boundary spanning the range", () => {
    // View from mid-bar-1 to mid-bar-3.
    const marks = rulerMarks(BAR * 0.5, BAR * 2.5, meta, 1);
    const bars = marks.filter((m) => m.kind === "bar");
    expect(bars.map((m) => m.tick)).toEqual([0, BAR, BAR * 2, BAR * 3]);
    // Absolute 1-based bar numbers, matching the arrangement ruler.
    expect(bars.map((m) => m.bar)).toEqual([1, 2, 3, 4]);
  });

  it("labels every bar at high zoom", () => {
    // tpp 1 -> bar width = BAR px, far above the 40px label minimum.
    const marks = rulerMarks(0, BAR * 2, meta, 1);
    const bars = marks.filter((m) => m.kind === "bar");
    expect(bars.every((m) => m.labeled)).toBe(true);
  });

  it("thins labels when zoomed far out, keeping bar 1 labeled", () => {
    // tpp such that bar width = 10px -> step 4 (40px labels).
    const tpp = BAR / 10;
    const marks = rulerMarks(0, BAR * 9, meta, tpp);
    const labeled = marks.filter((m) => m.kind === "bar" && m.labeled);
    expect(labeled.map((m) => m.bar)).toEqual([1, 5, 9]);
  });

  it("emits beat subdivisions when beats are wide enough", () => {
    // tpp 10 -> beat width 48px >= 8px minimum.
    const marks = rulerMarks(0, BAR, meta, 10);
    const beats = marks.filter((m) => m.kind === "beat");
    // 3/4 time: beats 2 and 3 of bar 1 (bar boundaries are bar marks).
    expect(beats.map((m) => m.tick)).toEqual([480, 960]);
  });

  it("omits beat subdivisions when beats would be too dense", () => {
    // tpp such that beat width = 4px < 8px minimum.
    const tpp = meta.ticksPerBeat / 4;
    const marks = rulerMarks(0, BAR * 2, meta, tpp);
    expect(marks.filter((m) => m.kind === "beat")).toEqual([]);
  });

  it("never emits marks at negative ticks", () => {
    const marks = rulerMarks(0, BAR, meta, 1);
    expect(marks.every((m) => m.tick >= 0)).toBe(true);
  });
});
