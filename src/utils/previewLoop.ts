import { snapUnit, ticksPerBar, type GridMeta, type SnapMode } from "./grid";

/**
 * The transport preview loop: a bar-snapped play range set from the ruler.
 *
 * Transport-scratch state only — deliberately NOT saved into the project.
 * S4's SongSettingsPanel will own the persisted song-level loop; this is the
 * quick audition loop the L key / loop button toggles.
 */
export interface PreviewLoopRange {
  start: number;
  end: number;
}

/**
 * The range the loop toggle uses when the user never dragged one: one bar
 * at the bar containing the seek cursor.
 */
export function defaultPreviewLoop(seekTick: number, meta: GridMeta): PreviewLoopRange {
  const bar = ticksPerBar(meta);
  const start = Math.floor(Math.max(0, seekTick) / bar) * bar;
  return { start, end: start + bar };
}

/**
 * Live range for a ruler loop drag: the low edge floors to the snap step,
 * the high edge ceils to it, and the range always spans at least one step
 * (one bar in the default snap mode). Direction-agnostic.
 */
export function dragPreviewLoop(
  anchorTick: number,
  currentTick: number,
  meta: GridMeta,
  mode: SnapMode,
): PreviewLoopRange {
  const unit = snapUnit(meta, mode);
  const lo = Math.max(0, Math.min(anchorTick, currentTick));
  const hi = Math.max(0, anchorTick, currentTick);
  const start = Math.floor(lo / unit) * unit;
  const end = Math.max(Math.ceil(hi / unit) * unit, start + unit);
  return { start, end };
}
