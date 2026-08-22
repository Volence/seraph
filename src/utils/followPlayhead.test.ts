import { describe, it, expect } from "vitest";
import {
  followScrollLeft,
  FOLLOW_EDGE_FRACTION,
  FOLLOW_REPOSITION_FRACTION,
  FOLLOW_SUSPEND_MS,
} from "./followPlayhead";

// All positions are content-space pixels; scrollLeft is the app's px-based
// scroll offset (see useArrangementZoom / PianoRoll pianoScrollLeft).

describe("followScrollLeft", () => {
  const viewWidth = 1000;

  it("does nothing while the playhead is inside the follow edge", () => {
    // Exactly at the edge is still inside.
    const atEdge = viewWidth * FOLLOW_EDGE_FRACTION;
    expect(followScrollLeft(atEdge, 0, viewWidth)).toBeNull();
    expect(followScrollLeft(100, 0, viewWidth)).toBeNull();
  });

  it("pages forward when the playhead crosses the follow edge", () => {
    const playheadPx = viewWidth * FOLLOW_EDGE_FRACTION + 1;
    const next = followScrollLeft(playheadPx, 0, viewWidth);
    // Playhead lands at the reposition fraction of the view.
    expect(next).toBe(playheadPx - viewWidth * FOLLOW_REPOSITION_FRACTION);
  });

  it("accounts for the current scroll offset", () => {
    // Playhead at content px 5000 with the view scrolled to 4500: view x is
    // 500, inside the edge -> no scroll.
    expect(followScrollLeft(5000, 4500, viewWidth)).toBeNull();
    // View scrolled to 4000: view x 1000 > 800 -> page forward.
    expect(followScrollLeft(5000, 4000, viewWidth)).toBe(5000 - viewWidth * FOLLOW_REPOSITION_FRACTION);
  });

  it("does not yank the view when the playhead is behind it (user scrolled ahead)", () => {
    // Playhead far left of the view: leave the user alone.
    expect(followScrollLeft(100, 4000, viewWidth)).toBeNull();
  });

  it("never returns a negative scroll and ignores degenerate widths", () => {
    // Clamp is defensive: reachable only with a (bogus) negative scrollLeft.
    expect(followScrollLeft(0.05, -1, 1)).toBe(0);
    expect(followScrollLeft(500, 0, 0)).toBeNull();
    expect(followScrollLeft(500, 0, -5)).toBeNull();
  });

  describe("backward playhead jumps (loop wrap / seek-back)", () => {
    it("snaps the view back when the playhead wraps behind the left edge", () => {
      // Loop wrap: view is at the loop end (scrollLeft 4000), the playhead
      // jumps from content px 4900 back to the loop start at 500. The view
      // must page back so the playhead sits at the reposition anchor.
      const next = followScrollLeft(500, 4000, viewWidth, 4900);
      expect(next).toBe(Math.max(0, 500 - viewWidth * FOLLOW_REPOSITION_FRACTION));
    });

    it("clamps the wrap target at zero for early loop starts", () => {
      // Loop start near tick 0: reposition anchor would be negative.
      expect(followScrollLeft(20, 4000, viewWidth, 4900)).toBe(0);
    });

    it("snaps when the playhead jumps backward but lands right of the view", () => {
      // User is inspecting early bars (scrollLeft 0) while the loop runs in
      // bars far to the right: wrap from 9000 to 5000 is still off-screen.
      const next = followScrollLeft(5000, 0, viewWidth, 9000);
      expect(next).toBe(5000 - viewWidth * FOLLOW_REPOSITION_FRACTION);
    });

    it("leaves the view alone when a backward jump stays visible", () => {
      // Wrap landed inside the current view: nothing to fix.
      expect(followScrollLeft(4200, 4000, viewWidth, 4900)).toBeNull();
    });

    it("ignores backward jitter while the playhead is already behind the view", () => {
      // The user scrolled ahead (playhead behind the left edge). A small
      // backward correction from interpolation jank must not yank the view:
      // only a jump from a visible/ahead playhead counts as a wrap.
      expect(followScrollLeft(3000, 4000, viewWidth, 3050)).toBeNull();
    });

    it("keeps the scrolled-ahead behavior for forward motion", () => {
      // Playhead moving forward while behind the view (user scrolled ahead):
      // still left alone, exactly as without the prev argument.
      expect(followScrollLeft(110, 4000, viewWidth, 100)).toBeNull();
    });

    it("ignores degenerate widths on backward jumps too", () => {
      expect(followScrollLeft(500, 4000, 0, 4900)).toBeNull();
    });
  });

  it("exposes sane constants", () => {
    expect(FOLLOW_EDGE_FRACTION).toBeGreaterThan(0);
    expect(FOLLOW_EDGE_FRACTION).toBeLessThan(1);
    expect(FOLLOW_REPOSITION_FRACTION).toBeLessThan(FOLLOW_EDGE_FRACTION);
    expect(FOLLOW_SUSPEND_MS).toBeGreaterThan(0);
  });
});
