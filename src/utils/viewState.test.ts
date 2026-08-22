import { describe, it, expect, beforeEach } from "vitest";
import type { Track } from "../types/model";
import {
  VIEW_STATE_KEY,
  MAX_VIEW_STATE_PROJECTS,
  getViewState,
  patchViewState,
  clearViewState,
  sanitizeViewState,
  resolveOpenRegion,
  resolveLoop,
  songEndTick,
  clampNumber,
  clampIndex,
} from "./viewState";

function track(id: string, regionIds: string[]): Track {
  return {
    id,
    name: `Track ${id}`,
    channel: { Fm: 0 },
    instrumentId: null,
    muted: false,
    solo: false,
    volume: 1,
    pan: "Center",
    regions: regionIds.map((rid, i) => ({
      id: rid,
      startTick: i * 1920,
      durationTicks: 1920,
      notes: [],
      instrumentId: null,
    })),
  } as unknown as Track;
}

describe("viewState storage", () => {
  beforeEach(() => {
    localStorage.clear();
  });

  it("returns an empty view when nothing is stored", () => {
    expect(getViewState("/songs/A")).toEqual({});
  });

  it("round-trips a patch under the documented key", () => {
    patchViewState("/songs/A", { panel: { collapsed: true, height: 420 } });
    expect(getViewState("/songs/A")).toEqual({ panel: { collapsed: true, height: 420 } });
    expect(JSON.parse(localStorage.getItem(VIEW_STATE_KEY)!)).toEqual([
      { path: "/songs/A", state: { panel: { collapsed: true, height: 420 } } },
    ]);
  });

  it("keeps each project's view separate", () => {
    patchViewState("/songs/A", { pianoRoll: { gridIdx: 2 } });
    patchViewState("/songs/B", { pianoRoll: { gridIdx: 7 } });
    expect(getViewState("/songs/A").pianoRoll).toEqual({ gridIdx: 2 });
    expect(getViewState("/songs/B").pianoRoll).toEqual({ gridIdx: 7 });
  });

  it("treats trailing-slash variants as the same project", () => {
    patchViewState("/songs/A", { pianoRoll: { gridIdx: 3 } });
    expect(getViewState("/songs/A/").pianoRoll).toEqual({ gridIdx: 3 });
    patchViewState("/songs/A/", { pianoRoll: { gridIdx: 5 } });
    expect(JSON.parse(localStorage.getItem(VIEW_STATE_KEY)!)).toHaveLength(1);
  });

  it("merges successive patches of different slices", () => {
    patchViewState("/songs/A", { panel: { collapsed: false, height: 300 } });
    patchViewState("/songs/A", { openRegion: { trackId: "t1", regionId: "r1" } });
    expect(getViewState("/songs/A")).toEqual({
      panel: { collapsed: false, height: 300 },
      openRegion: { trackId: "t1", regionId: "r1" },
    });
  });

  it("lets a null patch clear the open region and the loop", () => {
    patchViewState("/songs/A", {
      openRegion: { trackId: "t1", regionId: "r1" },
      loop: { start: 0, end: 1920, enabled: true },
      panel: { collapsed: true },
    });
    patchViewState("/songs/A", { openRegion: null, loop: null });
    // Cleared reads back as ABSENT, not as a stored null — a closed roll and
    // a never-opened roll are the same thing to every consumer.
    const view = getViewState("/songs/A");
    expect(view.openRegion ?? null).toBeNull();
    expect(view.loop ?? null).toBeNull();
    // …and clearing one slice must not wipe the others.
    expect(view.panel).toEqual({ collapsed: true });
  });

  it("ignores an empty project path in both directions", () => {
    patchViewState("", { pianoRoll: { gridIdx: 1 } });
    expect(localStorage.getItem(VIEW_STATE_KEY)).toBeNull();
    expect(getViewState("")).toEqual({});
  });

  it("caps the table at MAX_VIEW_STATE_PROJECTS, dropping the least recent", () => {
    for (let i = 0; i < MAX_VIEW_STATE_PROJECTS + 3; i++) {
      patchViewState(`/songs/p${i}`, { pianoRoll: { gridIdx: 1 } });
    }
    const stored = JSON.parse(localStorage.getItem(VIEW_STATE_KEY)!);
    expect(stored).toHaveLength(MAX_VIEW_STATE_PROJECTS);
    expect(stored[0].path).toBe(`/songs/p${MAX_VIEW_STATE_PROJECTS + 2}`);
    expect(getViewState("/songs/p0")).toEqual({});
  });

  it("promotes a re-patched project to the front rather than duplicating it", () => {
    patchViewState("/songs/A", { pianoRoll: { gridIdx: 1 } });
    patchViewState("/songs/B", { pianoRoll: { gridIdx: 1 } });
    patchViewState("/songs/A", { pianoRoll: { gridIdx: 6 } });
    const stored = JSON.parse(localStorage.getItem(VIEW_STATE_KEY)!);
    expect(stored.map((e: { path: string }) => e.path)).toEqual(["/songs/A", "/songs/B"]);
  });

  it("clears one project's view and leaves the others", () => {
    patchViewState("/songs/A", { pianoRoll: { gridIdx: 1 } });
    patchViewState("/songs/B", { pianoRoll: { gridIdx: 2 } });
    clearViewState("/songs/A");
    expect(getViewState("/songs/A")).toEqual({});
    expect(getViewState("/songs/B").pianoRoll).toEqual({ gridIdx: 2 });
  });

  it("survives corrupt stored JSON by starting fresh", () => {
    localStorage.setItem(VIEW_STATE_KEY, "{not json[");
    expect(getViewState("/songs/A")).toEqual({});
    patchViewState("/songs/A", { pianoRoll: { gridIdx: 4 } });
    expect(getViewState("/songs/A").pianoRoll).toEqual({ gridIdx: 4 });
  });

  it("survives a stored value that is not an array", () => {
    localStorage.setItem(VIEW_STATE_KEY, JSON.stringify({ "/songs/A": { panel: {} } }));
    expect(getViewState("/songs/A")).toEqual({});
  });
});

