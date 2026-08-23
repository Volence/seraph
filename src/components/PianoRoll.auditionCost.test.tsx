import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRoll } from "./PianoRoll";
import * as ipc from "../api/ipc";
import { PITCH_RANGES } from "../utils/pianoRollEdit";
import type { Note, SelectedRegion, SongMetadata, Track } from "../types/model";

vi.mock("../api/ipc");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

/**
 * AUDITION COST (F26).
 *
 * `handleAudition` used to open with `ipc.listTracks()` — the whole
 * track/region/note tree serialized across IPC — to read ONE field, the
 * track's instrument binding. It runs on note press, on grid double-click,
 * on every keys-column click, and (since Draw Mode, F6) once per new pitch
 * of a paint drag, so a painted run over N rows issued N full-song fetches.
 *
 * The fix is a narrow read (`get_track_instrument`), not a cache: the
 * binding changes from surfaces this component never hears about (a library
 * drop or unbind on TrackHeader, a track delete), and the only cross-
 * component signal that exists is SONG_REVERTED_EVENT, which undo/redo
 * alone dispatches. The freshness pin below is what a cache would fail.
 */

const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [4, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

const REGION_DURATION = 7680;
const [, FM_MAX] = PITCH_RANGES.fm;

// Derived from the component's own defaults, exactly as the Draw Mode tests
// in PianoRoll.test.tsx derive them — never pinned.
const ROW_H = 14;
const BAR = meta.ticksPerBeat * meta.timeSignature[0];
const SNAP = BAR / 16;
const TPP = Math.min(REGION_DURATION / 800, (BAR * 8) / 800);
const CELL_PX = SNAP / TPP;
const rowY = (pitch: number) => (FM_MAX - pitch) * ROW_H + ROW_H / 2;

function note(tick: number, pitch: number, instrumentId?: string): Note {
  return { tick, pitch, velocity: 100, durationTicks: 240, instrumentId };
}

function track(channel: Track["channel"], instrumentId: string | null, notes: Note[]): Track {
  return {
    id: "track-1",
    name: "Lane",
    channel,
    instrumentId,
    regions: [{ id: "region-1", startTick: 0, durationTicks: REGION_DURATION, notes }],
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
}

function sel(channelType: SelectedRegion["channelType"]): SelectedRegion {
  return {
    trackId: "track-1",
    trackName: "Lane",
    regionId: "region-1",
    channelType,
    startTick: 0,
    durationTicks: REGION_DURATION,
  };
}

function roll(channelType: SelectedRegion["channelType"]) {
  return (
    <PianoRoll
      region={sel(channelType)}
      onClose={vi.fn()}
      playing={false}
      projectMeta={meta}
      seekTick={0}
      onSeek={vi.fn()}
    />
  );
}

/** No timers in these chains — drain the microtasks and assert directly. */
async function flush() {
  await act(async () => {});
}

/** DOM order: [0] bar ruler, [1] key column, [2] note grid, [3] velocity. */
const keysCanvas = (c: HTMLElement) => c.querySelectorAll("canvas")[1];
const noteCanvas = (c: HTMLElement) => c.querySelectorAll("canvas")[2];

function clickKey(container: HTMLElement, pitch: number, y = rowY(pitch)) {
  fireEvent.click(keysCanvas(container), { clientX: 5, clientY: y });
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ipc.listTracks).mockResolvedValue([track({ Fm: 0 }, "fm-voice", [note(0, 60)])]);
  vi.mocked(ipc.getTrackInstrument).mockResolvedValue("fm-voice");
  vi.mocked(ipc.listDacInstruments).mockResolvedValue([]);
  vi.mocked(ipc.previewFmInstrument).mockResolvedValue(undefined);
  vi.mocked(ipc.previewPsgInstrument).mockResolvedValue(undefined);
  vi.mocked(ipc.previewDac).mockResolvedValue(undefined);
  vi.mocked(ipc.addNote).mockResolvedValue(undefined as never);
  vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
  vi.mocked(ipc.beginUndoGroup).mockResolvedValue(undefined);
  vi.mocked(ipc.endUndoGroup).mockResolvedValue(undefined);
});

