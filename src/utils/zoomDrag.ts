/**
 * Vertical-drag zoom math shared by the arrangement and piano-roll rulers.
 *
 * Grammar (FL Studio convention): grab a ruler and drag DOWN to zoom in,
 * UP to zoom out, centered on the grab position. Vertical and horizontal
 * drags are dominant-axis locked — the first axis to leave the slop radius
 * owns the whole gesture — so a loop paint never re-scales under the
 * cursor mid-drag.
 */

export interface ZoomView {
  ticksPerPixel: number;
  scrollLeft: number; // pixels
}

/** Movement inside this radius is still a click; past it, the drag locks. */
export const AXIS_LOCK_SLOP_PX = 4;

export type DragAxis = "none" | "horizontal" | "vertical";

/** Zoom sensitivity: ~1% scale change per pixel of vertical drag. */
const ZOOM_PER_PIXEL = 1.01;

/**
 * Re-scale the view to `newTicksPerPixel`, adjusting scrollLeft so the
 * tick under `anchorPx` (view-relative pixels) stays put. scrollLeft is
 * clamped at 0, matching every other scroll path.
 */
export function zoomAroundPixel(view: ZoomView, anchorPx: number, newTicksPerPixel: number): ZoomView {
  const anchorTick = (anchorPx + view.scrollLeft) * view.ticksPerPixel;
  const scrollLeft = Math.max(0, anchorTick / newTicksPerPixel - anchorPx);
  return { ticksPerPixel: newTicksPerPixel, scrollLeft };
}

/**
 * Multiplier for ticksPerPixel from a vertical drag increment. Positive dy
 * (downward) shrinks ticksPerPixel (zoom in). Multiplicative, so applying
 * per-mousemove increments composes to the same zoom as one big delta.
 */
export function dragZoomFactor(dyPx: number): number {
  return Math.pow(ZOOM_PER_PIXEL, -dyPx);
}

/**
 * Dominant-axis lock: "none" until the pointer leaves the slop radius,
 * then whichever axis moved farther. Ties go horizontal so the existing
 * loop/scroll gestures keep priority on diagonal grabs.
 */
export function resolveDragAxis(dx: number, dy: number, slop = AXIS_LOCK_SLOP_PX): DragAxis {
  const ax = Math.abs(dx);
  const ay = Math.abs(dy);
  if (Math.max(ax, ay) <= slop) return "none";
  return ay > ax ? "vertical" : "horizontal";
}
