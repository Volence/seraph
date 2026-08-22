import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRoll } from "./PianoRoll";
import * as ipc from "../api/ipc";
import { resetClipboardForTest } from "../utils/clipboard";
import { pianoRollNoteSelectionActive } from "../utils/noteSelection";
import { PITCH_RANGES } from "../utils/pianoRollEdit";
import type { Note, SelectedRegion, SongMetadata, Track } from "../types/model";

vi.mock("../api/ipc");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

/**
 * REGION-SWITCH STALENESS (the bug CLASS behind the ruler-scale fix).
 *
 * BottomPanel renders ONE persistent PianoRoll with no React `key`, so
 * opening a different region does NOT remount it: every piece of state in
 * that subtree survives the switch. The zoom half was fixed by refitting on
 * `region.regionId`; these tests cover the rest of the family.
 *
 * The hazardous items are the ones that INDEX into a region's note array:
 *  - `selectedNotes` is a Set of indices. Carried into another region, the
 *    next Delete / transpose / nudge / cut / set-voice hits arbitrary notes
 *    of the newly-opened region.
 *  - `notes` itself is fetched asynchronously, so between the switch and the
 *    reply landing the component holds region A's notes while every IPC call
 *    it makes carries region B's ids.
 *  - replies can land OUT OF ORDER: a reply for the region the user just
 *    left must not overwrite the open region's notes, and must not fire the
 *    close-on-missing path (which would close the region just opened).
 *
 *  - a MOUSE GESTURE in flight when the switch lands keeps writing, through
 *    indices captured in the region it began on. A region switch needs no
 *    pointer event at all: ArrangementView's window keydown handler calls
 *    onSelectRegions from Ctrl+D and Ctrl+V, so nothing tears the gesture
 *    down (see the describe block for the per-path reachability derivation).
 *
 * The pins at the bottom fix the other side of the ledger: state that is
 * SUPPOSED to survive a switch (the module clipboard, the grid selector).
 * They exist so a future "just remount the subtree" fix cannot silently
 * throw those away.
 */