describe("audition round-trip cost (F26)", () => {
  it("auditions without a single whole-song fetch", async () => {
    const { container } = render(roll("fm"));
    await flush();
    // The mount's own refresh() is the ONLY listTracks the roll owes.
    const afterMount = vi.mocked(ipc.listTracks).mock.calls.length;

    for (const pitch of [60, 62, 64, 65, 67]) {
      clickKey(container, pitch);
      await flush();
    }

    expect(ipc.previewFmInstrument).toHaveBeenCalledTimes(5);
    expect(vi.mocked(ipc.listTracks).mock.calls.length).toBe(afterMount);
    expect(ipc.getTrackInstrument).toHaveBeenCalledTimes(5);
    expect(ipc.getTrackInstrument).toHaveBeenCalledWith("track-1");
  });

  it("a Draw-Mode paint run costs no whole-song fetch per row it crosses", async () => {
    // The amplified case F26 names: one audition per NEW pitch of the drag.
    // Empty region — a mousedown ON a note is a move-drag, not a paint run.
    vi.mocked(ipc.listTracks).mockResolvedValue([track({ Fm: 0 }, "fm-voice", [])]);
    const { container, getByRole } = render(roll("fm"));
    await flush();
    const afterMount = vi.mocked(ipc.listTracks).mock.calls.length;

    fireEvent.click(getByRole("button", { name: "Draw" }));
    const canvas = noteCanvas(container);
    fireEvent.mouseDown(canvas, { clientX: 1, clientY: rowY(60), button: 0 });
    fireEvent.mouseMove(window, { clientX: CELL_PX + 1, clientY: rowY(61) });
    fireEvent.mouseMove(window, { clientX: CELL_PX * 2 + 1, clientY: rowY(62) });
    fireEvent.mouseUp(window, { clientX: CELL_PX * 2 + 1, clientY: rowY(62) });
    await flush();

    // Three distinct pitches painted ⇒ three auditions...
    expect(ipc.addNote).toHaveBeenCalledTimes(3);
    expect(ipc.getTrackInstrument).toHaveBeenCalledTimes(3);
    // ...and the run's own commit is the only thing that refetches the song.
    expect(vi.mocked(ipc.listTracks).mock.calls.length).toBe(afterMount + 1);
  });

  it("hears a voice change made outside the roll on the very next audition", async () => {
    // TrackHeader's library drop / unbind and ArrangementView's track delete
    // all rewrite this binding, and NONE of them notify the piano roll. A
    // value cached off refresh() would keep auditioning the old voice.
    const { container } = render(roll("fm"));
    await flush();

    clickKey(container, 60);
    await flush();
    expect(ipc.previewFmInstrument).toHaveBeenLastCalledWith("fm-voice", 60);

    // ...the user drops a different voice on the track header.
    vi.mocked(ipc.getTrackInstrument).mockResolvedValue("fm-voice-2");
    clickKey(container, 60);
    await flush();
    expect(ipc.previewFmInstrument).toHaveBeenLastCalledWith("fm-voice-2", 60);

    // ...and then unbinds it: nothing to play, and nothing played.
    vi.mocked(ipc.getTrackInstrument).mockResolvedValue(null);
    vi.mocked(ipc.previewFmInstrument).mockClear();
    clickKey(container, 60);
    await flush();
    expect(ipc.previewFmInstrument).not.toHaveBeenCalled();
  });

  it("routes PSG through the PSG preview with the same narrow read", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue([track({ Psg: 0 }, "psg-voice", [note(0, 60)])]);
    vi.mocked(ipc.getTrackInstrument).mockResolvedValue("psg-voice");
    const { container } = render(roll("psg"));
    await flush();
    const afterMount = vi.mocked(ipc.listTracks).mock.calls.length;

    clickKey(container, 60);
    await flush();

    expect(ipc.previewPsgInstrument).toHaveBeenCalledWith("psg-voice", 60);
    expect(vi.mocked(ipc.listTracks).mock.calls.length).toBe(afterMount);
  });

  it("keeps the DAC per-note override winning over the lane binding", async () => {
    // The DAC branch reads the override off the notes it already holds, so
    // the narrow read must not have cost it that precedence.
    vi.mocked(ipc.listTracks).mockResolvedValue([
      track({ Dac: 0 }, "dac-lane", [note(0, 41, "dac-kick")]),
    ]);
    vi.mocked(ipc.getTrackInstrument).mockResolvedValue("dac-lane");
    const { container } = render(roll("dac"));
    await flush();

    // DAC rows are 22px tall, and the DAC pitch range is derived from the
    // notes present: [lo-1, hi+1] = [40, 42] for a lone note at 41.
    const dacRowY = (pitch: number) => (42 - pitch) * 22 + 11;
    clickKey(container, 41, dacRowY(41));
    await flush();
    expect(ipc.previewDac).toHaveBeenCalledWith("dac-kick");

    // A row with no voiced note falls back to the lane binding.
    vi.mocked(ipc.previewDac).mockClear();
    clickKey(container, 40, dacRowY(40));
    await flush();
    expect(ipc.previewDac).toHaveBeenCalledWith("dac-lane");
  });
});
