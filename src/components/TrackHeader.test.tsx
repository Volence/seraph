import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import { TrackHeader } from "./TrackHeader";
import * as ipc from "../api/ipc";
import type { Track } from "../types/model";

vi.mock("../api/ipc");
vi.mock("../api/library");

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
