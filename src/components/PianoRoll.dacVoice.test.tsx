import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { PianoRoll } from "./PianoRoll";
import * as ipc from "../api/ipc";
import * as library from "../api/library";
import type { DacInstrument, Note, SelectedRegion, SongMetadata, Track } from "../types/model";

vi.mock("../api/ipc");
// Keep the real LIBRARY_DRAG_TYPE constant (the drop handlers key off it);
// mock only the async API surface.
vi.mock("../api/library", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api/library")>()),
  libraryEnsureProjectInstrument: vi.fn(),
  libraryAssignToTrack: vi.fn(),
}));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

/**
 * PER-NOTE DAC VOICE ASSIGNMENT (F25).
 *
 * The library holds FM patches and PSG envelopes only — `LibraryInstrument`
 * (src-tauri/src/library/entry.rs) has exactly the `Fm` and `Psg` variants —
 * so `handleVoiceDrop`'s `kind !== region.channelType` gate can NEVER pass on
 * a DAC lane. The backend has supported per-note DAC voices all along
 * (`set_note_instrument` kind-gates DAC in `check_instrument_kind`, and
 * `resolve_instrument_data_by_id` has a DAC arm), so the gap was purely that
 * no UI gesture could reach it. These pin the gesture that now can: the
 * header Sample picker, fed by the PROJECT's DAC bank (`list_dac_instruments`).
 */

