import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { TrackHeader } from "./TrackHeader";
import * as ipc from "../api/ipc";
import * as library from "../api/library";
import { whenReloadsSettled, resetLiveReloadForTests } from "../utils/liveReload";
import type { Track } from "../types/model";

vi.mock("../api/ipc");
// Keep the real LIBRARY_DRAG_TYPE constant (the drop handler keys off it);
// mock only the async API surface.
vi.mock("../api/library", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api/library")>()),
  libraryAssignToTrack: vi.fn(),
  libraryGetEntry: vi.fn(),
}));

const track: Track = {
  id: "track-1",
  name: "Bass Lane",
  channel: { Fm: 0 },
  instrumentId: null,
  regions: [],
  muted: false,
  solo: false,
  volume: 100,
  pan: "Center",
  pitchOffset: 0,
};

const handlers = {
  selected: false,
  level: 0,
  onUpdate: vi.fn(),
  onClick: vi.fn(),
};

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ipc.updateTrack).mockResolvedValue(undefined);
  vi.mocked(ipc.deleteTrack).mockResolvedValue(undefined);
  vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("TrackHeader rename", () => {
  it("double-click the name, type, Enter commits via updateTrack", async () => {
    render(<TrackHeader track={track} {...handlers} />);

    fireEvent.doubleClick(screen.getByText("Bass Lane"));
    const input = screen.getByDisplayValue("Bass Lane");
    fireEvent.change(input, { target: { value: "Lead Lane" } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(ipc.updateTrack).toHaveBeenCalledWith(
      track.id, "Lead Lane", track.channel, track.instrumentId,
      track.muted, track.solo, track.volume, track.pan, track.pitchOffset,
    ));
    await waitFor(() => expect(handlers.onUpdate).toHaveBeenCalled());
  });

  it("Escape cancels without calling updateTrack", async () => {
    render(<TrackHeader track={track} {...handlers} />);

    fireEvent.doubleClick(screen.getByText("Bass Lane"));
    const input = screen.getByDisplayValue("Bass Lane");
    fireEvent.change(input, { target: { value: "Nope" } });
    fireEvent.keyDown(input, { key: "Escape" });

    expect(screen.getByText("Bass Lane")).toBeInTheDocument();
    expect(ipc.updateTrack).not.toHaveBeenCalled();
  });

  it("an unchanged or empty name commits nothing", async () => {
    render(<TrackHeader track={track} {...handlers} />);

    fireEvent.doubleClick(screen.getByText("Bass Lane"));
    const input = screen.getByDisplayValue("Bass Lane");
    fireEvent.change(input, { target: { value: "   " } });
    fireEvent.keyDown(input, { key: "Enter" });

    await waitFor(() => expect(screen.getByText("Bass Lane")).toBeInTheDocument());
    expect(ipc.updateTrack).not.toHaveBeenCalled();
  });
});

describe("TrackHeader silent-lane cue (F2)", () => {
  // Seeded lanes carry instrumentId: null and build_snapshot silently drops
  // their notes — the header must say so instead of looking bound.

  it("marks an instrument-less lane with a 'no voice' cue and a how-to-fix tooltip", () => {
    render(<TrackHeader track={track} {...handlers} />);

    const marker = screen.getByText("no voice");
    expect(marker).toBeInTheDocument();
    // The tooltip must state the consequence and the fix.
    expect(marker.getAttribute("title")).toMatch(/won't sound/);
    expect(marker.getAttribute("title")).toMatch(/Library/);
  });

  it("shows no cue once an instrument is bound", () => {
    render(<TrackHeader track={{ ...track, instrumentId: "inst-1" }} {...handlers} />);
    expect(screen.queryByText("no voice")).toBeNull();
  });
});

describe("TrackHeader delete", () => {
  it("confirmed delete calls deleteTrack and reloads the sequence", async () => {
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(true));
    render(<TrackHeader track={track} {...handlers} />);

    fireEvent.click(screen.getByTitle("Delete track"));

    await waitFor(() => expect(ipc.deleteTrack).toHaveBeenCalledWith(track.id));
    await waitFor(() => expect(ipc.reloadSequence).toHaveBeenCalled());
    await waitFor(() => expect(handlers.onUpdate).toHaveBeenCalled());
  });

  it("cancelled confirm deletes nothing", async () => {
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(false));
    render(<TrackHeader track={track} {...handlers} />);

    fireEvent.click(screen.getByTitle("Delete track"));

    expect(ipc.deleteTrack).not.toHaveBeenCalled();
  });
});

