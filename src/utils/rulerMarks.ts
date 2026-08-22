import { ticksPerBar, type GridMeta } from "./grid";

/**
 * Shared ruler-mark generation for the arrangement and piano-roll rulers.
 *
 * Marks are in ABSOLUTE song ticks with 1-based absolute bar numbers, so
 * both rulers agree with each other and with the piano roll's "Bars N-M"
 * header. All bar math derives from ticksPerBar(meta) — never hardcoded.
 */
export interface RulerMark {
  /** Absolute song tick of the mark. */
  tick: number;
  kind: "bar" | "beat";
  /** 1-based absolute bar number (kind "bar" only). */
  bar?: number;
  /** Whether to draw the bar number (kind "bar" only). */
  labeled?: boolean;
}

/** Labels stop overlapping below this many pixels per label. */
const MIN_LABEL_PX = 40;
/** Beat ticks disappear when beats get denser than this. */
const MIN_BEAT_PX = 8;

/**
 * Label every Nth bar, N the smallest power of two whose combined width
 * meets the minimum label spacing. Keeps bar 1 labeled at any zoom.
 */
export function barLabelStep(barWidthPx: number, minLabelPx = MIN_LABEL_PX): number {
  let step = 1;
  while (step * barWidthPx < minLabelPx && step < 1 << 15) {
    step *= 2;
  }
  return step;
}

/**
 * Bar (and, when zoom allows, beat) marks covering [startTick, endTick].
 * The first mark may fall before startTick (the enclosing bar boundary) so
 * callers can draw partially visible bars; marks never go below tick 0.
 */
export function rulerMarks(
  startTick: number,
  endTick: number,
  meta: GridMeta,
  ticksPerPixel: number,
): RulerMark[] {
  const bar = ticksPerBar(meta);
  const beat = meta.ticksPerBeat;
  const beatsPerBar = meta.timeSignature[0];

  const labelStep = barLabelStep(bar / ticksPerPixel);
  const showBeats = beat / ticksPerPixel >= MIN_BEAT_PX;

  const firstBar = Math.max(0, Math.floor(startTick / bar));
  const lastBar = Math.max(firstBar, Math.ceil(endTick / bar));

  const marks: RulerMark[] = [];
  for (let b = firstBar; b <= lastBar; b++) {
    marks.push({
      tick: b * bar,
      kind: "bar",
      bar: b + 1,
      labeled: b % labelStep === 0,
    });
    if (showBeats) {
      for (let i = 1; i < beatsPerBar; i++) {
        const t = b * bar + i * beat;
        if (t <= endTick) marks.push({ tick: t, kind: "beat" });
      }
    }
  }
  return marks;
}
