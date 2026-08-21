import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ArrangementView } from "./ArrangementView";
import * as ipc from "../api/ipc";
import { ticksPerBar } from "../utils/grid";
import type { SongMetadata, Track } from "../types/model";

vi.mock("../api/ipc");
vi.mock("../api/library");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));

// jsdom has no ResizeObserver; TimelineRuler observes its container.
vi.stubGlobal(
  "ResizeObserver",
  class {
    observe() {}
    unobserve() {}
    disconnect() {}
  },
);

// Deliberately non-4/4 so a hardcoded 4-beats-per-bar region default
// (ticksPerBeat * 4 = 1920 instead of 1440) would fail the assertions.
const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [3, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

const fmTrack: Track = {
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

function renderView(overrides: Partial<Parameters<typeof ArrangementView>[0]> = {}) {
  return render(
    <ArrangementView
      projectMeta={meta}
      playing={false}
      onSelectRegions={vi.fn()}
      selectedRegions={[]}
      onSelectInstrument={vi.fn()}
      selectedInstrument={null}
      {...overrides}
    />,
  );
}

describe("ArrangementView", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.listTracks).mockResolvedValue([fmTrack]);
    vi.mocked(ipc.addRegion).mockResolvedValue("region-new");
    vi.mocked(ipc.getDriverInfo).mockResolvedValue({
      id: "flamedriver",
      name: "Flamedriver (S3K)",
      layout: {
        fmChannels: [{ index: 0, name: "FM1", supportsSpecialMode: false }],
        psgChannels: [{ index: 0, name: "PSG1", isNoise: false }],
        dacChannels: [{ index: 0, name: "DAC", }],
      },
      features: [],
    });
  });

  it("offers + Track and instrument buttons named for what they add", async () => {
    renderView();
    expect(await screen.findByText("+ Track")).toBeInTheDocument();
    expect(screen.getByText("+ FM Patch")).toBeInTheDocument();
    expect(screen.getByText("+ PSG Env")).toBeInTheDocument();
    expect(screen.getByText("+ DAC Sample")).toBeInTheDocument();
  });

  it("+ Track opens the AddTrackDialog", async () => {
    renderView();
    fireEvent.click(await screen.findByText("+ Track"));
    expect(await screen.findByText("Add Track")).toBeInTheDocument();
  });

  it("empty-lane double-click adds a one-bar region, selects it, reloads the sequence", async () => {
    const onSelectRegions = vi.fn();
    const { container } = renderView({ onSelectRegions });

    // The canvas hit-tests against the fetched track list — wait for it.
    await screen.findByText("Bass Lane");
    // Two canvases render: the TimelineRuler's first, then the timeline lanes.
    const canvases = container.querySelectorAll("canvas");
    const canvas = canvases[canvases.length - 1];
    expect(canvas).toBeDefined();

    // jsdom rects are all zeros, so clientX/clientY are canvas coordinates;
    // x=10 falls in bar 0, y=10 falls in lane 0.
    fireEvent.doubleClick(canvas as HTMLCanvasElement, { clientX: 10, clientY: 10 });

    const oneBar = ticksPerBar(meta);
    await waitFor(() =>
      expect(vi.mocked(ipc.addRegion)).toHaveBeenCalledWith(fmTrack.id, 0, oneBar),
    );
    await waitFor(() => expect(vi.mocked(ipc.reloadSequence)).toHaveBeenCalled());
    await waitFor(() =>
      expect(onSelectRegions).toHaveBeenCalledWith([
        expect.objectContaining({
          trackId: fmTrack.id,
          regionId: "region-new",
          channelType: "fm",
          startTick: 0,
          durationTicks: oneBar,
        }),
      ]),
    );
  });
});
