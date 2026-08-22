import type { SongMetadata } from "../types/model";

/** The slice of project metadata the grid math needs. */
export type GridMeta = Pick<SongMetadata, "ticksPerBeat" | "timeSignature">;

/**
 * Ticks in one bar, derived from project metadata.
 *
 * The single seam for bar-length math (piano roll, arrangement zoom,
 * region-create defaults) — never hardcode 480 ticks or 4 beats per bar;
 * the future tick-native-grid work lands here.
 */
export function ticksPerBar(meta: GridMeta): number {
  return meta.ticksPerBeat * meta.timeSignature[0];
}

/** Arrangement snap granularity: bar, beat, or free (1-tick). */
export type SnapMode = "bar" | "beat" | "off";

/**
 * Tick length of one snap step for the given mode. "off" is a single tick
 * so callers can use one code path for snapped and free placement.
 */
export function snapUnit(meta: GridMeta, mode: SnapMode): number {
  switch (mode) {
    case "bar":
      return ticksPerBar(meta);
    case "beat":
      return meta.ticksPerBeat;
    case "off":
      return 1;
  }
}

/**
 * Floor `tick` to the enclosing snap step. The single seam for arrangement
 * snapping (region create/move/resize, ruler loop drag) — replaces the
 * per-component hardcoded snapToBar helpers. The piano roll keeps its own
 * grid selector.
 */
export function snapTick(tick: number, meta: GridMeta, mode: SnapMode): number {
  const unit = snapUnit(meta, mode);
  return Math.floor(tick / unit) * unit;
}
