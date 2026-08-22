import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { TimelineCanvas } from "./TimelineCanvas";
import type { Track } from "../types/model";

// jsdom canvases have no 2d context and getBoundingClientRect() is all
// zeros, so drawing is a no-op and event clientX/clientY map 1:1 onto
// canvas coordinates — which makes the hit-test/gesture math testable.

function makeTrack(overrides: Partial<Track> = {}): Track {
  return {
    id: "track-1",
    name: "FM1",
    channel: { Fm: 0 },
    instrumentId: null,
    regions: [],
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
    ...overrides,
  };
}

const handlers = {
  onRegionClick: vi.fn(),
  onRegionDoubleClick: vi.fn(),
  onSelectRegions: vi.fn(),
  onRegionMove: vi.fn(),
  onRegionResize: vi.fn(),
  onRegionCreate: vi.fn(),
  onSeek: vi.fn(),
};

// Deliberately non-4/4 so a hardcoded 4-beats-per-bar snap would fail.
const gridProps = {
  ticksPerPixel: 10,
  scrollLeft: 0,
  trackHeight: 60,
  ticksPerBeat: 480,
  beatsPerBar: 3,
  playbackTick: 0,
  playing: false,
  selectedRegions: [],
  seekTick: 0,
  snapMode: "bar" as import("../utils/grid").SnapMode,
};

function renderCanvas(tracks: Track[], overrides: Partial<typeof gridProps> = {}) {
  const { container } = render(
    <TimelineCanvas tracks={tracks} {...gridProps} {...overrides} {...handlers} />,
  );
  const canvas = container.querySelector("canvas");
  expect(canvas).not.toBeNull();
  return canvas as HTMLCanvasElement;
}

describe("TimelineCanvas single-click region select (G6)", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  const region = { id: "region-1", startTick: 0, durationTicks: 5000, notes: [] };

  it("press + sub-slop move + release + click still selects the region", () => {
    // Audit G6 suspected the click handler's `if (drag) return` guard could
    // read a stale non-null drag from the render closure and swallow the
    // selection click. Reproduce the exact browser sequence: mousedown on the
    // region (starts a move-drag), a <4px move, mouseup (clears drag), then
    // the browser-generated click.
    const track = makeTrack({ regions: [region] });
    const canvas = renderCanvas([track]);

    fireEvent.mouseDown(canvas, { clientX: 100, clientY: 10 });
    fireEvent.mouseMove(window, { clientX: 102, clientY: 10 });
    fireEvent.mouseUp(window, { clientX: 102, clientY: 10 });
    fireEvent.click(canvas, { clientX: 102, clientY: 10 });

    expect(handlers.onRegionClick).toHaveBeenCalledWith(track.id, region.id, false);
    expect(handlers.onRegionMove).not.toHaveBeenCalled();
  });

  it("press + release with no move at all selects the region", () => {
    const track = makeTrack({ regions: [region] });
    const canvas = renderCanvas([track]);

    fireEvent.mouseDown(canvas, { clientX: 100, clientY: 10 });
    fireEvent.mouseUp(window, { clientX: 100, clientY: 10 });
    fireEvent.click(canvas, { clientX: 100, clientY: 10 });

    expect(handlers.onRegionClick).toHaveBeenCalledWith(track.id, region.id, false);
  });
});

describe("TimelineCanvas double-click", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("creates a bar-snapped region on an empty stretch of a track lane", () => {
    const canvas = renderCanvas([makeTrack()]);

    const clickX = 200; // inside lane 0, no region there
    fireEvent.doubleClick(canvas, { clientX: clickX, clientY: 10 });

    const barTicks = gridProps.ticksPerBeat * gridProps.beatsPerBar;
    const clickTick = clickX * gridProps.ticksPerPixel;
    const snapped = Math.floor(clickTick / barTicks) * barTicks;
    expect(snapped).toBeGreaterThan(0); // the case must actually exercise snapping
    expect(handlers.onRegionCreate).toHaveBeenCalledWith(0, snapped);
    expect(handlers.onSeek).not.toHaveBeenCalled();
  });

  it("keeps seek below the last lane", () => {
    const canvas = renderCanvas([makeTrack()]);

    const clickX = 200;
    fireEvent.doubleClick(canvas, {
      clientX: clickX,
      clientY: gridProps.trackHeight + 10, // one lane exists; this is below it
    });

    expect(handlers.onSeek).toHaveBeenCalledWith(
      Math.round(clickX * gridProps.ticksPerPixel),
    );
    expect(handlers.onRegionCreate).not.toHaveBeenCalled();
  });

  it("creates a beat-snapped region when snap mode is beat", () => {
    const canvas = renderCanvas([makeTrack()], { snapMode: "beat" });

    const clickX = 200; // tick 2000: inside beat 5 (beat = 480 ticks)
    fireEvent.doubleClick(canvas, { clientX: clickX, clientY: 10 });

    const clickTick = clickX * gridProps.ticksPerPixel;
    const beatSnapped =
      Math.floor(clickTick / gridProps.ticksPerBeat) * gridProps.ticksPerBeat;
    const barTicks = gridProps.ticksPerBeat * gridProps.beatsPerBar;
    // The case must distinguish beat snap from bar snap.
    expect(beatSnapped % barTicks).not.toBe(0);
    expect(handlers.onRegionCreate).toHaveBeenCalledWith(0, beatSnapped);
  });

  it("creates at the exact tick when snap mode is off", () => {
    const canvas = renderCanvas([makeTrack()], { snapMode: "off" });

    const clickX = 201; // tick 2010: on no bar or beat boundary
    fireEvent.doubleClick(canvas, { clientX: clickX, clientY: 10 });

    const clickTick = clickX * gridProps.ticksPerPixel;
    expect(clickTick % gridProps.ticksPerBeat).not.toBe(0);
    expect(handlers.onRegionCreate).toHaveBeenCalledWith(0, clickTick);
  });

  it("opens the region under the cursor instead of creating", () => {
    const region = {
      id: "region-1",
      startTick: 0,
      durationTicks: 5000,
      notes: [],
    };
    const track = makeTrack({ regions: [region] });
    const canvas = renderCanvas([track]);

    fireEvent.doubleClick(canvas, { clientX: 200, clientY: 10 });

    expect(handlers.onRegionDoubleClick).toHaveBeenCalledWith(track.id, region.id);
    expect(handlers.onRegionCreate).not.toHaveBeenCalled();
    expect(handlers.onSeek).not.toHaveBeenCalled();
  });
});
