import { describe, it, expect } from "vitest";
import type { Note } from "../types/model";
import {
  OCTAVE_SEMITONES,
  PITCH_RANGES,
  notesIntersectingRect,
  transposeNotes,
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
