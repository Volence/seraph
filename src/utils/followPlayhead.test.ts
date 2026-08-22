import { describe, it, expect } from "vitest";
import {
  followScrollLeft,
  followAllowed,
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

  describe("backward playhead jumps never scroll (owner ruling)", () => {
    // A loop wrap or seek-back must not move the user's view. The function
    // is forward-only and position-based: a playhead behind the left edge
    // is always left alone, wherever it came from. (While a preview loop is
    // active, callers additionally suppress follow entirely — see
    // followAllowed.)
    it("leaves the view alone after a wrap behind the left edge", () => {
      // View parked at the loop end (scrollLeft 4000); playhead wrapped
      // back to content px 500. No scroll.
      expect(followScrollLeft(500, 4000, viewWidth)).toBeNull();
    });

    it("leaves the view alone after a wrap to the very start", () => {
      expect(followScrollLeft(0, 4000, viewWidth)).toBeNull();
    });
  });

  it("exposes sane constants", () => {
    expect(FOLLOW_EDGE_FRACTION).toBeGreaterThan(0);
    expect(FOLLOW_EDGE_FRACTION).toBeLessThan(1);
    expect(FOLLOW_REPOSITION_FRACTION).toBeLessThan(FOLLOW_EDGE_FRACTION);
    expect(FOLLOW_SUSPEND_MS).toBeGreaterThan(0);
  });
});

describe("followAllowed", () => {
  const now = 100_000;
  const longAgo = now - FOLLOW_SUSPEND_MS - 1;

  it("allows follow while playing with no loop and no recent manual scroll", () => {
    expect(followAllowed(true, false, longAgo, now)).toBe(true);
    // -Infinity is the components' "never scrolled" initial value.
    expect(followAllowed(true, false, -Infinity, now)).toBe(true);
  });

  it("suppresses follow entirely while a preview loop is active (owner ruling)", () => {
    // While looping, the view moves only by user input.
    expect(followAllowed(true, true, longAgo, now)).toBe(false);
    expect(followAllowed(true, true, -Infinity, now)).toBe(false);
  });

  it("suppresses follow while not playing", () => {
    expect(followAllowed(false, false, longAgo, now)).toBe(false);
  });

  it("suspends follow for FOLLOW_SUSPEND_MS after a manual scroll", () => {
    // Just inside the window: suspended. At/after the boundary: allowed.
    expect(followAllowed(true, false, now - FOLLOW_SUSPEND_MS + 1, now)).toBe(false);
    expect(followAllowed(true, false, now - FOLLOW_SUSPEND_MS, now)).toBe(true);
  });
});
