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
 * Follow is strictly forward-only (owner ruling): a backward playhead jump
 * (loop wrap, seek-back) NEVER scrolls the view. Additionally, while a
 * preview loop is active follow is suppressed entirely — see followAllowed —
 * so a looping playback moves the view only by user input.
 */

/** Page forward once the playhead passes this fraction of the visible width. */
export const FOLLOW_EDGE_FRACTION = 0.8;
/** After paging, the playhead sits at this fraction of the visible width. */
export const FOLLOW_REPOSITION_FRACTION = 0.1;
/** Manual scrolling suspends following for this long (ms). */
export const FOLLOW_SUSPEND_MS = 2000;

/**
 * Whether follow-playhead may act at all right now. Shared by both views so
 * the gating rules can't drift:
 * - only while playing;
 * - NEVER while a preview loop is active (owner ruling: a looping playback
 *   must not move the user's view — they are editing in place);
 * - not within FOLLOW_SUSPEND_MS of a manual scroll.
 */
export function followAllowed(
  playing: boolean,
  loopActive: boolean,
  lastManualScrollAt: number,
  now: number,
): boolean {
  if (!playing || loopActive) return false;
  return now - lastManualScrollAt >= FOLLOW_SUSPEND_MS;
}

/**
 * Returns the new scrollLeft when the view should page forward to follow the
 * playhead, or null to leave the scroll alone. Forward-only: a playhead at
 * or behind the follow edge — including one that just jumped backward on a
 * loop wrap or seek — never moves the view.
 *
 * @param playheadContentPx playhead position in content px (tick / ticksPerPixel)
 * @param scrollLeft current scroll offset in content px
 * @param viewWidth visible width in px (0 or negative disables following)
 */
export function followScrollLeft(
  playheadContentPx: number,
  scrollLeft: number,
  viewWidth: number,
): number | null {
  if (viewWidth <= 0) return null;
  const viewX = playheadContentPx - scrollLeft;
  if (viewX <= viewWidth * FOLLOW_EDGE_FRACTION) return null;
  return Math.max(0, playheadContentPx - viewWidth * FOLLOW_REPOSITION_FRACTION);
}
