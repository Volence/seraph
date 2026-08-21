/**
 * Shared guard for window-level keyboard shortcut handlers (G2).
 *
 * Returns true when the event target is a place the user types or picks
 * values — text inputs, textareas, selects, contentEditable regions — so
 * global shortcuts (Space, l, Home, Delete, …) must not fire.
 *
 * Accepts the raw `event.target` (EventTarget | null) so every handler can
 * call it without casting.
 */
export function isEditableTarget(target: EventTarget | null): boolean {
  const el = target as (Partial<HTMLElement> & { tagName?: unknown }) | null;
  if (!el || typeof el.tagName !== "string") return false;
  if (el.isContentEditable) return true;
  const tag = el.tagName;
  return tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT";
}
