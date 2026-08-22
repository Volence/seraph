import { describe, it, expect } from "vitest";
import {
  zoomAroundPixel,
  dragZoomFactor,
  resolveDragAxis,
  AXIS_LOCK_SLOP_PX,
} from "./zoomDrag";

describe("zoomAroundPixel", () => {
  it("keeps the tick under the anchor pixel fixed across the zoom", () => {
    const view = { ticksPerPixel: 10, scrollLeft: 200 };
    const anchorPx = 150;
    const tickBefore = (anchorPx + view.scrollLeft) * view.ticksPerPixel;

    const next = zoomAroundPixel(view, anchorPx, 5);
    const tickAfter = (anchorPx + next.scrollLeft) * next.ticksPerPixel;

    expect(next.ticksPerPixel).toBe(5);
    expect(tickAfter).toBeCloseTo(tickBefore, 6);
  });

  it("clamps scrollLeft at zero when zooming out near the origin", () => {
    const view = { ticksPerPixel: 10, scrollLeft: 5 };
    const next = zoomAroundPixel(view, 10, 100);
    expect(next.scrollLeft).toBe(0);
  });
});

describe("dragZoomFactor", () => {
  it("drag down (positive dy) zooms in: factor shrinks ticksPerPixel", () => {
    expect(dragZoomFactor(50)).toBeLessThan(1);
  });

  it("drag up (negative dy) zooms out: factor grows ticksPerPixel", () => {
    expect(dragZoomFactor(-50)).toBeGreaterThan(1);
  });

  it("no movement is identity", () => {
    expect(dragZoomFactor(0)).toBe(1);
  });

  it("composes: incremental deltas multiply to the same total", () => {
    expect(dragZoomFactor(30) * dragZoomFactor(20)).toBeCloseTo(dragZoomFactor(50), 10);
  });
});

describe("resolveDragAxis", () => {
  it("stays undecided inside the slop radius", () => {
    expect(resolveDragAxis(AXIS_LOCK_SLOP_PX, 0)).toBe("none");
    expect(resolveDragAxis(0, -AXIS_LOCK_SLOP_PX)).toBe("none");
    expect(resolveDragAxis(2, 3)).toBe("none");
  });

  it("locks to the dominant axis once past the slop", () => {
    expect(resolveDragAxis(10, 3)).toBe("horizontal");
    expect(resolveDragAxis(-10, 3)).toBe("horizontal");
    expect(resolveDragAxis(3, 10)).toBe("vertical");
    expect(resolveDragAxis(3, -10)).toBe("vertical");
  });

  it("ties go horizontal (loop/scroll gestures keep priority)", () => {
    expect(resolveDragAxis(10, 10)).toBe("horizontal");
  });
});
