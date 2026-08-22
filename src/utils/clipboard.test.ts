import { describe, it, expect, beforeEach } from "vitest";
import type { Note } from "../types/model";
import {
  copyNotes,
  getNoteClipboard,
  copyRegions,
  getRegionClipboard,
  lastCopiedKind,
  resetClipboardForTest,
  planNotePaste,
} from "./clipboard";

function note(tick: number, pitch: number, durationTicks = 240): Note {
  return { tick, pitch, velocity: 100, durationTicks };
}

beforeEach(() => {
  resetClipboardForTest();
});

describe("note clipboard", () => {
  it("copies only the selected notes, snapshotted against later mutation", () => {
    const notes = [note(0, 60), note(480, 64), note(960, 67)];
    const count = copyNotes(notes, new Set([0, 2]));
    expect(count).toBe(2);
    // Mutating the source after the copy must not leak into the clipboard.
    notes[0].pitch = 99;
    const clip = getNoteClipboard();
    expect(clip.map((n) => n.pitch)).toEqual([60, 67]);
    expect(lastCopiedKind()).toBe("notes");
  });

  it("ignores stale indices and keeps note order by tick", () => {
    const notes = [note(480, 64), note(0, 60)];
    copyNotes(notes, new Set([1, 0, 7]));
    expect(getNoteClipboard().map((n) => n.tick)).toEqual([0, 480]);
  });

  it("copying an empty selection leaves the clipboard untouched", () => {
    copyNotes([note(0, 60)], new Set([0]));
    copyNotes([note(0, 60)], new Set());
    expect(getNoteClipboard()).toHaveLength(1);
  });
});

describe("region clipboard", () => {
  it("stores entries and flips the last-copied kind to regions", () => {
    copyNotes([note(0, 60)], new Set([0]));
    copyRegions([
      {
        trackId: "t1",
        regionId: "r1",
        payload: { startTick: 0, durationTicks: 1920, notes: [note(0, 60)] },
      },
    ]);
    expect(getRegionClipboard()).toHaveLength(1);
    expect(lastCopiedKind()).toBe("regions");
    // The note clipboard survives — only the paste arbitration flips.
    expect(getNoteClipboard()).toHaveLength(1);
  });
});

describe("planNotePaste", () => {
  it("places notes relative to the earliest copied note at the anchor", () => {
    // Earliest tick 480 becomes the anchor; the 960 note keeps its +480 offset.
    const clip = [note(480, 60), note(960, 64)];
    const plan = planNotePaste(clip, 1200, 7680);
    expect(plan.placements).toEqual([
      { tick: 1200, pitch: 60, velocity: 100, durationTicks: 240 },
      { tick: 1680, pitch: 64, velocity: 100, durationTicks: 240 },
    ]);
    expect(plan.skipped).toBe(0);
  });

  it("skips notes that would start at/past the region end and counts them", () => {
    const clip = [note(0, 60), note(1000, 64)];
    // Region is 1000 ticks: the second note would start exactly at the end.
    const plan = planNotePaste(clip, 0, 1000);
    expect(plan.placements).toHaveLength(1);
    expect(plan.placements[0].pitch).toBe(60);
    expect(plan.skipped).toBe(1);
  });

  it("clamps a duration that overflows the region end", () => {
    const clip = [note(0, 60, 480)];
    const plan = planNotePaste(clip, 900, 1000);
    expect(plan.placements).toEqual([
      { tick: 900, pitch: 60, velocity: 100, durationTicks: 100 },
    ]);
    expect(plan.skipped).toBe(0);
  });

  it("returns an empty plan for an empty clipboard", () => {
    const plan = planNotePaste([], 0, 1000);
    expect(plan.placements).toEqual([]);
    expect(plan.skipped).toBe(0);
  });
});
