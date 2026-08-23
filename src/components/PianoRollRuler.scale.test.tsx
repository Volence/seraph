import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRoll } from "./PianoRoll";
import * as ipc from "../api/ipc";
import * as grid from "../utils/grid";
import { MIN_REGION_VIEW_PX } from "../utils/pianoRollEdit";
import type { SelectedRegion, SongMetadata, Track } from "../types/model";

vi.mock("../api/ipc");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

/**
 * Live-bug repro (owner screenshot): the piano-roll ruler drew ~1281
 * one-pixel bars over a small region while the note grid looked normal.
 * The grid only LOOKED normal — it silently skips gridlines denser than
 * 4px and floors note widths at 2px, so a broken shared ticksPerPixel is
 * invisible there while the ruler renders it faithfully. Two real paths
 * put ticksPerPixel in that state:
 *
 *  A. Stale zoom across region switches — the PianoRoll instance persists
 *     when a different region opens, keeping the previous region's zoom.
 *  B. An absurd zoom-out clamp (2 bars per 1px) reachable by wheel/drag.
 *
 * These tests execute the real component with a recording 2D context and
 * assert on what the ruler actually draws.
 */

const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [4, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};
const BAR = grid.ticksPerBar(meta);

// ---- recording 2D context ---------------------------------------------

interface CanvasRecord {
  fillText: { text: string; x: number }[];
  fillRect: { x: number; y: number; w: number; h: number }[];
}

const records = new Map<HTMLCanvasElement, CanvasRecord>();

function recordFor(canvas: HTMLCanvasElement): CanvasRecord {
  let rec = records.get(canvas);
  if (!rec) {
    rec = { fillText: [], fillRect: [] };
    records.set(canvas, rec);
  }
  return rec;
}

function makeContext(rec: CanvasRecord): CanvasRenderingContext2D {
  return new Proxy(
    {},
    {
      get(_t, prop) {
        if (prop === "clearRect") {
          // Every draw() starts by clearing — treat it as a frame boundary
          // so the record always holds exactly the LAST rendered frame.
          return () => {
            rec.fillText.length = 0;
            rec.fillRect.length = 0;
          };
        }
        if (prop === "fillText") {
          return (text: string, x: number) => rec.fillText.push({ text, x });
        }
        if (prop === "fillRect") {
          return (x: number, y: number, w: number, h: number) => rec.fillRect.push({ x, y, w, h });
        }
        if (prop === "measureText") {
          return () => ({ width: 0 });
        }
        return () => {};
      },
      set() {
        return true;
      },
    },
  ) as unknown as CanvasRenderingContext2D;
}

const VIEW_W = 1280;
const RECT = {
  x: 0, y: 0, top: 0, left: 0, right: VIEW_W, bottom: 40,
  width: VIEW_W, height: 40,
  toJSON: () => {},
} as DOMRect;

let origGetContext: typeof HTMLCanvasElement.prototype.getContext;
let origGetRect: typeof Element.prototype.getBoundingClientRect;

beforeEach(() => {
  vi.clearAllMocks();
  records.clear();
  origGetContext = HTMLCanvasElement.prototype.getContext;
  origGetRect = Element.prototype.getBoundingClientRect;
  HTMLCanvasElement.prototype.getContext = function (this: HTMLCanvasElement) {
    return makeContext(recordFor(this));
  } as unknown as typeof HTMLCanvasElement.prototype.getContext;
  Element.prototype.getBoundingClientRect = () => RECT;
});

afterEach(() => {
  HTMLCanvasElement.prototype.getContext = origGetContext;
  Element.prototype.getBoundingClientRect = origGetRect;
});

// ---- fixtures -----------------------------------------------------------

function makeRegion(id: string, startTick: number, durationTicks: number): SelectedRegion {
  return {
    trackId: "track-1",
    trackName: "Lead",
    regionId: id,
    channelType: "fm",
    startTick,
    durationTicks,
  };
}

