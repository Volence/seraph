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

/** Channel kind ("fm" | "psg" | "dac") of the region notes were copied
 *  from — lets paste preserve per-note voices only into a same-kind region
 *  (a voice id is meaningless on another chip, and the backend kind gate
 *  would reject it). */
export type NoteChannelKind = "fm" | "psg" | "dac";

let noteClipboard: Note[] = [];
let noteClipboardChannelType: NoteChannelKind | null = null;
let regionClipboard: RegionClipboardEntry[] = [];
let lastCopied: ClipboardKind | null = null;

/**
 * Snapshot the selected notes onto the clipboard (deep copies — later edits
 * to the source must not leak in). Notes are stored sorted by tick so the
 * earliest note is the paste anchor. An empty selection is a no-op (the
 * previous clipboard survives, like every other DAW). Returns the count.
 * `sourceChannelType` records the copied region's channel kind so paste can
 * keep per-note voices when (and only when) the target kind matches.
 */
export function copyNotes(
  notes: Note[],
  selected: Iterable<number>,
  sourceChannelType?: NoteChannelKind,
): number {
  const picked: Note[] = [];
  for (const index of selected) {
    const n = notes[index];
    if (n) picked.push({ ...n });
  }
  if (picked.length === 0) return 0;
  picked.sort((a, b) => a.tick - b.tick);
  noteClipboard = picked;
  noteClipboardChannelType = sourceChannelType ?? null;
  lastCopied = "notes";
  return picked.length;
}

export function getNoteClipboard(): Note[] {
  return noteClipboard;
}

export function getNoteClipboardChannelType(): NoteChannelKind | null {
  return noteClipboardChannelType;
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

/**
 * Drop everything on the clipboard.
 *
 * The clipboard survives region and view switches on purpose, but NOT a
 * project switch: `noteClipboard` entries carry `instrumentId`s from the old
 * project and `regionClipboard` carries old `trackId`/`regionId`s, so a paste
 * after new/open/import either fails a backend validation or (worse) resolves
 * an id that now means something else. App calls this on every path that
 * changes the open project.
 */
export function resetClipboard(): void {
  noteClipboard = [];
  noteClipboardChannelType = null;
  regionClipboard = [];
  lastCopied = null;
}

/** Test seam: clear all module state between test cases. Deliberately an
 *  alias of `resetClipboard` rather than a second copy of the body — two
 *  bodies drift the moment a third clipboard slot is added, and the separate
 *  name keeps test intent ("isolate the case") readable at the call site. */
export function resetClipboardForTest(): void {
  resetClipboard();
}

export interface NotePastePlan {
  placements: {
    tick: number;
    pitch: number;
    velocity: number;
    durationTicks: number;
    /** Per-note voice override carried from the copied note (null = none).
     *  Callers pass it to addNote only when the target region's channel
     *  kind matches the clipboard's recorded kind. */
    instrumentId: string | null;
  }[];
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
      instrumentId: n.instrumentId ?? null,
    });
  }
  return { placements, skipped };
}
