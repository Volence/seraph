/**
 * MRU list of project-location directories, used to prefill the Location
 * field in the New Project / Import dialogs and to seed the Browse dialogs'
 * starting directory.
 *
 * PERSISTENCE SEAM: this is the app's first (and so far only) use of
 * localStorage. The future workspace-persistence feature (audit F15) should
 * reuse this pattern — a clearly named versioned key plus a small typed
 * get/set module — rather than scattering raw localStorage calls.
 */

/** localStorage key. Versioned so a future shape change can migrate cleanly. */
export const RECENT_LOCATIONS_KEY = "seraph.recentProjectLocations.v1";

/** Maximum entries kept; oldest are dropped. */
export const MAX_RECENT_LOCATIONS = 8;

/**
 * Strip trailing separators so "/a/b" and "/a/b/" dedup as one entry.
 * Exported as `normalizePath` because `viewState.ts` keys its per-project
 * records by the same normalization — two modules disagreeing about whether
 * "/a/b/" and "/a/b" are one project would silently split its stored state.
 */
export function normalizePath(path: string): string {
  const trimmed = path.trim();
  if (trimmed === "") return "";
  const stripped = trimmed.replace(/[/\\]+$/, "");
  // "/" (or "\\") alone would strip to "" — keep the root itself.
  return stripped === "" ? trimmed[0] : stripped;
}

/**
 * All remembered locations, most-recent first. Returns [] when nothing is
 * stored, storage is unavailable, or the stored value is corrupt.
 */
export function getRecentLocations(): string[] {
  try {
    const raw = localStorage.getItem(RECENT_LOCATIONS_KEY);
    if (raw === null) return [];
    const parsed: unknown = JSON.parse(raw);
    if (!Array.isArray(parsed)) return [];
    return parsed.filter((e): e is string => typeof e === "string" && e !== "");
  } catch {
    return [];
  }
}

/** The most recently used location, or `fallback` when none is stored. */
export function mostRecentLocation(fallback = ""): string {
  return getRecentLocations()[0] ?? fallback;
}

/**
 * Record a successfully used location: dedup (trailing-slash insensitive),
 * most-recent first, capped at MAX_RECENT_LOCATIONS. Empty/whitespace paths
 * and storage failures are ignored silently — this is a convenience, never
 * an error path.
 */
export function rememberLocation(path: string): void {
  const entry = normalizePath(path);
  if (entry === "") return;
  const list = [entry, ...getRecentLocations().filter((e) => normalizePath(e) !== entry)];
  try {
    localStorage.setItem(RECENT_LOCATIONS_KEY, JSON.stringify(list.slice(0, MAX_RECENT_LOCATIONS)));
  } catch {
    // Storage unavailable/full: skip remembering rather than surface an error.
  }
}

/**
 * Parent directory of a path ("" when there is none). Used by the Open
 * Project flow: the user picks a project directory, and its parent is the
 * "location" worth remembering for future creates.
 */
export function parentDirectory(path: string): string {
  const normalized = normalizePath(path);
  const cut = Math.max(normalized.lastIndexOf("/"), normalized.lastIndexOf("\\"));
  if (cut <= 0) return "";
  return normalized.slice(0, cut);
}
