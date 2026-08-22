import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRollRuler } from "./PianoRollRuler";
import { ticksPerBar, type GridMeta } from "../utils/grid";

// jsdom's getBoundingClientRect() is all zeros; pin an 800x20 rect so
// clientX/clientY map 1:1 onto ruler space.
const RECT = {
  x: 0, y: 0, top: 0, left: 0, right: 800, bottom: 20,
  width: 800, height: 20,
  toJSON: () => {},
} as DOMRect;

// Non-4/4 so hardcoded 4-beats-per-bar math fails: bar = 1440.
const meta: GridMeta = { ticksPerBeat: 480, timeSignature: [3, 4] };
const BAR = ticksPerBar(meta);

const handlers = {
  onSeek: vi.fn(),
  onZoom: vi.fn(),
  onScrollChange: vi.fn(),
};

// Region opens at bar 3 (absolute tick 2*BAR), four bars long.
const props = {
  regionStartTick: BAR * 2,
  regionDurationTicks: BAR * 4,
  ticksPerPixel: 10,
  scrollLeft: 0,
  ticksPerBeat: meta.ticksPerBeat,
  beatsPerBar: meta.timeSignature[0],
};

function renderRuler() {
  const { container } = render(<PianoRollRuler {...props} {...handlers} />);
  const ruler = container.firstChild as HTMLElement;
  ruler.getBoundingClientRect = () => RECT;
  return ruler;
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("PianoRollRuler click-to-seek", () => {
  it("clicking seeks to the ABSOLUTE song tick under the pointer", () => {
    const ruler = renderRuler();

    // x=144 -> local tick 1440 (one bar into the region).
    fireEvent.mouseDown(ruler, { clientX: BAR / props.ticksPerPixel, clientY: 10 });
    fireEvent.mouseUp(window, { clientX: BAR / props.ticksPerPixel, clientY: 10 });

    expect(handlers.onSeek).toHaveBeenCalledWith(props.regionStartTick + BAR);
    expect(handlers.onZoom).not.toHaveBeenCalled();
    expect(handlers.onScrollChange).not.toHaveBeenCalled();
  });

  it("clicks past the region end clamp to the region end", () => {
    const ruler = renderRuler();

    // x=700 -> local tick 7000, past the 4-bar (5760) region.
    fireEvent.mouseDown(ruler, { clientX: 700, clientY: 10 });
    fireEvent.mouseUp(window, { clientX: 700, clientY: 10 });

    expect(handlers.onSeek).toHaveBeenCalledWith(props.regionStartTick + props.regionDurationTicks);
  });
});

describe("PianoRollRuler vertical-drag zoom", () => {
  it("dragging down zooms in (factor < 1) around the grab x", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 200, clientY: 10 });
    fireEvent.mouseMove(window, { clientX: 200, clientY: 60 });
    fireEvent.mouseUp(window, { clientX: 200, clientY: 60 });

    expect(handlers.onZoom).toHaveBeenCalled();
    const [anchorPx, factor] = handlers.onZoom.mock.calls[0];
    expect(anchorPx).toBe(200);
    expect(factor).toBeLessThan(1);
    expect(handlers.onSeek).not.toHaveBeenCalled();
    expect(handlers.onScrollChange).not.toHaveBeenCalled();
  });

  it("dragging up zooms out (factor > 1)", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 200, clientY: 10 });
    fireEvent.mouseMove(window, { clientX: 200, clientY: -40 });
    fireEvent.mouseUp(window, { clientX: 200, clientY: -40 });

    const [, factor] = handlers.onZoom.mock.calls[0];
    expect(factor).toBeGreaterThan(1);
  });
});

describe("PianoRollRuler horizontal-drag scroll", () => {
  it("dragging left scrolls the view right, clamped at zero", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 100, clientY: 10 });
    fireEvent.mouseMove(window, { clientX: 40, clientY: 10 });
    fireEvent.mouseUp(window, { clientX: 40, clientY: 10 });

    // startScroll 0 - delta (-60) = 60.
    expect(handlers.onScrollChange).toHaveBeenCalledWith(60);
    expect(handlers.onSeek).not.toHaveBeenCalled();
    expect(handlers.onZoom).not.toHaveBeenCalled();
  });
});
