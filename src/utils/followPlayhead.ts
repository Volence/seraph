/**
 * Follow-playhead auto-scroll (G28/G22).
 *
 * Both the arrangement and the piano roll scroll in content-space pixels
 * (tick / ticksPerPixel). While playing, once the playhead crosses
 * FOLLOW_EDGE_FRACTION of the visible width the view pages forward so the
 * playhead lands at FOLLOW_REPOSITION_FRACTION of the width. A view the user
 * has scrolled ahead of the playhead is left alone (the playhead is behind
 * the left edge), and callers suspend following for FOLLOW_SUSPEND_MS after
 * any manual scroll so the user can inspect other bars during playback.
 *
 * Backward jumps (loop wrap, seek-back) are the exception to "behind the
 * left edge means leave it alone": when the caller passes the previous
 * playhead position and the playhead moved BACKWARD off-screen, the view
 * snaps so the playhead sits at the reposition anchor again — otherwise a
 * looping playback strands the view at the loop end every cycle.
 */

/** Page forward once the playhead passes this fraction of the visible width. */
export const FOLLOW_EDGE_FRACTION = 0.8;
/** After paging, the playhead sits at this fraction of the visible width. */
export const FOLLOW_REPOSITION_FRACTION = 0.1;
/** Manual scrolling suspends following for this long (ms). */
export const FOLLOW_SUSPEND_MS = 2000;

/**
 * Returns the new scrollLeft when the view should page forward to follow the
 * playhead, or null to leave the scroll alone.
 *
 * @param playheadContentPx playhead position in content px (tick / ticksPerPixel)
 * @param scrollLeft current scroll offset in content px
 * @param viewWidth visible width in px (0 or negative disables following)
 * @param prevPlayheadContentPx previous playhead position in content px, in
 *   the SAME scale as playheadContentPx (convert both with the current
 *   ticksPerPixel). null/omitted = direction unknown, forward-only behavior.
 */
export function followScrollLeft(
  playheadContentPx: number,
  scrollLeft: number,
  viewWidth: number,
  prevPlayheadContentPx: number | null = null,
): number | null {
  if (viewWidth <= 0) return null;
  const viewX = playheadContentPx - scrollLeft;
  if (prevPlayheadContentPx !== null && playheadContentPx < prevPlayheadContentPx) {
    // Backward jump (loop wrap / seek-back). If the playhead is still
    // visible, leave the view alone; if it left the view, snap it back to
    // the reposition anchor so the user keeps their place across the wrap.
    // Exception: a playhead that was ALREADY behind the view before the
    // jump means the user scrolled ahead — small backward corrections from
    // interpolation jank must not yank the view away from them.
    if (prevPlayheadContentPx - scrollLeft < 0) return null;
    if (viewX >= 0 && viewX <= viewWidth) return null;
    return Math.max(0, playheadContentPx - viewWidth * FOLLOW_REPOSITION_FRACTION);
  }
  if (viewX <= viewWidth * FOLLOW_EDGE_FRACTION) return null;
  return Math.max(0, playheadContentPx - viewWidth * FOLLOW_REPOSITION_FRACTION);
}
