// Pure piano-roll editing helpers: marquee-selection hit-testing and
// keyboard transpose. Kept free of canvas/React so they are unit-testable.
import type { Note } from "../types/model";

/** Semitones in an octave — the Ctrl+Arrow transpose step. */
export const OCTAVE_SEMITONES = 12;

/**
 * Top of the velocity scale. Velocity is TL-DENOMINATED engine-wide: the
 * sequencer adds `(127 - volume) + (127 - velocity)` to every FM carrier TL
 * at ~0.75 dB/step (`src-tauri/src/sequencer/mod.rs`), so this value — and
 * only this value — means "no attenuation".
 */
export const MAX_VELOCITY = 127;

/**
 * Velocity every hand-placed note gets. DERIVED from `MAX_VELOCITY`, never
 * typed: a MIDI-convention 100 here once put every hand-placed FM note ~35 dB
 * under the audition the instrument was chosen with (fixed in `5e3b309`).
 * Every note-creating path in the piano roll must use this.
 */
export const DEFAULT_NOTE_VELOCITY = MAX_VELOCITY;

/** One grid-snapped cell of a paint gesture: a note-to-be. */
export interface PaintCell {
  tick: number;
  pitch: number;
}

/**
 * The grid cell a pointer position falls in: the tick floors to the grid
 * (same rule the double-click draw path uses), the pitch is already a row.
 */
export function paintCellAt(tick: number, pitch: number, gridSnapTicks: number): PaintCell {
  return { tick: Math.floor(tick / gridSnapTicks) * gridSnapTicks, pitch };
}

/**
 * True when a paint cell must NOT become a note. A cell is refused when it
 * starts at/after the region end, when an existing note on that pitch
 * overlaps its `[tick, tick + gridSnapTicks)` span, or when this gesture
 * already painted it (a fast drag re-enters the same cell constantly).
 *
 * Overlapping an existing note is refused rather than stacked because
 * one-channel overlaps resolve as last-note-priority in the driver: a
 * stacked duplicate would be inaudible but real, i.e. silent corruption.
 */
export function paintCellBlocked(
  cell: PaintCell,
  painted: readonly PaintCell[],
  notes: readonly Note[],
  gridSnapTicks: number,
  regionDurationTicks: number,
): boolean {
  if (cell.tick < 0 || cell.tick >= regionDurationTicks) return true;
  const end = cell.tick + gridSnapTicks;
  for (const p of painted) {
    if (p.pitch === cell.pitch && p.tick === cell.tick) return true;
  }
  for (const n of notes) {
    if (n.pitch !== cell.pitch) continue;
    if (n.tick < end && n.tick + n.durationTicks > cell.tick) return true;
  }
  return false;
}

/**
 * Playable pitch range per channel type. The piano roll's visible range (and
 * therefore its move/transpose clamp) starts from these and expands to cover
 * any out-of-range notes that arrive via import.
 */
export const PITCH_RANGES: Record<string, [number, number]> = {
  fm: [24, 106],
  psg: [33, 106],
  dac: [24, 72],
};

/** Fallback range for unknown channel types (matches PianoRoll's default). */
export const DEFAULT_PITCH_RANGE: [number, number] = [24, 95];

/** Marquee rectangle in musical space: ticks along x, pitches along y. */
export interface MarqueeRect {
  tickMin: number;
  tickMax: number;
  /** Inclusive pitch band. */
  pitchMin: number;
  pitchMax: number;
}

/**
 * Indices (ascending) of notes intersecting the rect. A note intersects when
 * its [tick, tick+duration) span overlaps (tickMin, tickMax) — boundary
 * touching does not count — and its pitch lies inside the inclusive band.
 */
export function notesIntersectingRect(notes: Note[], rect: MarqueeRect): number[] {
  const hits: number[] = [];
  for (let i = 0; i < notes.length; i++) {
    const n = notes[i];
    if (n.pitch < rect.pitchMin || n.pitch > rect.pitchMax) continue;
    if (n.tick + n.durationTicks <= rect.tickMin || n.tick >= rect.tickMax) continue;
    hits.push(i);
  }
  return hits;
}