describe("sanitizeViewState", () => {
  it("returns an empty view for non-objects", () => {
    expect(sanitizeViewState(null)).toEqual({});
    expect(sanitizeViewState(42)).toEqual({});
    expect(sanitizeViewState("nope")).toEqual({});
  });

  it("drops wrong-typed fields and keeps the intact ones alongside them", () => {
    expect(
      sanitizeViewState({
        arrangement: { ticksPerPixel: "wide", scrollLeft: 120, snapMode: "beat" },
      }),
    ).toEqual({ arrangement: { scrollLeft: 120, snapMode: "beat" } });
  });

  it("rejects NaN and Infinity, which survive JSON round-trips as nulls or strings", () => {
    expect(
      sanitizeViewState({ arrangement: { ticksPerPixel: NaN, scrollLeft: Infinity } }),
    ).toEqual({});
  });

  it("rejects an unknown snap mode", () => {
    expect(sanitizeViewState({ arrangement: { snapMode: "triplet" } })).toEqual({});
  });

  it("filters non-string entries out of collapsedChannels", () => {
    expect(
      sanitizeViewState({ arrangement: { collapsedChannels: ["FM1", 7, null, "", "PSG2"] } }),
    ).toEqual({ arrangement: { collapsedChannels: ["FM1", "PSG2"] } });
  });

  it("drops a half-written loop rather than restoring half a range", () => {
    expect(sanitizeViewState({ loop: { start: 0, enabled: true } })).toEqual({});
    expect(sanitizeViewState({ loop: { start: 0, end: 960 } })).toEqual({});
  });

  it("drops an open-region reference missing either id", () => {
    expect(sanitizeViewState({ openRegion: { trackId: "t1" } })).toEqual({});
    expect(sanitizeViewState({ openRegion: { trackId: "", regionId: "r1" } })).toEqual({});
  });

  it("keeps a fully-typed record intact", () => {
    const full = {
      arrangement: { ticksPerPixel: 6.4, scrollLeft: 200, snapMode: "off", collapsedChannels: ["FM1"] },
      panel: { collapsed: true, height: 380 },
      pianoRoll: { gridIdx: 5 },
      loop: { start: 1920, end: 3840, enabled: true },
      openRegion: { trackId: "t1", regionId: "r1" },
    };
    expect(sanitizeViewState(full)).toEqual(full);
  });
});

