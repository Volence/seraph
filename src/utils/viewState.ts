/**
 * Per-project VIEW state: where the user was sitting when they last had this
 * project open — arrangement zoom/scroll, arrangement snap, the piano-roll
 * grid selector, the bottom-panel layout, the preview loop, and which region
 * was open in the roll (audit F15).
 *
 * WHY localStorage AND NOT THE PROJECT FILE. This is machine-local, per-user
 * state, not part of the document: putting it in `ProjectFile` would change
 * the serialized format (compat risk plus `src/bindings.ts` churn) and would
 * make every scroll wheel tick mark the project dirty. It follows the shape
 * `recentLocations.ts` established — one versioned key, a small typed
 * get/set module, every failure swallowed.
 *
 * WHY EVERY READ IS SANITIZED. A project can be edited by another session,
 * another machine, or by hand between two sittings. A stored region id, a
 * stored loop range, a stored zoom level may name something that no longer
 * exists or sits outside the range the live UI allows. The rule here is:
 * restore what still validates, silently drop what does not, and never throw
 * into a render path. `sanitizeViewState` enforces the SHAPE; the `resolve*`
 * helpers enforce agreement with the project as it is right now.
 */

import type { SelectedRegion, Track } from "../types/model";
import { channelTypeOf } from "../types/model";
import { normalizePath } from "./recentLocations";
import type { SnapMode } from "./grid";

/** localStorage key. Versioned so a future shape change can migrate cleanly. */
export const VIEW_STATE_KEY = "seraph.projectViewState.v1";

/** How many projects keep a remembered view; oldest are dropped. */
export const MAX_VIEW_STATE_PROJECTS = 16;

/**
 * Trailing-edge delay on write-through. Scroll and zoom change on every wheel
 * event; a `setTimeout` inside the writing effect coalesces a whole gesture
 * into one `localStorage.setItem`. The cost is that a change made in the last
 * `VIEW_STATE_WRITE_DELAY_MS` before the app quits is lost — acceptable for a
 * convenience, and the reason this is a constant rather than a literal.
 */
export const VIEW_STATE_WRITE_DELAY_MS = 250;

/** Which region was open in the piano roll, by identity only. */
export interface OpenRegionRef {
  trackId: string;
  regionId: string;
}

/** The transport preview loop, plus whether it was armed. */
export interface LoopViewState {
  start: number;
  end: number;
  enabled: boolean;
}

export interface ArrangementViewState {
  ticksPerPixel: number;
  scrollLeft: number;
  snapMode: SnapMode;
  /** Channel-group labels ("FM1", "PSG2", …) the user collapsed. */
  collapsedChannels: string[];
}

export interface PanelViewState {
  collapsed: boolean;
  height: number;
}

export interface PianoRollViewState {
  /** Index into the roll's GRID_OPTIONS list. */
  gridIdx: number;
}

/** Everything remembered for one project. Every field is optional: a record
 *  written by an older build simply restores less. */
export interface ProjectViewState {
  arrangement?: Partial<ArrangementViewState>;
  panel?: Partial<PanelViewState>;
  pianoRoll?: Partial<PianoRollViewState>;
  loop?: LoopViewState | null;
  openRegion?: OpenRegionRef | null;
}

interface StoredEntry {
  path: string;
  state: ProjectViewState;
}

const SNAP_MODES: readonly SnapMode[] = ["bar", "beat", "off"];

/** A finite number, or undefined. Rejects NaN/Infinity/strings/null. */
function finiteNumber(v: unknown): number | undefined {
  return typeof v === "number" && Number.isFinite(v) ? v : undefined;
}

function boolean(v: unknown): boolean | undefined {
  return typeof v === "boolean" ? v : undefined;
}

function nonEmptyString(v: unknown): string | undefined {
  return typeof v === "string" && v !== "" ? v : undefined;
}