const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [4, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

const REGION_DURATION = 7680;

function note(tick: number, pitch: number, instrumentId?: string): Note {
  return { tick, pitch, velocity: 100, durationTicks: 240, instrumentId };
}

function dacSample(id: string, name: string): DacInstrument {
  return {
    id,
    name,
    targetSampleRate: 16000,
    loopStart: null,
    loopLength: null,
    originalFile: `${name}.wav`,
    pcmFile: `${id}.pcm`,
    sourceIsRaw: false,
    metadata: { category: "", author: "", tags: [] },
  };
}

const KICK = dacSample("dac-kick", "Kick.wav");
const HAT = dacSample("dac-hat", "Hat.wav");

/** Two DAC notes on DIFFERENT drum rows, so an assignment aimed at one is
 *  unambiguous in the assertion. */
const DAC_NOTES = [note(0, 41), note(240, 46)];

function dacTrack(notes: Note[]): Track {
  return {
    id: "track-1",
    name: "Drums",
    channel: { Dac: 0 },
    instrumentId: "dac-kick",
    regions: [
      { id: "region-1", startTick: 0, durationTicks: REGION_DURATION, notes },
      // A second DAC region on the same lane, for the region-switch pins.
      { id: "region-3", startTick: REGION_DURATION, durationTicks: REGION_DURATION, notes: [] },
    ],
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
}

function fmTrack(): Track {
  return {
    id: "track-2",
    name: "Lead",
    channel: { Fm: 0 },
    instrumentId: "fm-1",
    regions: [{ id: "region-2", startTick: 0, durationTicks: REGION_DURATION, notes: [note(0, 60)] }],
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
}

function sel(kind: "dac" | "fm" | "dac2"): SelectedRegion {
  if (kind === "fm") {
    return { trackId: "track-2", trackName: "Lead", regionId: "region-2", channelType: "fm", startTick: 0, durationTicks: REGION_DURATION };
  }
  return kind === "dac"
    ? { trackId: "track-1", trackName: "Drums", regionId: "region-1", channelType: "dac", startTick: 0, durationTicks: REGION_DURATION }
    : { trackId: "track-1", trackName: "Drums", regionId: "region-3", channelType: "dac", startTick: REGION_DURATION, durationTicks: REGION_DURATION };
}

function roll(region: SelectedRegion) {
  return (
    <PianoRoll
      region={region}
      onClose={vi.fn()}
      playing={false}
      projectMeta={meta}
      seekTick={0}
      onSeek={vi.fn()}
    />
  );
}

/** Let every pending microtask + effect settle. No timers are involved in
 *  these chains, so `waitFor` would buy nothing but a 1s failure budget. */
async function flush() {
  await act(async () => {});
}

function picker(container: HTMLElement): HTMLSelectElement | null {
  return container.querySelector<HTMLSelectElement>(
    'select[aria-label="DAC sample for selected notes"]',
  );
}

function optionNames(container: HTMLElement): (string | null)[] {
  return Array.from(picker(container)!.options).map((o) => o.textContent);
}

function selectAll() {
  fireEvent.keyDown(window, { key: "a", ctrlKey: true });
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ipc.listTracks).mockResolvedValue([dacTrack(DAC_NOTES), fmTrack()]);
  vi.mocked(ipc.listDacInstruments).mockResolvedValue([KICK, HAT]);
  vi.mocked(ipc.setNoteInstrument).mockResolvedValue(undefined);
  vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
  vi.mocked(ipc.previewDac).mockResolvedValue(undefined);
});

describe("DAC per-note sample picker (F25)", () => {
  it("offers the project's DAC bank on a DAC lane", async () => {
    const { container } = render(roll(sel("dac")));
    await flush();
    const sel_ = picker(container);
    expect(sel_).not.toBeNull();
    const names = Array.from(sel_!.options).map((o) => o.textContent);
    // Derived from the bank the mock returns, plus the clear-to-default entry.
    expect(names).toContain("Kick.wav");
    expect(names).toContain("Hat.wav");
    expect(names).toContain("Lane default");
  });

  it("is absent on a melodic lane, which the library drop gesture already serves", async () => {
    const { container } = render(roll(sel("fm")));
    await flush();
    expect(picker(container)).toBeNull();
    // ...and no DAC bank is fetched for a lane that cannot use one.
    expect(ipc.listDacInstruments).not.toHaveBeenCalled();
  });

  it("sets the chosen sample on the SELECTED notes and makes it audible", async () => {
    const { container } = render(roll(sel("dac")));
    await flush();
    selectAll();
    await flush();

    fireEvent.change(picker(container)!, { target: { value: "dac-hat" } });
    await flush();

    expect(ipc.setNoteInstrument).toHaveBeenCalledWith(
      "track-1",
      "region-1",
      [0, 1],
      "dac-hat",
    );
    // Playback consumes a play-time snapshot: without the reload the running
    // transport never hears the swap (the F1 seam).
    expect(ipc.reloadSequence).toHaveBeenCalled();
    expect(ipc.previewDac).toHaveBeenCalledWith("dac-hat");
  });

  it("clears back to the lane default with a null instrument id", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue([
      dacTrack([note(0, 41, "dac-hat"), note(240, 46, "dac-hat")]),
      fmTrack(),
    ]);
    const { container } = render(roll(sel("dac")));
    await flush();
    selectAll();
    await flush();

    fireEvent.change(picker(container)!, { target: { value: "" } });
    await flush();

    expect(ipc.setNoteInstrument).toHaveBeenCalledWith("track-1", "region-1", [0, 1], null);
    // Nothing to audition when the override is cleared.
    expect(ipc.previewDac).not.toHaveBeenCalled();
  });

  it("names the sample the selection already plays", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue([
      dacTrack([note(0, 41, "dac-hat"), note(240, 46, "dac-hat")]),
      fmTrack(),
    ]);
    const { container } = render(roll(sel("dac")));
    await flush();
    selectAll();
    await flush();
    const p = picker(container)!;
    expect(p.value).toBe("dac-hat");
    expect(p.options[p.selectedIndex].textContent).toBe("Hat.wav");
  });

  it("reads 'mixed' when the selected notes disagree", async () => {
    vi.mocked(ipc.listTracks).mockResolvedValue([
      dacTrack([note(0, 41, "dac-kick"), note(240, 46, "dac-hat")]),
      fmTrack(),
    ]);
    const { container } = render(roll(sel("dac")));
    await flush();
    selectAll();
    await flush();
    const p = picker(container)!;
    expect(p.options[p.selectedIndex].textContent).toBe("Sample (mixed)");
  });

  it("refuses to act with no selection rather than retargeting the lane", async () => {
    const { container } = render(roll(sel("dac")));
    await flush();
    // No selection: the control is disabled, so the destructive whole-lane
    // interpretation is unreachable by construction.
    expect(picker(container)!.disabled).toBe(true);
    expect(ipc.setNoteInstrument).not.toHaveBeenCalled();
  });

  it("surfaces a backend rejection instead of failing silently", async () => {
    vi.mocked(ipc.setNoteInstrument).mockRejectedValue(
      "voice-overlap: notes with different voices would overlap on DAC at ticks 0-240",
    );
    const { container } = render(roll(sel("dac")));
    await flush();
    selectAll();
    await flush();

    fireEvent.change(picker(container)!, { target: { value: "dac-hat" } });
    await flush();

    expect(container.textContent).toContain("voice-overlap");
    // A rejected edit must not be announced to the transport.
    expect(ipc.reloadSequence).not.toHaveBeenCalled();
  });

  it("points a library drop at the picker instead of a DAC library voice that cannot exist", async () => {
    const { container } = render(roll(sel("dac")));
    await flush();

    // DOM order: ruler canvas, keys canvas, note canvas, velocity lane canvas.
    const noteCanvas = container.querySelectorAll("canvas")[2];
    const payload = JSON.stringify({ hash: "sha256:abc", kind: "fm" });
    fireEvent.drop(noteCanvas, {
      dataTransfer: {
        types: [library.LIBRARY_DRAG_TYPE, `${library.LIBRARY_DRAG_TYPE}-fm`],
        getData: (t: string) => (t === library.LIBRARY_DRAG_TYPE ? payload : ""),
      },
    });
    await flush();

    expect(container.textContent).toContain("Sample picker");
    expect(container.textContent).not.toContain("Only DAC voices");
  });

  it("does not carry the DAC bank into a melodic region opened after it", async () => {
    const { container, rerender } = render(roll(sel("dac")));
    await flush();
    expect(picker(container)).not.toBeNull();

    rerender(roll(sel("fm")));
    await flush();
    expect(picker(container)).toBeNull();
  });

  it("ignores a bank reply that lands after the user moved to another region", async () => {
    // Replies can land out of order across a region switch (the class
    // `e01f6d1` swept). A reply issued for the region the user has already
    // left must not repopulate the picker of the one now open.
    let resolveStale!: (v: DacInstrument[]) => void;
    const stale = new Promise<DacInstrument[]>((r) => { resolveStale = r; });
    vi.mocked(ipc.listDacInstruments)
      .mockReturnValueOnce(stale)
      .mockResolvedValue([KICK, HAT]);

    const { container, rerender } = render(roll(sel("dac")));
    await flush();
    // region-1's fetch is still in flight; the user opens region-3, whose
    // fetch resolves first.
    rerender(roll(sel("dac2")));
    await flush();
    expect(optionNames(container)).toContain("Hat.wav");

    // region-1's reply lands last, carrying a bank that no longer describes
    // anything the user is looking at.
    resolveStale([KICK]);
    await flush();
    expect(optionNames(container)).toContain("Hat.wav");
  });
});
