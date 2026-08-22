import { describe, it, expect } from "vitest";
import type { Note } from "../types/model";
import {
  OCTAVE_SEMITONES,
  PITCH_RANGES,
  notesIntersectingRect,
  marqueeRectFromView,
  marqueePreviewSelection,
  transposeNotes,
  nudgeNotes,
  type MarqueeRect,
} from "./pianoRollEdit";

function note(tick: number, pitch: number, durationTicks = 100): Note {
  return { tick, pitch, velocity: 100, durationTicks };
}

function rect(tickMin: number, tickMax: number, pitchMin: number, pitchMax: number): MarqueeRect {
  return { tickMin, tickMax, pitchMin, pitchMax };
}

describe("notesIntersectingRect", () => {
  const notes = [
    note(0, 60, 100), // ends at 100
    note(200, 62, 100), // 200..300
    note(500, 64, 100), // 500..600
  ];

  it("selects notes whose time span overlaps the rect", () => {
    expect(notesIntersectingRect(notes, rect(150, 350, 50, 70))).toEqual([1]);
  });

  it("includes a note that merely straddles the rect edge in time", () => {
    // note 0 spans 0..100; rect starts at 50 — partial overlap counts
    expect(notesIntersectingRect(notes, rect(50, 120, 50, 70))).toEqual([0]);
  });

  it("excludes a note that only touches the rect boundary exactly (end == tickMin)", () => {
    // note 0 ends at tick 100; a rect starting exactly at 100 does not overlap it
    expect(notesIntersectingRect(notes, rect(100, 150, 50, 70))).toEqual([]);
  });

  it("filters by pitch band inclusively", () => {
    // rect pitch band exactly [62, 62] catches only the pitch-62 note
    expect(notesIntersectingRect(notes, rect(0, 1000, 62, 62))).toEqual([1]);
  });

  it("excludes notes outside the pitch band even when time overlaps", () => {
    expect(notesIntersectingRect(notes, rect(0, 1000, 70, 80))).toEqual([]);
  });

  it("returns all overlapping notes, in index order", () => {
    expect(notesIntersectingRect(notes, rect(0, 1000, 0, 127))).toEqual([0, 1, 2]);
  });

  it("returns empty for an empty note list", () => {
    expect(notesIntersectingRect([], rect(0, 1000, 0, 127))).toEqual([]);
  });
});

describe("transposeNotes", () => {
  const [fmLo, fmHi] = PITCH_RANGES.fm;

  it("octave shortcut constant is 12 semitones", () => {
    expect(OCTAVE_SEMITONES).toBe(12);
  });

  it("transposes every selected note by the delta, leaving others alone", () => {
    const notes = [note(0, 60), note(100, 64), note(200, 67)];
    const result = transposeNotes(notes, new Set([0, 2]), 1, fmLo, fmHi);
    expect(result).toEqual([
      { index: 0, pitch: 61 },
      { index: 2, pitch: 68 },
    ]);
  });

  it("transposes by a full octave via the named constant", () => {
    const notes = [note(0, 60)];
    const result = transposeNotes(notes, new Set([0]), OCTAVE_SEMITONES, fmLo, fmHi);
    expect(result).toEqual([{ index: 0, pitch: 60 + OCTAVE_SEMITONES }]);
  });

  it("allows a move that lands exactly on the range bounds", () => {
    const notes = [note(0, fmHi - 1), note(100, fmLo + 1)];
    expect(transposeNotes(notes, new Set([0]), 1, fmLo, fmHi)).toEqual([
      { index: 0, pitch: fmHi },
    ]);
    expect(transposeNotes(notes, new Set([1]), -1, fmLo, fmHi)).toEqual([
      { index: 1, pitch: fmLo },
    ]);
  });

  it("blocks the whole move when ANY selected note would leave the range (intervals stay intact)", () => {
    // one note is at the ceiling; the other has room. Blocking, not per-note
    // clamping, is what preserves the interval between them.
    const notes = [note(0, fmHi), note(100, 60)];
    expect(transposeNotes(notes, new Set([0, 1]), 1, fmLo, fmHi)).toBeNull();
    expect(transposeNotes(notes, new Set([0, 1]), -1, fmLo, fmHi)).not.toBeNull();
  });

  it("blocks at the low bound too", () => {
    const notes = [note(0, fmLo)];
    expect(transposeNotes(notes, new Set([0]), -1, fmLo, fmHi)).toBeNull();
  });

  it("returns null for an empty selection", () => {
    expect(transposeNotes([note(0, 60)], new Set(), 1, fmLo, fmHi)).toBeNull();
  });
});