/** Drop `key` entirely when `value` is undefined, so a partially-corrupt
 *  record restores its intact fields instead of storing `undefined`s. */
function put<T extends object, K extends keyof T>(target: T, key: K, value: T[K] | undefined): void {
  if (value !== undefined) target[key] = value;
}

/**
 * Coerce an arbitrary parsed value into a ProjectViewState, keeping only
 * fields that are the right TYPE. Range/existence checks belong to the
 * `resolve*` / `clamp*` helpers, which need live project data.
 */
export function sanitizeViewState(raw: unknown): ProjectViewState {
  const out: ProjectViewState = {};
  if (typeof raw !== "object" || raw === null) return out;
  const r = raw as Record<string, unknown>;

  if (typeof r.arrangement === "object" && r.arrangement !== null) {
    const a = r.arrangement as Record<string, unknown>;
    const arrangement: Partial<ArrangementViewState> = {};
    put(arrangement, "ticksPerPixel", finiteNumber(a.ticksPerPixel));
    put(arrangement, "scrollLeft", finiteNumber(a.scrollLeft));
    put(
      arrangement,
      "snapMode",
      SNAP_MODES.includes(a.snapMode as SnapMode) ? (a.snapMode as SnapMode) : undefined,
    );
    if (Array.isArray(a.collapsedChannels)) {
      arrangement.collapsedChannels = a.collapsedChannels.filter(
        (c): c is string => typeof c === "string" && c !== "",
      );
    }
    if (Object.keys(arrangement).length > 0) out.arrangement = arrangement;
  }

  if (typeof r.panel === "object" && r.panel !== null) {
    const p = r.panel as Record<string, unknown>;
    const panel: Partial<PanelViewState> = {};
    put(panel, "collapsed", boolean(p.collapsed));
    put(panel, "height", finiteNumber(p.height));
    if (Object.keys(panel).length > 0) out.panel = panel;
  }

  if (typeof r.pianoRoll === "object" && r.pianoRoll !== null) {
    const pr = r.pianoRoll as Record<string, unknown>;
    const pianoRoll: Partial<PianoRollViewState> = {};
    put(pianoRoll, "gridIdx", finiteNumber(pr.gridIdx));
    if (Object.keys(pianoRoll).length > 0) out.pianoRoll = pianoRoll;
  }

  if (typeof r.loop === "object" && r.loop !== null) {
    const l = r.loop as Record<string, unknown>;
    const start = finiteNumber(l.start);
    const end = finiteNumber(l.end);
    const enabled = boolean(l.enabled);
    // A loop is all-or-nothing: half a range is not a range.
    if (start !== undefined && end !== undefined && enabled !== undefined) {
      out.loop = { start, end, enabled };
    }
  }

  if (typeof r.openRegion === "object" && r.openRegion !== null) {
    const o = r.openRegion as Record<string, unknown>;
    const trackId = nonEmptyString(o.trackId);
    const regionId = nonEmptyString(o.regionId);
    if (trackId !== undefined && regionId !== undefined) {
      out.openRegion = { trackId, regionId };
    }
  }

  return out;
}

/** Whole stored table, most-recently-written project first. Never throws. */
function readAll(): StoredEntry[] {
  try {
    const raw = localStorage.getItem(VIEW_STATE_KEY);
    if (raw === null) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    const entries: StoredEntry[] = [];
    for (const e of parsed) {
      if (typeof e !== "object" || e === null) continue;
      const path = normalizePath(String((e as Record<string, unknown>).path ?? ""));
      if (path === "") continue;
      entries.push({ path, state: sanitizeViewState((e as Record<string, unknown>).state) });
    }
    return entries;
  } catch {
    return [];
  }
}

function writeAll(entries: StoredEntry[]): void {
  try {
    localStorage.setItem(VIEW_STATE_KEY, JSON.stringify(entries.slice(0, MAX_VIEW_STATE_PROJECTS)));
  } catch {
    // Storage unavailable/full: skip remembering rather than surface an error.
  }
}