describe("resolveOpenRegion", () => {
  const tracks = [track("t1", ["r1", "r2"]), track("t2", ["r3"])];

  it("rebuilds the selection from the LIVE track, not from storage", () => {
    expect(resolveOpenRegion({ trackId: "t1", regionId: "r2" }, tracks)).toEqual({
      trackId: "t1",
      trackName: "Track t1",
      regionId: "r2",
      channelType: "fm",
      startTick: 1920,
      durationTicks: 1920,
    });
  });

  it("drops a region deleted while the project was closed", () => {
    expect(resolveOpenRegion({ trackId: "t1", regionId: "gone" }, tracks)).toBeNull();
  });

  it("drops a region whose whole track was deleted", () => {
    expect(resolveOpenRegion({ trackId: "gone", regionId: "r1" }, tracks)).toBeNull();
  });

  it("drops a region id that now lives on a different track", () => {
    expect(resolveOpenRegion({ trackId: "t2", regionId: "r1" }, tracks)).toBeNull();
  });

  it("returns null for an absent reference and for an empty project", () => {
    expect(resolveOpenRegion(null, tracks)).toBeNull();
    expect(resolveOpenRegion(undefined, tracks)).toBeNull();
    expect(resolveOpenRegion({ trackId: "t1", regionId: "r1" }, [])).toBeNull();
  });
});

describe("songEndTick / resolveLoop", () => {
  it("reports the last tick any region covers", () => {
    expect(songEndTick([track("t1", ["r1", "r2"]), track("t2", ["r3"])])).toBe(3840);
    expect(songEndTick([])).toBe(0);
  });

  it("keeps a range that still sits inside the project", () => {
    const loop = { start: 0, end: 1920, enabled: true };
    expect(resolveLoop(loop, 3840)).toEqual(loop);
  });

  it("drops a range whose content was deleted while the project was closed", () => {
    expect(resolveLoop({ start: 7680, end: 9600, enabled: true }, 3840)).toBeNull();
  });

  it("keeps a range when the project has no regions to measure against", () => {
    const loop = { start: 7680, end: 9600, enabled: false };
    expect(resolveLoop(loop, 0)).toEqual(loop);
  });

  it("drops an inverted, empty or negative range", () => {
    expect(resolveLoop({ start: 1920, end: 960, enabled: true }, 3840)).toBeNull();
    expect(resolveLoop({ start: 1920, end: 1920, enabled: true }, 3840)).toBeNull();
    expect(resolveLoop({ start: -960, end: 960, enabled: true }, 3840)).toBeNull();
  });

  it("returns null for an absent loop", () => {
    expect(resolveLoop(null, 3840)).toBeNull();
    expect(resolveLoop(undefined, 3840)).toBeNull();
  });
});

describe("clampNumber / clampIndex", () => {
  it("falls back when the stored value is absent", () => {
    expect(clampNumber(undefined, 1, 10, 4)).toBe(4);
    expect(clampIndex(undefined, 8, 4)).toBe(4);
  });

  it("clamps an out-of-range number to the bound", () => {
    expect(clampNumber(0.001, 0.05, 1920, 12)).toBe(0.05);
    expect(clampNumber(99999, 0.05, 1920, 12)).toBe(1920);
    expect(clampNumber(6.4, 0.05, 1920, 12)).toBe(6.4);
  });

  it("falls back for an index outside the current option list", () => {
    expect(clampIndex(8, 8, 4)).toBe(4);
    expect(clampIndex(-1, 8, 4)).toBe(4);
    expect(clampIndex(7, 8, 4)).toBe(7);
  });

  it("falls back for a fractional index", () => {
    expect(clampIndex(2.5, 8, 4)).toBe(4);
  });
});
