/** Fired after undo/redo so open views (arrangement, piano roll) re-fetch. */
export const SONG_REVERTED_EVENT = "seraph:song-reverted";

/** True when a key event targets a form control that should keep its own
 *  keyboard behavior (don't hijack app-level shortcuts while typing). */
export function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as HTMLElement | null;
  if (!el || !el.tagName) return false;
  return (
    ["INPUT", "SELECT", "TEXTAREA"].includes(el.tagName) || el.isContentEditable
  );
}
