import { describe, it, expect, vi, beforeEach } from "vitest";
import { render } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { TimelineRuler } from "./TimelineRuler";
import type { SnapMode } from "../utils/grid";

// jsdom's getBoundingClientRect() is all zeros, which would make the
// upper/lower-half split untestable — so both the container and the canvas
// get a fixed 800x30 rect (clientX/clientY then map 1:1 onto ruler space).
const RECT = {
  x: 0, y: 0, top: 0, left: 0, right: 800, bottom: 30,
  width: 800, height: 30,
  toJSON: () => {},
} as DOMRect;

const handlers = {
  onSeek: vi.fn(),
  onScrollChange: vi.fn(),
  onLoopDrag: vi.fn(),
  onZoom: vi.fn(),
};

// Non-4/4 so hardcoded 4-beats-per-bar snapping would fail: bar = 1440.
const props = {
  ticksPerPixel: 10,
  scrollLeft: 0,
  ticksPerBeat: 480,
  beatsPerBar: 3,
  loop: null as { start: number; end: number } | null,
  loopEnabled: false,
  snapMode: "bar" as SnapMode,
};

function renderRuler(overrides: Partial<typeof props> = {}) {
  const { container } = render(<TimelineRuler {...props} {...overrides} {...handlers} />);
  const ruler = container.firstChild as HTMLElement;
  ruler.getBoundingClientRect = () => RECT;
  const canvas = ruler.querySelector("canvas") as HTMLCanvasElement;
  canvas.getBoundingClientRect = () => RECT;
  return ruler;
}

// Derived, not hardcoded: bar length from the non-4/4 test meta.
const BAR = props.ticksPerBeat * props.beatsPerBar;

beforeEach(() => {
  vi.clearAllMocks();
});

describe("TimelineRuler upper-half loop drag", () => {
  it("drag across the upper half commits a bar-snapped loop range", () => {
    const ruler = renderRuler();

    // x=150 → tick 1500 (bar 2), drag to x=300 → tick 3000 (bar 3).
    fireEvent.mouseDown(ruler, { clientX: 150, clientY: 5 });
    fireEvent.mouseMove(window, { clientX: 300, clientY: 5 });
    fireEvent.mouseUp(window, { clientX: 300, clientY: 5 });

    // Bars 2-3 enclosed: [1440, 4320) with bar = 480 * 3.
    expect(handlers.onLoopDrag).toHaveBeenCalledWith(1440, 4320);
    expect(handlers.onScrollChange).not.toHaveBeenCalled();
    expect(handlers.onSeek).not.toHaveBeenCalled();
  });

  it("a plain click on the upper half sets a one-bar loop at that bar", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 150, clientY: 5 });
    fireEvent.mouseUp(window, { clientX: 150, clientY: 5 });

    expect(handlers.onLoopDrag).toHaveBeenCalledWith(1440, 2880);
    expect(handlers.onSeek).not.toHaveBeenCalled();
  });
});

describe("TimelineRuler lower half keeps scroll and seek", () => {
  it("drag on the lower half scrolls", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 100, clientY: 25 });
    fireEvent.mouseMove(window, { clientX: 50, clientY: 25 });
    fireEvent.mouseUp(window, { clientX: 50, clientY: 25 });

    // startScroll 0 - delta (-50) = 50.
    expect(handlers.onScrollChange).toHaveBeenCalledWith(50);
    expect(handlers.onSeek).not.toHaveBeenCalled();
    expect(handlers.onLoopDrag).not.toHaveBeenCalled();
  });

  it("click on the lower half seeks", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 100, clientY: 25 });
    fireEvent.mouseUp(window, { clientX: 100, clientY: 25 });

    expect(handlers.onSeek).toHaveBeenCalledWith(1000);
    expect(handlers.onLoopDrag).not.toHaveBeenCalled();
  });
});