const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [4, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

const REGION_DURATION = 7680;

function note(tick: number, pitch: number): Note {
  return { tick, pitch, velocity: 100, durationTicks: 240 };
}

/** Region B deliberately holds MORE notes than region A, so every index a
 *  stale selection carries over is IN RANGE for B. A smaller B would let
 *  the out-of-range guards refuse the edit and the tests would pass for the
 *  wrong reason — the corruption these pin is "the wrong notes were
 *  edited", not "the edit was rejected". A and B share no pitch or tick,
 *  so a wrongly-applied edit is unambiguous in the assertion. */
const NOTES_A = [note(0, 60), note(480, 64), note(960, 67)];
const NOTES_B = [note(1920, 72), note(2400, 74), note(2880, 76), note(3360, 77)];

function trackWith(regions: { id: string; notes: Note[] }[]): Track {
  return {
    id: "track-1",
    name: "Lead",
    channel: { Fm: 0 },
    instrumentId: "inst-1",
    regions: regions.map((r) => ({
      id: r.id,
      startTick: 0,
      durationTicks: REGION_DURATION,
      notes: r.notes,
    })),
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
}

const BOTH_REGIONS = [trackWith([{ id: "region-1", notes: NOTES_A }, { id: "region-2", notes: NOTES_B }])];
/** The list the backend returns after region-1 was deleted. */
const ONLY_REGION_2 = [trackWith([{ id: "region-2", notes: NOTES_B }])];

function sel(regionId: string): SelectedRegion {
  return {
    trackId: "track-1",
    trackName: "Lead",
    regionId,
    channelType: "fm",
    startTick: 0,
    durationTicks: REGION_DURATION,
  };
}

function roll(regionId: string, onClose = vi.fn()) {
  return (
    <PianoRoll
      region={sel(regionId)}
      onClose={onClose}
      playing={false}
      projectMeta={meta}
      seekTick={0}
      onSeek={vi.fn()}
    />
  );
}

function selectAll() {
  fireEvent.keyDown(window, { key: "a", ctrlKey: true });
}

/** Let every pending microtask + effect settle. */
async function flush() {
  await act(async () => {});
}

/** A promise whose resolution this test controls — the seam for landing
 *  IPC replies out of order across a region switch. */
function deferred<T>() {
  let resolve!: (v: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

beforeEach(() => {
  vi.clearAllMocks();
  resetClipboardForTest();
});

describe("region switch clears index-based note selection", () => {
  it("drops the selection when a different region is opened in the same roll", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue(BOTH_REGIONS);
    const { rerender, getByText, queryByText } = render(roll("region-1"));
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await flush();

    selectAll();
    // Three notes selected — indices {0,1,2} into region-1.
    await waitFor(() => expect(getByText(`${NOTES_A.length} notes`)).toBeInTheDocument());

    rerender(roll("region-2"));
    await flush();

    // The header selection readout is the observable face of selectedNotes.
    expect(queryByText(`${NOTES_A.length} notes`)).toBeNull();

    fireEvent.keyDown(window, { key: "Delete" });
    await flush();
    // A surviving {0,1,2} deletes region-2's first three notes — every
    // index is in range there, so nothing refuses the edit.
    expect(ipc.deleteNote).not.toHaveBeenCalled();
  });

  it("releases the cross-tree note-selection signal so arrangement Delete works again (G1)", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue(BOTH_REGIONS);
    const { rerender } = render(roll("region-1"));
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await flush();

    selectAll();
    await flush();
    expect(pianoRollNoteSelectionActive()).toBe(true);

    rerender(roll("region-2"));
    await flush();
    // Otherwise ArrangementView's Delete keeps deferring to a piano roll
    // that no longer has a real selection.
    expect(pianoRollNoteSelectionActive()).toBe(false);
  });

  it("does not carry the selection into a transpose on the new region", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue(BOTH_REGIONS);
    const { rerender } = render(roll("region-1"));
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await flush();

    selectAll();
    rerender(roll("region-2"));
    await flush();

    fireEvent.keyDown(window, { key: "ArrowUp" });
    await flush();
    // Region-2's notes 0-2 would be transposed a semitone up, silently, on
    // a selection the user made in another region.
    expect(ipc.updateNote).not.toHaveBeenCalled();
  });
});

describe("region switch does not edit through the previous region's notes", () => {
  it("holds no notes while the new region's fetch is still in flight", async () => {
    vi.mocked(ipc.listTracks)
      .mockResolvedValueOnce(BOTH_REGIONS)
      // The reply for region-2 never lands during this test.
      .mockReturnValueOnce(new Promise<Track[]>(() => {}));
    const { rerender } = render(roll("region-1"));
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await flush();

    rerender(roll("region-2"));
    await flush();

    // Ctrl+A must not select region-1's notes while region-2 is the open
    // document: the ids on every following IPC call are region-2's.
    selectAll();
    await flush();
    fireEvent.keyDown(window, { key: "Delete" });
    await flush();
    expect(ipc.deleteNote).not.toHaveBeenCalled();
  });

  it("ignores a reply for the region the user just left", async () => {
    const first = deferred<Track[]>();
    const second = deferred<Track[]>();
    vi.mocked(ipc.listTracks)
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);

    const { rerender } = render(roll("region-1"));
    await flush();
    rerender(roll("region-2"));
    await flush();

    // region-2's reply lands first, then region-1's stale one.
    await act(async () => { second.resolve(BOTH_REGIONS); });
    await act(async () => { first.resolve(BOTH_REGIONS); });

    selectAll();
    await flush();
    fireEvent.keyDown(window, { key: "Delete" });
    await waitFor(() => expect(ipc.deleteNote).toHaveBeenCalled());
    // Ctrl+A selects every note of the OPEN region: region-2's four, not
    // region-1's three that the late reply tried to install.
    expect(ipc.deleteNote).toHaveBeenCalledTimes(NOTES_B.length);
    for (let i = 0; i < NOTES_B.length; i++) {
      expect(ipc.deleteNote).toHaveBeenCalledWith("track-1", "region-2", i);
    }
  });

  it("a late reply that lost its region does not close the region just opened", async () => {
    const first = deferred<Track[]>();
    const second = deferred<Track[]>();
    vi.mocked(ipc.listTracks)
      .mockReturnValueOnce(first.promise)
      .mockReturnValueOnce(second.promise);

    const onClose = vi.fn();
    const { rerender } = render(roll("region-1", onClose));
    await flush();
    rerender(roll("region-2", onClose));
    await flush();

    // region-1 was deleted while its fetch was in flight; both replies show
    // the post-delete world. Only the OPEN region's absence may close.
    await act(async () => { second.resolve(ONLY_REGION_2); });
    await act(async () => { first.resolve(ONLY_REGION_2); });
    await flush();

    expect(onClose).not.toHaveBeenCalled();
  });
});

