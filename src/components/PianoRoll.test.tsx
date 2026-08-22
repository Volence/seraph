import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRoll } from "./PianoRoll";
import * as ipc from "../api/ipc";
import { OCTAVE_SEMITONES, PITCH_RANGES } from "../utils/pianoRollEdit";
import type { Note, SelectedRegion, SongMetadata, Track } from "../types/model";

vi.mock("../api/ipc");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [4, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

const [FM_MIN, FM_MAX] = PITCH_RANGES.fm;

function note(tick: number, pitch: number): Note {
  return { tick, pitch, velocity: 100, durationTicks: 240 };
}

function setupTracks(notes: Note[]): { region: SelectedRegion } {
  const track: Track = {
    id: "track-1",
    name: "Lead",
    channel: { Fm: 0 },
    instrumentId: null,
    regions: [{ id: "region-1", startTick: 0, durationTicks: 7680, notes }],
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
  vi.mocked(ipc.listTracks).mockResolvedValue([track]);
  return {
    region: {
      trackId: "track-1",
      trackName: "Lead",
      regionId: "region-1",
      channelType: "fm",
      startTick: 0,
      durationTicks: 7680,
    },
  };
}

async function renderRoll(notes: Note[]) {
  const { region } = setupTracks(notes);
  const utils = render(
    <PianoRoll region={region} onClose={vi.fn()} playing={false} projectMeta={meta} />,
  );
  // Flush the initial refresh() so the notes state (and the keydown
  // handler's re-registration against it) has definitely landed before
  // any keyboard events fire.
  await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
  await act(async () => {});
  return utils;
}

function selectAll() {
  fireEvent.keyDown(window, { key: "a", ctrlKey: true });
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe("PianoRoll keyboard transpose", () => {
  it("ArrowUp transposes every selected note up one semitone", async () => {
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowUp" });
    await waitFor(() => expect(ipc.updateNote).toHaveBeenCalledTimes(2));
    expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 0, 0, 61, 100, 240);
    expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 1, 480, 65, 100, 240);
  });

  it("ArrowDown transposes down one semitone", async () => {
    await renderRoll([note(0, 60)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowDown" });
    await waitFor(() => expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 0, 0, 59, 100, 240));
  });

  it("Ctrl+ArrowUp transposes by a full octave (named constant, not a magic 12)", async () => {
    await renderRoll([note(0, 60)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowUp", ctrlKey: true });
    await waitFor(() =>
      expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 0, 0, 60 + OCTAVE_SEMITONES, 100, 240),
    );
  });

  it("blocks the whole move when any selected note would leave the pitch range", async () => {
    // One note sits at the FM ceiling; the other has headroom. The move
    // must be refused for BOTH (intervals stay intact), not clamped.
    await renderRoll([note(0, FM_MAX), note(480, 60)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowUp" });
    // give any wrongly-issued updates a tick to land
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.updateNote).not.toHaveBeenCalled();
  });

  it("blocks at the low bound too", async () => {
    await renderRoll([note(0, FM_MIN)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowDown" });
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.updateNote).not.toHaveBeenCalled();
  });

  it("does nothing with no selection", async () => {
    await renderRoll([note(0, 60)]);
    fireEvent.keyDown(window, { key: "ArrowUp" });
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.updateNote).not.toHaveBeenCalled();
  });
});

/** All calls to `inner` landed between the group's begin and its end. */
function expectGroupWraps(inner: ReturnType<typeof vi.mocked<any>>) {
  expect(ipc.beginUndoGroup).toHaveBeenCalledTimes(1);
  expect(ipc.endUndoGroup).toHaveBeenCalledTimes(1);
  const begin = vi.mocked(ipc.beginUndoGroup).mock.invocationCallOrder[0];
  const end = vi.mocked(ipc.endUndoGroup).mock.invocationCallOrder[0];
  for (const order of inner.mock.invocationCallOrder) {
    expect(order).toBeGreaterThan(begin);
    expect(order).toBeLessThan(end);
  }
}

describe("PianoRoll batch edits coalesce into one undo group", () => {
  it("keyboard transpose wraps its updateNote loop in a single undo group", async () => {
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowUp" });
    await waitFor(() => expect(ipc.endUndoGroup).toHaveBeenCalled());
    expect(ipc.updateNote).toHaveBeenCalledTimes(2);
    expectGroupWraps(vi.mocked(ipc.updateNote));
  });

  it("multi-note Delete wraps its deleteNote loop in a single undo group", async () => {
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "Delete" });
    await waitFor(() => expect(ipc.endUndoGroup).toHaveBeenCalled());
    expect(ipc.deleteNote).toHaveBeenCalledTimes(2);
    expectGroupWraps(vi.mocked(ipc.deleteNote));
  });

  it("Ctrl+D duplicate wraps its addNote loop in a single undo group", async () => {
    vi.mocked(ipc.addNote).mockResolvedValue(2);
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "d", ctrlKey: true });
    await waitFor(() => expect(ipc.endUndoGroup).toHaveBeenCalled());
    expect(ipc.addNote).toHaveBeenCalledTimes(2);
    expectGroupWraps(vi.mocked(ipc.addNote));
  });
});

describe("PianoRoll refresh on undo/redo", () => {
  it("re-fetches notes when the song is reverted", async () => {
    await renderRoll([note(0, 60)]);
    vi.mocked(ipc.listTracks).mockClear();
    act(() => {
      window.dispatchEvent(new Event("seraph:song-reverted"));
    });
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
  });

  it("closes when the open region no longer exists after a revert", async () => {
    const { region } = setupTracks([note(0, 60)]);
    const onClose = vi.fn();
    render(<PianoRoll region={region} onClose={onClose} playing={false} projectMeta={meta} />);
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await act(async () => {});
    // The revert removed the region (e.g. undo of add_region).
    vi.mocked(ipc.listTracks).mockResolvedValue([]);
    act(() => {
      window.dispatchEvent(new Event("seraph:song-reverted"));
    });
    await waitFor(() => expect(onClose).toHaveBeenCalled());
  });
});