/**
 * Remembered view for `projectPath`. Returns `{}` when nothing is stored,
 * storage is unavailable, or the stored value is corrupt.
 */
export function getViewState(projectPath: string): ProjectViewState {
  const key = normalizePath(projectPath);
  if (key === "") return {};
  return readAll().find((e) => e.path === key)?.state ?? {};
}

/**
 * Merge `patch` into the project's remembered view and promote it to the
 * front of the table. Top-level fields REPLACE (an `arrangement` patch
 * carries the whole arrangement slice); this keeps writers from having to
 * read-modify-write. A `null` loop/openRegion clears it.
 */
export function patchViewState(projectPath: string, patch: ProjectViewState): void {
  const key = normalizePath(projectPath);
  if (key === "") return;
  const entries = readAll();
  const existing = entries.find((e) => e.path === key)?.state ?? {};
  const merged: ProjectViewState = { ...existing, ...patch };
  writeAll([{ path: key, state: merged }, ...entries.filter((e) => e.path !== key)]);
}

/** Forget one project's view (used by tests; also the seam for a future
 *  "reset view" action). */
export function clearViewState(projectPath: string): void {
  const key = normalizePath(projectPath);
  if (key === "") return;
  writeAll(readAll().filter((e) => e.path !== key));
}

// ---------------------------------------------------------------------------
// Resolvers: agreement between a stored value and the project as it is NOW.
// ---------------------------------------------------------------------------

/**
 * Rebuild the full SelectedRegion for a stored region reference, or null when
 * the track or the region is gone (project edited elsewhere between sittings).
 *
 * Only the two IDs come from storage — name, channel kind, start and duration
 * are read from the LIVE track, so a region that was moved or resized while we
 * were away reopens at its real position rather than at a remembered ghost.
 */
export function resolveOpenRegion(
  ref: OpenRegionRef | null | undefined,
  tracks: Track[],
): SelectedRegion | null {
  if (!ref) return null;
  const track = tracks.find((t) => t.id === ref.trackId);
  if (!track) return null;
  const region = track.regions.find((r) => r.id === ref.regionId);
  if (!region) return null;
  return {
    trackId: track.id,
    trackName: track.name,
    regionId: region.id,
    channelType: channelTypeOf(track),
    startTick: region.startTick,
    durationTicks: region.durationTicks,
  };
}

/** Last tick any region covers; 0 when the project has no regions. */
export function songEndTick(tracks: Track[]): number {
  let end = 0;
  for (const t of tracks) {
    for (const r of t.regions) {
      end = Math.max(end, r.startTick + r.durationTicks);
    }
  }
  return end;
}

/**
 * A stored loop range, or null when it no longer describes anything playable:
 * negative start, empty/inverted range, or a range that starts at or past the
 * end of every region left in the project (its content was deleted while we
 * were away — re-arming it would loop silence).
 */
export function resolveLoop(
  loop: LoopViewState | null | undefined,
  songEnd: number,
): LoopViewState | null {
  if (!loop) return null;
  if (loop.start < 0 || loop.end <= loop.start) return null;
  if (songEnd > 0 && loop.start >= songEnd) return null;
  return loop;
}

/** Clamp a stored value into [min, max]; `fallback` when it is absent. */
export function clampNumber(
  value: number | undefined,
  min: number,
  max: number,
  fallback: number,
): number {
  if (value === undefined) return fallback;
  return Math.min(max, Math.max(min, value));
}

/**
 * Clamp a stored list index into a list of `length` entries; `fallback` when
 * absent or not a whole number in range. Used for the piano-roll grid
 * selector, whose option list can grow or shrink between builds.
 */
export function clampIndex(value: number | undefined, length: number, fallback: number): number {
  if (value === undefined || !Number.isInteger(value)) return fallback;
  if (value < 0 || value >= length) return fallback;
  return value;
}