describe("a mouse gesture in flight when the region switches", () => {
  /**
   * REACHABILITY, derived from source (not from "a switch needs a click").
   * Every caller that changes which region is open:
   *   pointer-driven — ArrangementView.handleRegionDoubleClick,
   *     handleRegionCreate, TimelineCanvas region click/shift-click and its
   *     own marquee, App's onSelectInstrument (clears -> unmount);
   *   KEYBOARD-driven, no pointer event at all —
   *     ArrangementView Ctrl+D duplicate ("the duplicates become the
   *       selection"), gated by `if (pianoRollNoteSelectionActive()) return`;
   *     ArrangementView Ctrl+V region paste ("the pasted regions become the
   *       selection"), gated ONLY by `lastCopiedKind() === "regions"`;
   *     ArrangementView Delete -> onSelectRegions([]) -> unmount (safe: the
   *       gesture's window listeners are torn down with the component).
   * The pointer paths are safe for a different reason than I first claimed:
   * their own mousedown->mouseup fires the gesture's window-level mouseup
   * first. The KEYBOARD paths have no such event, so the gesture stays live
   * across the switch with the button still down.
   *
   * Which gesture is reachable under which key:
   *  - note MOVE drag: mousedown selects the note, so G1 is true and Ctrl+D
   *    is blocked — but Ctrl+V carries no G1 gate, so it IS reachable.
   *  - RESIZE drag (near-edge mousedown) does NOT select: with an empty
   *    selection G1 is false, so BOTH Ctrl+D and Ctrl+V reach it.
   *  - DRAW (double-click) does not touch the selection: same as resize.
   *  - MARQUEE commits its selection only on mouseup, so with an empty
   *    starting selection G1 is false: both keys reach it.
   * The switch mechanism is irrelevant to what happens next, so these tests
   * drive it the way App does — a new `region` prop on the same instance.
   */

  // Derived from the component, not pinned: melodic rowHeight, the FM pitch
  // ceiling that fixes row 0, the default zoom, and the default grid snap
  // (GRID_OPTIONS[4] = 1/16 of a bar).
  const ROW_H = 14;
  const MAX_PITCH = PITCH_RANGES.fm[1];
  const BAR = meta.ticksPerBeat * meta.timeSignature[0];
  const TPP = Math.min(REGION_DURATION / 800, (BAR * 8) / 800);
  const SNAP = BAR / 16;
  const rowY = (pitch: number) => (MAX_PITCH - pitch) * ROW_H + ROW_H / 2;
  /** DOM order: [0] bar ruler, [1] key column, [2] note grid, [3] velocity. */
  const noteCanvas = (c: HTMLElement) => c.querySelectorAll("canvas")[2];

  // Region A's first note spans ticks 0..240 => 0..25px at TPP. A press at
  // x=5 is inside it and >6px from the right edge, so it starts a MOVE;
  // x=22 is within EDGE_THRESHOLD of the edge, so it starts a RESIZE.
  const NOTE_A0_W = NOTES_A[0].durationTicks / TPP;
  const MOVE_X = 5;
  const RESIZE_X = Math.round(NOTE_A0_W) - 3;

  async function openRegion1() {
    vi.mocked(ipc.listTracks).mockResolvedValue(BOTH_REGIONS);
    const utils = render(roll("region-1"));
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await flush();
    return utils;
  }

  it("a note-move drag stops writing when the region switches under it", async () => {
    const { container, rerender } = await openRegion1();
    const canvas = noteCanvas(container);

    fireEvent.mouseDown(canvas, { clientX: MOVE_X, clientY: rowY(NOTES_A[0].pitch), button: 0 });
    fireEvent.mouseMove(window, { clientX: MOVE_X + 50, clientY: rowY(NOTES_A[0].pitch) });
    // Baseline: the drag DOES write while its own region is open, so the
    // guard below is scoped to the switch and not a blanket disable.
    const movedTick = Math.round((50 * TPP) / SNAP) * SNAP;
    await waitFor(() =>
      expect(ipc.updateNote).toHaveBeenCalledWith(
        "track-1", "region-1", 0, movedTick, NOTES_A[0].pitch, NOTES_A[0].velocity, NOTES_A[0].durationTicks,
      ),
    );

    vi.mocked(ipc.updateNote).mockClear();
    rerender(roll("region-2"));
    await flush(); // region-2's notes land: the index is now IN RANGE for B

    // Still dragging, button down. Without the guard this writes region A's
    // note 0 position onto region B's note 0 — index 0, tick from A's drag,
    // pitch 60 — over a note that lives at tick 1920, pitch 72.
    fireEvent.mouseMove(window, { clientX: MOVE_X + 100, clientY: rowY(NOTES_A[0].pitch) });
    await flush();
    expect(ipc.updateNote).not.toHaveBeenCalled();

    fireEvent.mouseUp(window);
  });

  it("a note-resize drag stops writing when the region switches under it", async () => {
    const { container, rerender } = await openRegion1();
    const canvas = noteCanvas(container);

    fireEvent.mouseDown(canvas, { clientX: RESIZE_X, clientY: rowY(NOTES_A[0].pitch), button: 0 });
    fireEvent.mouseMove(window, { clientX: RESIZE_X + 50, clientY: rowY(NOTES_A[0].pitch) });
    const grownDuration =
      Math.round((NOTES_A[0].durationTicks + 50 * TPP) / SNAP) * SNAP;
    await waitFor(() =>
      expect(ipc.updateNote).toHaveBeenCalledWith(
        "track-1", "region-1", 0, NOTES_A[0].tick, NOTES_A[0].pitch, NOTES_A[0].velocity, grownDuration,
      ),
    );

    vi.mocked(ipc.updateNote).mockClear();
    rerender(roll("region-2"));
    await flush();

    // Without the guard: region B's note 0 is resized to a duration derived
    // from region A's note.
    fireEvent.mouseMove(window, { clientX: RESIZE_X + 100, clientY: rowY(NOTES_A[0].pitch) });
    await flush();
    expect(ipc.updateNote).not.toHaveBeenCalled();

    fireEvent.mouseUp(window);
  });

  it("a note being drawn is not committed into the region that replaced it", async () => {
    const { container, rerender } = await openRegion1();
    const canvas = noteCanvas(container);

    // Empty row (region A has 60/64/67, region B has 72/74/76/77).
    const DRAW_PITCH = 62;
    fireEvent.doubleClick(canvas, { clientX: MOVE_X, clientY: rowY(DRAW_PITCH) });

    rerender(roll("region-2"));
    await flush();

    // This gesture never reads `notes`, so nothing masks it: the mouseup
    // would add a note to region B at coordinates picked in region A's view.
    fireEvent.mouseUp(window);
    await flush();
    expect(ipc.addNote).not.toHaveBeenCalled();
  });

  it("still commits a marquee begun before the switch — it selects what it visibly covers", async () => {
    // Deliberately NOT guarded, unlike the three above: a marquee writes
    // nothing, and draw() renders the band from the very same view pixels
    // that marqueeRectFromView converts, so what the user sees over region
    // B is exactly what gets selected. Refusing it would leave a visible
    // rubber band that does nothing.
    const { container, rerender, findByText } = await openRegion1();
    const canvas = noteCanvas(container);

    // Start on empty space, well right of region A's notes.
    fireEvent.mouseDown(canvas, { clientX: 400, clientY: rowY(80), button: 0 });

    rerender(roll("region-2"));
    await flush();

    // Sweep back across every one of region B's notes.
    fireEvent.mouseMove(window, { clientX: 150, clientY: rowY(70) });
    fireEvent.mouseUp(window, { clientX: 150, clientY: rowY(70) });
    expect(await findByText(`${NOTES_B.length} notes`)).toBeInTheDocument();
  });
});

