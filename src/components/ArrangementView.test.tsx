import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { ArrangementView } from "./ArrangementView";
import * as ipc from "../api/ipc";
import { ticksPerBar } from "../utils/grid";
import { setPianoRollNoteSelectionActive } from "../utils/noteSelection";
import { resetClipboardForTest } from "../utils/clipboard";
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
      seekTick={0}
      onSeek={vi.fn()}
      previewLoop={null}
      loopEnabled={false}
      onPreviewLoopSet={vi.fn()}
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

  describe("region edits reload the sequence so playback hears them (G30)", () => {
    const oneBar = ticksPerBar(meta);
    const region = { id: "region-1", startTick: 0, durationTicks: oneBar, notes: [] };
    const trackWithRegion: Track = { ...fmTrack, regions: [region] };
    const selected = {
      trackId: fmTrack.id,
      trackName: fmTrack.name,
      regionId: region.id,
      channelType: "fm" as const,
      startTick: 0,
      durationTicks: oneBar,
    };

    async function timelineCanvas(container: HTMLElement) {
      await screen.findByText("Bass Lane");
      const canvases = container.querySelectorAll("canvas");
      return canvases[canvases.length - 1] as HTMLCanvasElement;
    }

    it("after deleting a region via Delete", async () => {
      vi.mocked(ipc.listTracks).mockResolvedValue([trackWithRegion]);
      renderView({ selectedRegions: [selected] });
      await screen.findByText("Bass Lane");

      fireEvent.keyDown(window, { key: "Delete" });

      await waitFor(() =>
        expect(ipc.deleteRegion).toHaveBeenCalledWith(fmTrack.id, region.id),
      );
      await waitFor(() => expect(ipc.reloadSequence).toHaveBeenCalled());
    });

    it("after a region move (committed on mouse release)", async () => {
      vi.mocked(ipc.listTracks).mockResolvedValue([trackWithRegion]);
      const { container } = renderView();
      const canvas = await timelineCanvas(container);

      // Region spans x 0..oneBar/ticksPerPixel (75px at the default zoom).
      // Grab its middle and drag one bar to the right.
      fireEvent.mouseDown(canvas, { clientX: 30, clientY: 10 });
      fireEvent.mouseMove(window, { clientX: 130, clientY: 10 });
      fireEvent.mouseUp(window, { clientX: 130, clientY: 10 });

      await waitFor(() => expect(ipc.moveRegion).toHaveBeenCalled());
      await waitFor(() => expect(ipc.reloadSequence).toHaveBeenCalled());
    });

    it("after a region resize (committed on mouse release)", async () => {
      vi.mocked(ipc.listTracks).mockResolvedValue([trackWithRegion]);
      const { container } = renderView();
      const canvas = await timelineCanvas(container);

      // Right edge sits at 75px; press within the 8px edge threshold.
      fireEvent.mouseDown(canvas, { clientX: 71, clientY: 10 });
      fireEvent.mouseMove(window, { clientX: 150, clientY: 10 });
      fireEvent.mouseUp(window, { clientX: 150, clientY: 10 });

      await waitFor(() => expect(ipc.updateRegion).toHaveBeenCalled());
      await waitFor(() => expect(ipc.reloadSequence).toHaveBeenCalled());
    });
  });

  describe("region duplicate + clipboard", () => {
    const oneBar = ticksPerBar(meta);
    const notes = [{ tick: 0, pitch: 60, velocity: 100, durationTicks: 240 }];
    const region = { id: "region-1", startTick: 0, durationTicks: oneBar, notes };
    const trackWithRegion: Track = { ...fmTrack, regions: [region] };
    const selected = {
      trackId: fmTrack.id,
      trackName: fmTrack.name,
      regionId: region.id,
      channelType: "fm" as const,
      startTick: 0,
      durationTicks: oneBar,
    };

    beforeEach(() => {
      resetClipboardForTest();
      setPianoRollNoteSelectionActive(false);
      vi.mocked(ipc.listTracks).mockResolvedValue([trackWithRegion]);
      vi.mocked(ipc.duplicateRegion).mockResolvedValue("region-dup");
    });

    it("Ctrl+D duplicates each selected region immediately after itself", async () => {
      const onSelectRegions = vi.fn();
      renderView({ selectedRegions: [selected], onSelectRegions });
      await screen.findByText("Bass Lane");

      fireEvent.keyDown(window, { key: "d", ctrlKey: true });

      await waitFor(() =>
        expect(ipc.duplicateRegion).toHaveBeenCalledWith(fmTrack.id, region.id, oneBar),
      );
      await waitFor(() => expect(ipc.reloadSequence).toHaveBeenCalled());
      // The duplicates become the selection.
      await waitFor(() =>
        expect(onSelectRegions).toHaveBeenCalledWith([
          expect.objectContaining({ regionId: "region-dup", startTick: oneBar }),
        ]),
      );
      // One gesture = one undo step.
      expect(ipc.beginUndoGroup).toHaveBeenCalledTimes(1);
      expect(ipc.endUndoGroup).toHaveBeenCalledTimes(1);
    });

    it("Ctrl+D defers to the piano roll while it owns a note selection (G1 mirror)", async () => {
      renderView({ selectedRegions: [selected] });
      await screen.findByText("Bass Lane");
      setPianoRollNoteSelectionActive(true);

      fireEvent.keyDown(window, { key: "d", ctrlKey: true });

      await new Promise((r) => setTimeout(r, 20));
      expect(ipc.duplicateRegion).not.toHaveBeenCalled();
    });

    it("Ctrl+C then Ctrl+V pastes at the bar-snapped seek cursor", async () => {
      const onSelectRegions = vi.fn();
      // seekTick 2000 with 1440-tick bars snaps down to 1440.
      renderView({ selectedRegions: [selected], onSelectRegions, seekTick: 2000 });
      await screen.findByText("Bass Lane");

      fireEvent.keyDown(window, { key: "c", ctrlKey: true });
      fireEvent.keyDown(window, { key: "v", ctrlKey: true });

      const snapped = Math.floor(2000 / oneBar) * oneBar;
      expect(snapped).toBe(oneBar); // the case must actually exercise snapping
      await waitFor(() =>
        expect(ipc.duplicateRegion).toHaveBeenCalledWith(fmTrack.id, region.id, snapped),
      );
      await waitFor(() => expect(ipc.reloadSequence).toHaveBeenCalled());
      await waitFor(() =>
        expect(onSelectRegions).toHaveBeenCalledWith([
          expect.objectContaining({ regionId: "region-dup", startTick: snapped }),
        ]),
      );
    });

    it("region copy defers to the piano roll while it owns a note selection", async () => {
      renderView({ selectedRegions: [selected] });
      await screen.findByText("Bass Lane");
      setPianoRollNoteSelectionActive(true);
      fireEvent.keyDown(window, { key: "c", ctrlKey: true });
      setPianoRollNoteSelectionActive(false);
      fireEvent.keyDown(window, { key: "v", ctrlKey: true });
      await new Promise((r) => setTimeout(r, 20));
      // Nothing was copied, so nothing pastes.
      expect(ipc.duplicateRegion).not.toHaveBeenCalled();
      expect(ipc.addRegion).not.toHaveBeenCalled();
    });

    it("paste replays the copied payload when the source region is gone", async () => {
      renderView({ selectedRegions: [selected], seekTick: 0 });
      await screen.findByText("Bass Lane");

      fireEvent.keyDown(window, { key: "c", ctrlKey: true });
      // The source region was deleted after the copy: server-side clone fails.
      vi.mocked(ipc.duplicateRegion).mockRejectedValue("region not found");
      fireEvent.keyDown(window, { key: "v", ctrlKey: true });

      await waitFor(() =>
        expect(ipc.addRegion).toHaveBeenCalledWith(fmTrack.id, 0, oneBar),
      );
      await waitFor(() =>
        expect(ipc.addNote).toHaveBeenCalledWith(fmTrack.id, "region-new", 0, 60, 100, 240),
      );
    });
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
