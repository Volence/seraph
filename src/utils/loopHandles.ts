import { snapUnit, type GridMeta, type SnapMode } from "./grid";
import type { PreviewLoopRange } from "./previewLoop";

/**
 * Hit-testing and edit math for the ruler's loop bracket: grabbable edge
 * handles (resize) and a draggable body (move). Pixel-space hit tests,
 * tick-space edits; snapping goes through the shared arrangement snap.
 */

/** Half-width of each edge handle's grab zone, in pixels. */
export const LOOP_HANDLE_PX = 6;

export type LoopZone = "start" | "end" | "body" | null;

/**
 * Which part of the drawn loop band (start/end in view pixels) the pointer
 * is over. When the band is narrower than the two handle zones, the nearer
 * edge wins so both stay reachable.
 */
export function loopHitZone(xPx: number, startPx: number, endPx: number, handlePx = LOOP_HANDLE_PX): LoopZone {
  if (xPx < startPx - handlePx || xPx > endPx + handlePx) return null;
  const dStart = Math.abs(xPx - startPx);
  const dEnd = Math.abs(xPx - endPx);
  const nearStart = dStart <= handlePx;
  const nearEnd = dEnd <= handlePx;
  if (nearStart && nearEnd) return dStart <= dEnd ? "start" : "end";
  if (nearStart) return "start";
  if (nearEnd) return "end";
  return "body";
}

/**
 * Drag one edge of the loop to `tick`: the moved edge rounds to the snap
 * unit, the other edge stays, and the range never collapses below one
 * unit. Start clamps at 0.
 */
export function resizePreviewLoop(
  loop: PreviewLoopRange,
  edge: "start" | "end",
  tick: number,
  meta: GridMeta,
  mode: SnapMode,
): PreviewLoopRange {
  const unit = snapUnit(meta, mode);
  const snapped = Math.round(tick / unit) * unit;
  if (edge === "start") {
    const start = Math.min(Math.max(0, snapped), loop.end - unit);
    return { start, end: loop.end };
  }
  const end = Math.max(snapped, loop.start + unit);
  return { start: loop.start, end };
}

/**
 * Drag the whole loop by `deltaTick`: the new start rounds to the snap
 * unit and clamps at 0; the length is preserved exactly.
 */
export function movePreviewLoop(
  loop: PreviewLoopRange,
  deltaTick: number,
  meta: GridMeta,
  mode: SnapMode,
): PreviewLoopRange {
  const unit = snapUnit(meta, mode);
  const length = loop.end - loop.start;
  const start = Math.max(0, Math.round((loop.start + deltaTick) / unit) * unit);
  return { start, end: start + length };
}