describe("TimelineRuler vertical-drag zoom", () => {
  it("dragging down zooms in (factor < 1) around the grab x", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 200, clientY: 25 });
    fireEvent.mouseMove(window, { clientX: 200, clientY: 75 });
    fireEvent.mouseUp(window, { clientX: 200, clientY: 75 });

    expect(handlers.onZoom).toHaveBeenCalled();
    const [anchorPx, factor] = handlers.onZoom.mock.calls[0];
    expect(anchorPx).toBe(200);
    expect(factor).toBeLessThan(1);
    // A vertical gesture never scrolls, seeks, or sets a loop.
    expect(handlers.onScrollChange).not.toHaveBeenCalled();
    expect(handlers.onSeek).not.toHaveBeenCalled();
    expect(handlers.onLoopDrag).not.toHaveBeenCalled();
  });

  it("dragging up from the upper half zooms out without painting a loop", () => {
    const ruler = renderRuler();

    fireEvent.mouseDown(ruler, { clientX: 300, clientY: 10 });
    fireEvent.mouseMove(window, { clientX: 300, clientY: -30 });
    fireEvent.mouseUp(window, { clientX: 300, clientY: -30 });

    expect(handlers.onZoom).toHaveBeenCalled();
    const [, factor] = handlers.onZoom.mock.calls[0];
    expect(factor).toBeGreaterThan(1);
    expect(handlers.onLoopDrag).not.toHaveBeenCalled();
  });
});

describe("TimelineRuler loop handles", () => {
  // Band bars 2-4 -> pixels [BAR/10, 3*BAR/10] at ticksPerPixel 10.
  const loopProps = { loop: { start: BAR, end: BAR * 3 }, loopEnabled: true };
  const startPx = BAR / props.ticksPerPixel;
  const endPx = (BAR * 3) / props.ticksPerPixel;

  it("dragging the end handle resizes the loop, snapped to the bar", () => {
    const ruler = renderRuler(loopProps);

    fireEvent.mouseDown(ruler, { clientX: endPx, clientY: 5 });
    // Drag right by one bar's width -> end lands on bar 5's boundary.
    fireEvent.mouseMove(window, { clientX: endPx + BAR / props.ticksPerPixel, clientY: 5 });
    fireEvent.mouseUp(window, { clientX: endPx + BAR / props.ticksPerPixel, clientY: 5 });

    expect(handlers.onLoopDrag).toHaveBeenCalledWith(BAR, BAR * 4);
  });

  it("dragging the start handle resizes the other edge", () => {
    const ruler = renderRuler(loopProps);

    fireEvent.mouseDown(ruler, { clientX: startPx, clientY: 5 });
    fireEvent.mouseMove(window, { clientX: startPx + BAR / props.ticksPerPixel, clientY: 5 });
    fireEvent.mouseUp(window, { clientX: startPx + BAR / props.ticksPerPixel, clientY: 5 });

    expect(handlers.onLoopDrag).toHaveBeenCalledWith(BAR * 2, BAR * 3);
  });

  it("dragging the body moves the whole range, length preserved", () => {
    const ruler = renderRuler(loopProps);
    const bodyPx = (startPx + endPx) / 2;

    fireEvent.mouseDown(ruler, { clientX: bodyPx, clientY: 5 });
    fireEvent.mouseMove(window, { clientX: bodyPx + BAR / props.ticksPerPixel, clientY: 5 });
    fireEvent.mouseUp(window, { clientX: bodyPx + BAR / props.ticksPerPixel, clientY: 5 });

    expect(handlers.onLoopDrag).toHaveBeenCalledWith(BAR * 2, BAR * 4);
  });

  it("a plain click on the loop body leaves the loop alone", () => {
    const ruler = renderRuler(loopProps);
    const bodyPx = (startPx + endPx) / 2;

    fireEvent.mouseDown(ruler, { clientX: bodyPx, clientY: 5 });
    fireEvent.mouseUp(window, { clientX: bodyPx, clientY: 5 });

    expect(handlers.onLoopDrag).not.toHaveBeenCalled();
    expect(handlers.onSeek).not.toHaveBeenCalled();
  });

  it("upper-half clicks outside the band still set a one-bar loop", () => {
    const ruler = renderRuler(loopProps);
    // Well right of the band + handle tolerance: bar 5's territory.
    const x = (BAR * 4.5) / props.ticksPerPixel;

    fireEvent.mouseDown(ruler, { clientX: x, clientY: 5 });
    fireEvent.mouseUp(window, { clientX: x, clientY: 5 });

    expect(handlers.onLoopDrag).toHaveBeenCalledWith(BAR * 4, BAR * 5);
  });
});
