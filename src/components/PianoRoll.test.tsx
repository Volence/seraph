import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRoll } from "./PianoRoll";
import * as ipc from "../api/ipc";
import { OCTAVE_SEMITONES, PITCH_RANGES } from "../utils/pianoRollEdit";
import { resetClipboardForTest } from "../utils/clipboard";
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

async function renderRoll(notes: Note[], seekTick = 0) {
  const { region } = setupTracks(notes);
  const utils = render(
    <PianoRoll region={region} onClose={vi.fn()} playing={false} projectMeta={meta} seekTick={seekTick} onSeek={vi.fn()} />,
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
  resetClipboardForTest();
});

describe("velocity lane alignment (G5)", () => {
  // The velocity lane canvas starts at x=0 of the piano roll, but the note
  // canvas starts after the key column — the lane must be offset by the key
  // column width so bars sit under their notes.

  function velocityLaneContainer(container: HTMLElement): HTMLElement {
    // DOM order: keys canvas, note canvas, velocity lane canvas (last).
    const canvases = container.querySelectorAll("canvas");
    return canvases[canvases.length - 1].parentElement as HTMLElement;
  }

  it("offsets the lane by the melodic key column width", async () => {
    const { container } = await renderRoll([note(0, 60)]);
    const { MELODIC_KEYS_WIDTH } = await import("./PianoRollKeys");
    expect(velocityLaneContainer(container).style.marginLeft).toBe(`${MELODIC_KEYS_WIDTH}px`);
  });

  it("offsets by the DAC key column width and tracks its resize", async () => {
    const { region } = (() => {
      const track: Track = {
        id: "track-1",
        name: "Drums",
        channel: { Dac: 0 },
        instrumentId: null,
        regions: [{ id: "region-1", startTick: 0, durationTicks: 7680, notes: [note(0, 40)] }],
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
          trackName: "Drums",
          regionId: "region-1",
          channelType: "dac",
          startTick: 0,
          durationTicks: 7680,
        } as SelectedRegion,
      };
    })();
    const { container } = render(
      <PianoRoll region={region} onClose={vi.fn()} playing={false} projectMeta={meta} seekTick={0} onSeek={vi.fn()} />,
    );
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await act(async () => {});

    const { DAC_KEYS_WIDTH } = await import("./PianoRollKeys");
    expect(velocityLaneContainer(container).style.marginLeft).toBe(`${DAC_KEYS_WIDTH}px`);

    // Drag the key-column resize handle 50px right; the lane offset follows.
    // Canvas order: [0] bar ruler, [1] key column, [2] note grid.
    const keysContainer = container.querySelectorAll("canvas")[1].parentElement as HTMLElement;
    const handle = keysContainer.querySelector("div") as HTMLElement;
    expect(handle).not.toBeNull();
    fireEvent.mouseDown(handle, { clientX: 100 });
    fireEvent.mouseMove(document, { clientX: 150 });
    fireEvent.mouseUp(document);
    expect(velocityLaneContainer(container).style.marginLeft).toBe(`${DAC_KEYS_WIDTH + 50}px`);
  });
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

describe("PianoRoll clipboard (Ctrl+C/X/V)", () => {
  // Region: startTick 0, durationTicks 7680 (see setupTracks).

  it("Ctrl+C then Ctrl+V pastes at the seek cursor when it falls inside the region", async () => {
    // Cursor at tick 960 (inside 0..7680): the earliest copied note anchors
    // there; the second keeps its +480 offset.
    await renderRoll([note(0, 60), note(480, 64)], 960);
    selectAll();
    fireEvent.keyDown(window, { key: "c", ctrlKey: true });
    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await waitFor(() => expect(ipc.addNote).toHaveBeenCalledTimes(2));
    expect(ipc.addNote).toHaveBeenCalledWith("track-1", "region-1", 960, 60, 100, 240);
    expect(ipc.addNote).toHaveBeenCalledWith("track-1", "region-1", 1440, 64, 100, 240);
  });

  it("Ctrl+V anchors at tick 0 when the seek cursor is outside the region", async () => {
    await renderRoll([note(960, 60)], 9999); // cursor past the region end
    selectAll();
    fireEvent.keyDown(window, { key: "c", ctrlKey: true });
    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await waitFor(() => expect(ipc.addNote).toHaveBeenCalledWith("track-1", "region-1", 0, 60, 100, 240));
  });

  it("paste wraps its addNote loop in one undo group and selects the pasted notes", async () => {
    vi.mocked(ipc.addNote).mockResolvedValueOnce(2).mockResolvedValueOnce(3);
    const { getByText } = await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "c", ctrlKey: true });
    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await waitFor(() => expect(ipc.endUndoGroup).toHaveBeenCalled());
    expect(ipc.addNote).toHaveBeenCalledTimes(2);
    expectGroupWraps(vi.mocked(ipc.addNote));
    // Pasted notes become the selection (2 notes shown in the header).
    await waitFor(() => expect(getByText("2 notes")).toBeInTheDocument());
  });

  it("skips (with a console.warn) notes that would land past the region end", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    // Cursor at 7440: note at offset 0 fits (240 ticks to the end); the
    // +480-offset note would start at 7920 >= 7680 and must be skipped.
    await renderRoll([note(0, 60), note(480, 64)], 7440);
    selectAll();
    fireEvent.keyDown(window, { key: "c", ctrlKey: true });
    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await waitFor(() => expect(ipc.addNote).toHaveBeenCalledTimes(1));
    expect(ipc.addNote).toHaveBeenCalledWith("track-1", "region-1", 7440, 60, 100, 240);
    expect(warn).toHaveBeenCalledWith(expect.stringContaining("1"));
    warn.mockRestore();
  });

  it("Ctrl+X copies then deletes the selection in one undo group", async () => {
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "x", ctrlKey: true });
    await waitFor(() => expect(ipc.endUndoGroup).toHaveBeenCalled());
    expect(ipc.deleteNote).toHaveBeenCalledTimes(2);
    expectGroupWraps(vi.mocked(ipc.deleteNote));
    // The cut notes are on the clipboard: paste re-adds them.
    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await waitFor(() => expect(ipc.addNote).toHaveBeenCalledTimes(2));
  });

  it("clipboard survives switching regions (module state, not component state)", async () => {
    const first = await renderRoll([note(0, 60)]);
    selectAll();
    fireEvent.keyDown(window, { key: "c", ctrlKey: true });
    first.unmount();
    // A different region, freshly mounted.
    await renderRoll([]);
    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await waitFor(() => expect(ipc.addNote).toHaveBeenCalledWith("track-1", "region-1", 0, 60, 100, 240));
  });

  it("Ctrl+C with no selection copies nothing", async () => {
    await renderRoll([note(0, 60)]);
    fireEvent.keyDown(window, { key: "c", ctrlKey: true });
    fireEvent.keyDown(window, { key: "v", ctrlKey: true });
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.addNote).not.toHaveBeenCalled();
  });
});

