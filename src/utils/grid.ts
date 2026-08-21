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