describe("TrackHeader voice drop wipes require confirmation", () => {
  // The backend clear of per-region/per-note voices on assign is deliberate
  // (importer-stamped ids take precedence, so a swap without the clear is
  // inaudible) — but one header drag must not silently destroy an imported
  // song's mid-track voice changes.

  // Two overrides: one region-level, one note-level.
  const trackWithOverrides: Track = {
    ...track,
    regions: [
      {
        id: "r1",
        startTick: 0,
        durationTicks: 960,
        instrumentId: "voice-region",
        notes: [
          { tick: 0, pitch: 60, velocity: 100, durationTicks: 240, instrumentId: "voice-note" },
          { tick: 240, pitch: 62, velocity: 100, durationTicks: 240 },
        ],
      },
    ],
  };

  function dropLibraryEntry(el: Element, kind = "fm", hash = "hash-1") {
    fireEvent.drop(el, {
      dataTransfer: {
        types: [library.LIBRARY_DRAG_TYPE, `${library.LIBRARY_DRAG_TYPE}-${kind}`],
        getData: (t: string) =>
          t === library.LIBRARY_DRAG_TYPE ? JSON.stringify({ hash, kind }) : "",
      },
    });
  }

  beforeEach(() => {
    vi.mocked(library.libraryAssignToTrack).mockResolvedValue("inst-1");
    vi.mocked(library.libraryGetEntry).mockResolvedValue({
      name: "Cool Lead",
      game: "Test Game",
      tags: [],
      instrument: {} as never,
    });
  });

  it("asks with the override count and the voice name; confirm proceeds", async () => {
    const confirmFn = vi.fn().mockReturnValue(true);
    vi.stubGlobal("confirm", confirmFn);
    const { container } = render(<TrackHeader track={trackWithOverrides} {...handlers} />);

    dropLibraryEntry(container.firstChild as Element);

    await waitFor(() =>
      expect(library.libraryAssignToTrack).toHaveBeenCalledWith(trackWithOverrides.id, "hash-1"),
    );
    expect(confirmFn).toHaveBeenCalledTimes(1);
    const msg = confirmFn.mock.calls[0][0] as string;
    expect(msg).toContain("2 mid-song voice changes");
    expect(msg).toContain("Cool Lead");
  });

  it("cancel is a no-op: no assign, no reload", async () => {
    vi.stubGlobal("confirm", vi.fn().mockReturnValue(false));
    const { container } = render(<TrackHeader track={trackWithOverrides} {...handlers} />);

    dropLibraryEntry(container.firstChild as Element);

    // Give the async handler a beat to (wrongly) call through.
    await new Promise((r) => setTimeout(r, 0));
    expect(library.libraryAssignToTrack).not.toHaveBeenCalled();
    expect(ipc.reloadSequence).not.toHaveBeenCalled();
    expect(handlers.onUpdate).not.toHaveBeenCalled();
  });

  it("a track with no overrides assigns without any prompt", async () => {
    const confirmFn = vi.fn().mockReturnValue(false);
    vi.stubGlobal("confirm", confirmFn);
    const { container } = render(<TrackHeader track={track} {...handlers} />);

    dropLibraryEntry(container.firstChild as Element);

    await waitFor(() =>
      expect(library.libraryAssignToTrack).toHaveBeenCalledWith(track.id, "hash-1"),
    );
    expect(confirmFn).not.toHaveBeenCalled();
  });
});

// Audit F13: the volume slider fires updateTrack + reloadSequence on EVERY
// input event of a drag. The reload no longer silences sounding notes (proved
// by the rendered-audio tests in src-tauri/src/audio/live_edit_audibility.rs),
// but one snapshot rebuild per pixel is still work the backend cannot keep up
// with, so the reloads coalesce.
describe("TrackHeader volume ride (F13)", () => {
  beforeEach(() => {
    resetLiveReloadForTests();
  });

  function volumeSlider() {
    return document.querySelector('input[type="range"]') as HTMLInputElement;
  }

  it("a volume change commits the new value and reloads the running sequence", async () => {
    render(<TrackHeader track={track} {...handlers} />);

    fireEvent.change(volumeSlider(), { target: { value: "80" } });

    await waitFor(() => expect(ipc.updateTrack).toHaveBeenCalledWith(
      track.id, track.name, track.channel, track.instrumentId,
      track.muted, track.solo, 80, track.pan, track.pitchOffset,
    ));
    await whenReloadsSettled();
    expect(ipc.reloadSequence).toHaveBeenCalled();
  });

  it("a whole drag issues far fewer reloads than input events", async () => {
    // Hold every reload open for the duration of the drag, the way a real
    // backend rebuild outlasts the next mousemove.
    const outstanding: Array<() => void> = [];
    vi.mocked(ipc.reloadSequence).mockImplementation(
      () => new Promise<void>((resolve) => { outstanding.push(resolve); }),
    );
    render(<TrackHeader track={track} {...handlers} />);

    const slider = volumeSlider();
    // Start below the current value (100): setting a range input to the value
    // it already holds fires no change event.
    for (let v = 99; v >= 80; v--) {
      fireEvent.change(slider, { target: { value: String(v) } });
    }
    // Every input event must still commit — coalescing is about the reload,
    // never about dropping the user's edits.
    await waitFor(() => expect(ipc.updateTrack).toHaveBeenCalledTimes(20));
    expect(ipc.reloadSequence).toHaveBeenCalledTimes(1);

    // Drag over: let the in-flight reload and its trailing successor complete.
    vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
    while (outstanding.length) outstanding.shift()!();
    await whenReloadsSettled();
    // Exactly one trailing reload, carrying the value the drag ended on —
    // 20 input events cost 2 snapshot rebuilds, not 20.
    expect(ipc.reloadSequence).toHaveBeenCalledTimes(2);
  });
});