describe("nudgeNotes", () => {
  const REGION = 1920; // region length in ticks

  it("moves every selected note by the delta, leaving others alone", () => {
    const notes = [note(0, 60), note(480, 64), note(960, 67)];
    expect(nudgeNotes(notes, new Set([0, 2]), 120, REGION)).toEqual([
      { index: 0, tick: 120 },
      { index: 2, tick: 1080 },
    ]);
  });

  it("moves left with a negative delta", () => {
    const notes = [note(480, 60)];
    expect(nudgeNotes(notes, new Set([0]), -120, REGION)).toEqual([
      { index: 0, tick: 360 },
    ]);
  });

  it("allows a move that lands exactly on the bounds", () => {
    // Left edge: tick 120 - 120 = 0 is legal.
    expect(nudgeNotes([note(120, 60)], new Set([0]), -120, REGION)).toEqual([
      { index: 0, tick: 0 },
    ]);
    // Right edge: note end 1800 + 120 = 1920 == region end is legal.
    expect(nudgeNotes([note(1700, 60)], new Set([0]), 120, REGION)).toEqual([
      { index: 0, tick: 1820 },
    ]);
  });

  it("blocks the whole move when ANY note would cross tick 0 (rhythms stay intact)", () => {
    const notes = [note(0, 60), note(480, 64)];
    expect(nudgeNotes(notes, new Set([0, 1]), -120, REGION)).toBeNull();
  });

  it("blocks the whole move when ANY note end would pass the region end", () => {
    // note(1820).end = 1920: +1 tick overflows the region.
    const notes = [note(0, 60), note(1820, 64)];
    expect(nudgeNotes(notes, new Set([0, 1]), 1, REGION)).toBeNull();
  });

  it("returns null for an empty selection", () => {
    expect(nudgeNotes([note(0, 60)], new Set(), 120, REGION)).toBeNull();
  });

  it("returns null for a stale index", () => {
    expect(nudgeNotes([note(0, 60)], new Set([5]), 120, REGION)).toBeNull();
  });
});

describe("marqueeRectFromView", () => {
  // View geometry: scrollLeft 200 view-px, 2 ticks per px, rows drawn
  // top-down from pitch 95, 16 px per row.
  const scrollLeft = 200;
  const tpp = 2;
  const maxPitch = 95;
  const rowHeight = 16;

  it("maps view px to ticks and rows to an inclusive pitch band", () => {
    const r = marqueeRectFromView(50, 0, 150, 40, scrollLeft, tpp, maxPitch, rowHeight);
    // Ticks: (viewX + scrollLeft) * ticksPerPixel.
    expect(r.tickMin).toBe((50 + scrollLeft) * tpp);
    expect(r.tickMax).toBe((150 + scrollLeft) * tpp);
    // y 0..40 touches rows 0..2 => pitches maxPitch-2 .. maxPitch.
    expect(r.pitchMax).toBe(maxPitch);
    expect(r.pitchMin).toBe(maxPitch - Math.floor(40 / rowHeight));
  });

  it("normalizes drags in any direction (start below/right of end)", () => {
    const fwd = marqueeRectFromView(50, 0, 150, 40, scrollLeft, tpp, maxPitch, rowHeight);
    const rev = marqueeRectFromView(150, 40, 50, 0, scrollLeft, tpp, maxPitch, rowHeight);
    expect(rev).toEqual(fwd);
  });

  it("agrees with notesIntersectingRect for a mid-drag rect", () => {
    // The live-preview path: the rect from an in-flight drag must hit the
    // same notes the mouseup commit would.
    const notes = [note(500, 95, 100), note(700, 94, 100), note(5000, 95, 100)];
    const r = marqueeRectFromView(50, 0, 150, 40, scrollLeft, tpp, maxPitch, rowHeight);
    // Rect covers ticks 500..700, pitches 93..95: note 0 overlaps in time,
    // note 1 only touches the boundary (tick 700 == tickMax, excluded),
    // note 2 is far right.
    expect(notesIntersectingRect(notes, r)).toEqual([0]);
  });
});

describe("marqueePreviewSelection", () => {
  it("previews a plain marquee as a replacement (hits only)", () => {
    expect(marqueePreviewSelection(new Set([0, 3]), [1, 2], false)).toEqual(new Set([1, 2]));
  });

  it("keeps already-selected notes highlighted in additive (Shift) mode", () => {
    expect(marqueePreviewSelection(new Set([0, 3]), [1, 2], true)).toEqual(new Set([0, 1, 2, 3]));
  });

  it("previews the clear for a plain drag over nothing", () => {
    expect(marqueePreviewSelection(new Set([0, 3]), [], false)).toEqual(new Set());
  });

  it("leaves the selection alone for an additive drag over nothing", () => {
    expect(marqueePreviewSelection(new Set([0, 3]), [], true)).toEqual(new Set([0, 3]));
  });
});
