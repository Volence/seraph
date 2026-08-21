// Pure piano-roll editing helpers: marquee-selection hit-testing and
// keyboard transpose. Kept free of canvas/React so they are unit-testable.
import type { Note } from "../types/model";

/** Semitones in an octave — the Ctrl+Arrow transpose step. */
export const OCTAVE_SEMITONES = 12;

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
 * Transpose every selected note by `delta` semitones. Returns the new pitch
 * per selected index, or `null` when the move is blocked: standard DAW
 * behavior blocks the whole move if ANY selected note would leave
 * [minPitch, maxPitch], so intervals between selected notes stay intact.
 * An empty selection is also `null` (nothing to move).
 */
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