/**
 * Convert a marquee drag in view coordinates into a musical-space rect.
 * Points may be given in any order (drags go in all four directions);
 * x is view px (scrollLeft-relative), y is canvas/content px. Rows are drawn
 * top-down from maxPitch, so the rect's top edge maps to the high pitch and
 * every row the rect touches is included.
 */
export function marqueeRectFromView(
  ax: number,
  ay: number,
  bx: number,
  by: number,
  scrollLeft: number,
  ticksPerPixel: number,
  maxPitch: number,
  rowHeight: number,
): MarqueeRect {
  const x1 = Math.min(ax, bx);
  const x2 = Math.max(ax, bx);
  const y1 = Math.min(ay, by);
  const y2 = Math.max(ay, by);
  return {
    tickMin: (x1 + scrollLeft) * ticksPerPixel,
    tickMax: (x2 + scrollLeft) * ticksPerPixel,
    pitchMin: maxPitch - Math.floor(y2 / rowHeight),
    pitchMax: maxPitch - Math.floor(y1 / rowHeight),
  };
}

/**
 * The set of note indices that should render highlighted while a marquee
 * drag is in flight — a live preview of what mouseup will select. Additive
 * (Shift) keeps the existing selection highlighted and adds the hits; plain
 * marquee previews a replacement, so only the hits highlight.
 */
export function marqueePreviewSelection(
  selected: ReadonlySet<number>,
  hits: readonly number[],
  additive: boolean,
): Set<number> {
  const preview = new Set(hits);
  if (additive) {
    for (const i of selected) preview.add(i);
  }
  return preview;
}

/**
 * Transpose every selected note by `delta` semitones. Returns the new pitch
 * per selected index, or `null` when the move is blocked: standard DAW
 * behavior blocks the whole move if ANY selected note would leave
 * [minPitch, maxPitch], so intervals between selected notes stay intact.
 * An empty selection is also `null` (nothing to move).
 */
/**
 * Nudge every selected note horizontally by `deltaTicks`. Returns the new
 * tick per selected index, or `null` when the move is blocked: matching the
 * transpose convention, the whole move is refused if ANY selected note would
 * cross tick 0 or its end would pass `regionDurationTicks`, so relative
 * rhythms between selected notes stay intact. Empty selection => `null`.
 */
export function nudgeNotes(
  notes: Note[],
  selected: Iterable<number>,
  deltaTicks: number,
  regionDurationTicks: number,
): { index: number; tick: number }[] | null {
  const indices = Array.from(selected).sort((a, b) => a - b);
  if (indices.length === 0) return null;
  const moves: { index: number; tick: number }[] = [];
  for (const index of indices) {
    const n = notes[index];
    if (!n) return null;
    const tick = n.tick + deltaTicks;
    if (tick < 0 || tick + n.durationTicks > regionDurationTicks) return null;
    moves.push({ index, tick });
  }
  return moves;
}

export function transposeNotes(
  notes: Note[],
  selected: Iterable<number>,
  delta: number,
  minPitch: number,
  maxPitch: number,
): { index: number; pitch: number }[] | null {
  const indices = Array.from(selected).sort((a, b) => a - b);
  if (indices.length === 0) return null;
  const moves: { index: number; pitch: number }[] = [];
  for (const index of indices) {
    const n = notes[index];
    if (!n) return null;
    const pitch = n.pitch + delta;
    if (pitch < minPitch || pitch > maxPitch) return null;
    moves.push({ index, pitch });
  }
  return moves;
}

/**
 * Zoom-out floor for the piano-roll view: however far the user zooms out,
 * the open region (or one bar, for sub-bar regions) always spans at least
 * this many pixels — a region editor never benefits from shrinking its own
 * region toward invisibility, and an unbounded zoom-out is exactly how the
 * ruler ended up drawing ~1281 one-pixel bars over a 4-bar region.
 */
export const MIN_REGION_VIEW_PX = 400;

/**
 * Largest allowed ticksPerPixel for the piano roll editing `durationTicks`.
 * Derived, never a constant: long regions may legitimately zoom far out
 * (their overview needs it), short regions may not.
 */
export function maxPianoRollTicksPerPixel(durationTicks: number, barTicks: number): number {
  return Math.max(durationTicks, barTicks) / MIN_REGION_VIEW_PX;
}
