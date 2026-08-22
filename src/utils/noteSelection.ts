/**
 * Cross-tree signal: does the piano roll currently own a note selection?
 *
 * Two window-level Delete handlers coexist — ArrangementView deletes selected
 * regions, PianoRoll deletes selected notes — and opening a region necessarily
 * selects it, so without this signal deleting a note would also delete the
 * enclosing region (G1). The PianoRoll publishes its note-selection state here
 * (and clears it on unmount); the ArrangementView's Delete handler defers
 * while it is active. Handlers read it at event time, so there is no
 * stale-closure hazard and no dependence on listener registration order.
 */
let active = false;

export function setPianoRollNoteSelectionActive(value: boolean): void {
  active = value;
}

export function pianoRollNoteSelectionActive(): boolean {
  return active;
}