function mockTracks(regions: SelectedRegion[]) {
  const track: Track = {
    id: "track-1",
    name: "Lead",
    channel: { Fm: 0 },
    instrumentId: "inst-1",
    regions: regions.map((r) => ({
      id: r.regionId,
      startTick: r.startTick,
      durationTicks: r.durationTicks,
      notes: [],
    })),
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
  vi.mocked(ipc.listTracks).mockResolvedValue([track]);
}

function rollProps(region: SelectedRegion) {
  return {
    region,
    projectPath: null,
    onClose: vi.fn(),
    playing: false,
    projectMeta: meta,
    seekTick: 0,
    onSeek: vi.fn(),
  };
}

/** Canvas order in the DOM: [0] bar ruler, [1] key column, [2] note grid. */
function rulerCanvas(container: HTMLElement): HTMLCanvasElement {
  return container.querySelectorAll("canvas")[0] as HTMLCanvasElement;
}

function gridContainer(container: HTMLElement): HTMLElement {
  return container.querySelectorAll("canvas")[2].parentElement as HTMLElement;
}

/** Bar labels of the LAST completed ruler draw (draws start with clearRect;
 *  we just clear our record before the render that matters). */
function labeledBars(container: HTMLElement): number[] {
  const rec = records.get(rulerCanvas(container));
  return (rec?.fillText ?? []).map((f) => parseInt(f.text, 10));
}

function clearRulerRecord(container: HTMLElement) {
  const rec = records.get(rulerCanvas(container));
  if (rec) {
    rec.fillText.length = 0;
    rec.fillRect.length = 0;
  }
}

async function flush() {
  await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
  await act(async () => {});
}

/** Ctrl+wheel zoom-out steps on the note grid (the real user gesture). */
function zoomOutHard(container: HTMLElement, steps: number) {
  const target = gridContainer(container);
  for (let i = 0; i < steps; i++) {
    fireEvent.wheel(target, { deltaY: 100, ctrlKey: true });
  }
}

// ---- the repro ----------------------------------------------------------

describe("piano-roll ruler shares the note grid's effective scale", () => {
  it("switching from a zoomed-out large region to a small one refits the view (stale-zoom repro)", async () => {
    const large = makeRegion("region-large", 0, 512 * BAR);
    const small = makeRegion("region-small", 0, 4 * BAR);
    mockTracks([large, small]);

    const { container, rerender } = render(<PianoRoll {...rollProps(large)} />);
    await flush();

    // Zoom far out on the large region (hits whatever clamp exists).
    zoomOutHard(container, 40);

    // Open the small region in the SAME PianoRoll instance (no remount —
    // exactly what BottomPanel does).
    clearRulerRecord(container);
    rerender(<PianoRoll {...rollProps(small)} />);
    await act(async () => {});

    const bars = labeledBars(container);
    // The ruler must show the small region's bars: consecutive absolute
    // bar numbers starting at the region's first bar — never the thinned
    // every-Nth labels of a stale zoomed-out scale.
    expect(bars.length).toBeGreaterThan(0);
    expect(bars[0]).toBe(Math.floor(small.startTick / BAR) + 1);
    for (let i = 1; i < bars.length; i++) {
      expect(bars[i] - bars[i - 1]).toBe(1);
    }
    // And the region itself must be wide on screen: its 4 bars span at
    // least the zoom-out floor.
    expect(bars.length).toBeLessThanOrEqual(Math.ceil((VIEW_W * 4 * BAR) / (MIN_REGION_VIEW_PX * BAR)) + 2);
  });

  it("zoom-out on a small region floors at the region-derived clamp (clamp repro)", async () => {
    const small = makeRegion("region-small", 0, 4 * BAR);
    mockTracks([small]);

    const { container } = render(<PianoRoll {...rollProps(small)} />);
    await flush();

    clearRulerRecord(container);
    zoomOutHard(container, 40);
    await act(async () => {});

    const bars = labeledBars(container);
    // Even at full zoom-out the bars stay consecutive (no thinning: each
    // bar is at least MIN_REGION_VIEW_PX / 4 px wide) and start at bar 1.
    expect(bars.length).toBeGreaterThan(0);
    expect(bars[0]).toBe(1);
    for (let i = 1; i < bars.length; i++) {
      expect(bars[i] - bars[i - 1]).toBe(1);
    }

    // Same class of error must not hit the dimmed past-region-end overlay:
    // at max zoom-out the region spans >= MIN_REGION_VIEW_PX, so the
    // overlay (the ruler's only fillRect) never starts left of that.
    const rec = records.get(rulerCanvas(container));
    const overlays = (rec?.fillRect ?? []).filter((r) => r.w > 0);
    expect(overlays.length).toBeGreaterThan(0);
    const lastOverlay = overlays[overlays.length - 1];
    expect(lastOverlay.x).toBeGreaterThanOrEqual(MIN_REGION_VIEW_PX);
  });
});

describe("piano-roll header bar range agrees with the region's real span", () => {
  // The owner reported "Bars 1-3" over a region they counted as 4 bars.
  // These pin the header semantics: inclusive absolute bar numbers of the
  // bars the region OVERLAPS, derived from ticksPerBar(meta).

  async function headerFor(startTick: number, durationTicks: number) {
    const region = makeRegion("region-h", startTick, durationTicks);
    mockTracks([region]);
    const { container } = render(<PianoRoll {...rollProps(region)} />);
    await flush();
    return container.textContent ?? "";
  }

  it("bar-aligned region: 4 bars from bar 3 reads Bars 3-6", async () => {
    const text = await headerFor(2 * BAR, 4 * BAR);
    expect(text).toContain("Bars 3-6");
  });

  it("bar-aligned region at the song start: 4 bars reads Bars 1-4", async () => {
    const text = await headerFor(0, 4 * BAR);
    expect(text).toContain("Bars 1-4");
  });

  it("mid-bar region: 2 bars starting at a half bar overlaps bars 1-3", async () => {
    const text = await headerFor(BAR / 2, 2 * BAR);
    expect(text).toContain("Bars 1-3");
  });
});
