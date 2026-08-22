// In-memory app clipboard (module state, deliberately NOT the OS clipboard):
// survives switching regions/views for the lifetime of the session, mirroring
// noteSelection.ts's plain-module-state pattern. Two payload slots coexist —
// notes (piano roll) and regions (arrangement) — and `lastCopiedKind`
// arbitrates Ctrl+V: you paste whatever you copied last, so the two
// window-level paste handlers never both fire for one keypress.
import type { Note } from "../types/model";

export type ClipboardKind = "notes" | "regions";

/** A copied region: source ids for server-side cloning, plus a deep-copied
 *  payload so paste still works if the source region was deleted meanwhile. */
export interface RegionClipboardEntry {
  trackId: string;
  regionId: string;
  payload: {
    startTick: number;
    durationTicks: number;
    notes: Note[];
  };
}

let noteClipboard: Note[] = [];
let regionClipboard: RegionClipboardEntry[] = [];
let lastCopied: ClipboardKind | null = null;

/**
 * Snapshot the selected notes onto the clipboard (deep copies — later edits
 * to the source must not leak in). Notes are stored sorted by tick so the
 * earliest note is the paste anchor. An empty selection is a no-op (the
 * previous clipboard survives, like every other DAW). Returns the count.
 */
export function copyNotes(notes: Note[], selected: Iterable<number>): number {
  const picked: Note[] = [];
  for (const index of selected) {
    const n = notes[index];
    if (n) picked.push({ ...n });
  }
  if (picked.length === 0) return 0;
  picked.sort((a, b) => a.tick - b.tick);
  noteClipboard = picked;
  lastCopied = "notes";
  return picked.length;
}

export function getNoteClipboard(): Note[] {
  return noteClipboard;
}

/** Snapshot copied regions. Empty input is a no-op, matching copyNotes. */
export function copyRegions(entries: RegionClipboardEntry[]): number {
  if (entries.length === 0) return 0;
  regionClipboard = entries.map((e) => ({
    trackId: e.trackId,
    regionId: e.regionId,
    payload: {
      startTick: e.payload.startTick,
      durationTicks: e.payload.durationTicks,
      notes: e.payload.notes.map((n) => ({ ...n })),
    },
  }));
  lastCopied = "regions";
  return regionClipboard.length;
}

export function getRegionClipboard(): RegionClipboardEntry[] {
  return regionClipboard;
}

export function lastCopiedKind(): ClipboardKind | null {
  return lastCopied;
}

/** Test seam: clear all module state between test cases. */
export function resetClipboardForTest(): void {
  noteClipboard = [];
  regionClipboard = [];
  lastCopied = null;
}

export interface NotePastePlan {
  placements: { tick: number; pitch: number; velocity: number; durationTicks: number }[];
  /** Notes dropped because they would start at/past the region end. */
  skipped: number;
}

/**
 * Plan a note paste: the EARLIEST copied note lands on `anchorTick` and the
 * rest keep their offsets relative to it. Notes that would start at or past
 * the region end are skipped (callers report the count loudly); a note that
 * starts inside but overhangs the end has its duration clamped to fit.
 */
export function planNotePaste(
  clip: Note[],
  anchorTick: number,
  regionDurationTicks: number,
): NotePastePlan {
  if (clip.length === 0) return { placements: [], skipped: 0 };
  const base = Math.min(...clip.map((n) => n.tick));
  const placements: NotePastePlan["placements"] = [];
  let skipped = 0;
  for (const n of clip) {
    const tick = anchorTick + (n.tick - base);
    if (tick >= regionDurationTicks) {
      skipped++;
      continue;
    }
    placements.push({
      tick,
      pitch: n.pitch,
      velocity: n.velocity,
      durationTicks: Math.min(n.durationTicks, regionDurationTicks - tick),
    });
  }
  return { placements, skipped };
}
