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
};

// Non-4/4 so hardcoded 4-beats-per-bar snapping would fail: bar = 1440.
const props = {
  ticksPerPixel: 10,
  scrollLeft: 0,
  ticksPerBeat: 480,
  beatsPerBar: 3,
  loop: null,
  loopEnabled: false,
  snapMode: "bar" as SnapMode,
};

function renderRuler() {
  const { container } = render(<TimelineRuler {...props} {...handlers} />);
  const ruler = container.firstChild as HTMLElement;
  ruler.getBoundingClientRect = () => RECT;
  const canvas = ruler.querySelector("canvas") as HTMLCanvasElement;
  canvas.getBoundingClientRect = () => RECT;
  return ruler;
}

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