describe("PianoRoll arrow nudge (ArrowLeft/Right)", () => {
  // Default grid index is 4 => 1/16 of a bar. ticksPerBar = 480 * 4 = 1920,
  // so the default snap step is 1920 / 16 = 120 ticks (derived, not pinned).
  const SNAP = (meta.ticksPerBeat * meta.timeSignature[0]) / 16;

  it("ArrowRight moves every selected note right by the grid-snap step", async () => {
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => expect(ipc.updateNote).toHaveBeenCalledTimes(2));
    expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 0, SNAP, 60, 100, 240);
    expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 1, 480 + SNAP, 64, 100, 240);
  });

  it("ArrowLeft moves left by the grid-snap step", async () => {
    await renderRoll([note(480, 60)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await waitFor(() => expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 0, 480 - SNAP, 60, 100, 240));
  });

  it("Ctrl+Arrow nudges by exactly 1 tick", async () => {
    await renderRoll([note(480, 60)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowRight", ctrlKey: true });
    await waitFor(() => expect(ipc.updateNote).toHaveBeenCalledWith("track-1", "region-1", 0, 481, 60, 100, 240));
  });

  it("blocks the whole move when any note would cross tick 0", async () => {
    // One note at 0, one with room: block-whole-move (transpose convention).
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowLeft" });
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.updateNote).not.toHaveBeenCalled();
  });

  it("blocks at the region end", async () => {
    // Note end at 7680 (region end): any right nudge must be refused.
    await renderRoll([note(7440, 60)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowRight", ctrlKey: true });
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.updateNote).not.toHaveBeenCalled();
  });

  it("wraps the nudge batch in a single undo group", async () => {
    await renderRoll([note(0, 60), note(480, 64)]);
    selectAll();
    fireEvent.keyDown(window, { key: "ArrowRight" });
    await waitFor(() => expect(ipc.endUndoGroup).toHaveBeenCalled());
    expectGroupWraps(vi.mocked(ipc.updateNote));
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
    render(<PianoRoll region={region} onClose={onClose} playing={false} projectMeta={meta} seekTick={0} onSeek={vi.fn()} />);
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

describe("note placement velocity", () => {
  // Velocity is TL-denominated in the engine: the sequencer adds
  // (127 - velocity) straight to every FM carrier TL (0.75 dB/step), so 127
  // is the only "no attenuation" value. Hand-placed notes must default to
  // 127 or every new note plays ~20 dB under the audition the instrument
  // was chosen with (the "library voice is extremely quiet" bug — see
  // src-tauri/src/audio/rendered_rms.rs).
  it("draws new notes at full (driver-faithful) velocity 127", async () => {
    const { container } = await renderRoll([]);
    // DOM order: ruler canvas, keys canvas, note canvas, velocity lane canvas.
    const canvases = container.querySelectorAll("canvas");
    const noteCanvas = canvases[2];
    fireEvent.doubleClick(noteCanvas, { clientX: 5, clientY: 50 });
    fireEvent.mouseUp(window, { clientX: 5, clientY: 50 });
    await waitFor(() => expect(ipc.addNote).toHaveBeenCalled());
    const call = vi.mocked(ipc.addNote).mock.calls[0];
    expect(call[4]).toBe(127);
  });
});
