import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRoll } from "./PianoRoll";
import * as ipc from "../api/ipc";
import { resetClipboardForTest } from "../utils/clipboard";
import { pianoRollNoteSelectionActive } from "../utils/noteSelection";
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