describe("state that SHOULD survive a region switch stays put", () => {
  // Guard rail for the design call: the fix resets per-region state instead
  // of remounting the subtree, precisely so these survive.

  it("keeps the module clipboard across a switch in the same roll instance", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue(BOTH_REGIONS);
    vi.mocked(ipc.addNote).mockResolvedValue(0);
    const { rerender } = render(roll("region-1"));
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await flush();

    selectAll();
    fireEvent.keyDown(window, { key: "c", ctrlKey: true });

    rerender(roll("region-2"));
    await flush();

    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await waitFor(() => expect(ipc.addNote).toHaveBeenCalledTimes(NOTES_A.length));
    // Seek cursor 0 is inside region-2, so the earliest copied note anchors
    // at tick 0 and the rest keep their offsets.
    for (const n of NOTES_A) {
      expect(ipc.addNote).toHaveBeenCalledWith(
        "track-1", "region-2", n.tick, n.pitch, n.velocity, n.durationTicks, null,
      );
    }
  });

  it("keeps the grid-size selection across a switch", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue(BOTH_REGIONS);
    const { rerender, container } = render(roll("region-1"));
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await flush();

    const select = container.querySelector("select") as HTMLSelectElement;
    // Index 1 = "1/2" in PianoRoll's GRID_OPTIONS; the default is index 4.
    expect(select.value).not.toBe("1");
    fireEvent.change(select, { target: { value: "1" } });
    expect(select.value).toBe("1");

    rerender(roll("region-2"));
    await flush();
    // A tool setting, not a property of the document: it survives.
    expect((container.querySelector("select") as HTMLSelectElement).value).toBe("1");
  });
});
