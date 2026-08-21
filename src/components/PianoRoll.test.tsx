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
      <PianoRoll region={region} onClose={vi.fn()} playing={false} projectMeta={meta} />,
    );
    await waitFor(() => expect(ipc.listTracks).toHaveBeenCalled());
    await act(async () => {});

    const { DAC_KEYS_WIDTH } = await import("./PianoRollKeys");
    expect(velocityLaneContainer(container).style.marginLeft).toBe(`${DAC_KEYS_WIDTH}px`);

    // Drag the key-column resize handle 50px right; the lane offset follows.
    const keysContainer = container.querySelectorAll("canvas")[0].parentElement as HTMLElement;
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
