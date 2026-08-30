import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as lib from "./api/library";
import type { SongMetadata, Track } from "./types/model";

vi.mock("./api/ipc");
vi.mock("./api/library");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue("/tmp/projects"),
  save: vi.fn().mockResolvedValue(null),
}));
// App's close-confirm effect reaches for the real Tauri window otherwise, which
// throws inside Tauri's own code here and printed a stack per test in this file.
// Same shape as App.test.tsx's; the guard's behaviour is tested in
// App.closeGuard.test.tsx, this only keeps the effect quiet and inert.
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onCloseRequested: vi.fn().mockResolvedValue(() => {}),
    destroy: vi.fn(),
  }),
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

// Deliberately non-4/4 so hardcoded 4-beats-per-bar assumptions would fail.
const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [3, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

const oneBar = meta.ticksPerBeat * meta.timeSignature[0]; // 1440

function makeTrack(): Track {
  return {
    id: "track-1",
    name: "Bass Lane",
    channel: { Fm: 0 },
    instrumentId: null,
    regions: [
      {
        id: "region-1",
        startTick: 0,
        durationTicks: oneBar,
        notes: [
          { tick: 0, pitch: 60, velocity: 100, durationTicks: 240 },
          { tick: 480, pitch: 64, velocity: 100, durationTicks: 240 },
        ],
      },
    ],
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
}

/** Renders App and drives the NewProjectDialog until a project is open. */
async function openProject() {
  const utils = render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: "New Project" }));
  fireEvent.change(await screen.findByPlaceholderText("My Song"), {
    target: { value: "Test Song" },
  });
  fireEvent.click(screen.getByText("Browse"));
  // Browse resolves the mocked directory into the (read-only) location input.
  await waitFor(() =>
    expect(screen.getByPlaceholderText("/path/to/projects")).toHaveValue("/tmp/projects"),
  );
  fireEvent.click(screen.getByText("Create"));
  await waitFor(() => expect(ipc.createProject).toHaveBeenCalled());
  // Project open: the arrangement shows the track header.
  await screen.findByText("Bass Lane");
  return utils;
}

/** Double-clicks the timeline region so it opens in the piano roll. */
async function openRegionInPianoRoll(container: HTMLElement) {
  // Canvas order while no piano roll is open: SpectrumAnalyzer, TimelineRuler,
  // TimelineCanvas (last). jsdom rects are all zeros so clientX/clientY are
  // canvas coordinates; the region spans x 0..durationTicks/ticksPerPixel in
  // lane 0.
  const canvases = container.querySelectorAll("canvas");
  const timeline = canvases[canvases.length - 1] as HTMLCanvasElement;
  fireEvent.doubleClick(timeline, { clientX: 30, clientY: 10 });
  await screen.findByText("Piano Roll");
  // Wait for the piano roll to fetch its notes (its own listTracks round-trip),
  // then flush state so the keydown handlers see the loaded notes.
  await act(async () => {});
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ipc.listDrivers).mockResolvedValue([
    { id: "flamedriver", name: "Flamedriver (S3K)" },
  ]);
  vi.mocked(ipc.createProject).mockResolvedValue(undefined);
  vi.mocked(ipc.getProjectInfo).mockResolvedValue(meta);
  vi.mocked(ipc.listTracks).mockResolvedValue([makeTrack()]);
  vi.mocked(ipc.getPlaybackState).mockResolvedValue({
    playing: false,
    tick: 0,
    loopStart: null,
    loopEnd: null,
    channelLevels: [],
  });
  // The undo/dirty feature (merged alongside) polls undo state on render —
  // an unmocked call crashes the App under test.
  vi.mocked(ipc.getUndoState).mockResolvedValue({
    canUndo: false,
    canRedo: false,
    dirty: false,
  });
  vi.mocked(lib.libraryList).mockResolvedValue([]);
  vi.mocked(lib.libraryGames).mockResolvedValue([]);
  vi.mocked(lib.libraryWarnings).mockResolvedValue([]);
});

describe("global shortcuts are inert while typing in text inputs (G2)", () => {
  it("Space in a text input does not toggle playback", async () => {
    await openProject();
    const search = screen.getByPlaceholderText("Search name, game, tag…");
    fireEvent.keyDown(search, { key: " " });
    await act(async () => {});
    expect(ipc.transportPlay).not.toHaveBeenCalled();
    expect(ipc.transportStop).not.toHaveBeenCalled();
  });

  it("'l' in a text input does not toggle the loop", async () => {
    await openProject();
    const search = screen.getByPlaceholderText("Search name, game, tag…");
    fireEvent.keyDown(search, { key: "l" });
    await act(async () => {});
    expect(ipc.transportSetLoop).not.toHaveBeenCalled();
    expect(ipc.transportClearLoop).not.toHaveBeenCalled();
  });

  it("Delete in a text input does not delete the selected region", async () => {
    const { container } = await openProject();
    await openRegionInPianoRoll(container);
    const search = screen.getByPlaceholderText("Search name, game, tag…");
    fireEvent.keyDown(search, { key: "Delete" });
    await act(async () => {});
    expect(ipc.deleteRegion).not.toHaveBeenCalled();
  });

  it("Space outside inputs still starts playback", async () => {
    await openProject();
    fireEvent.keyDown(window, { key: " " });
    await waitFor(() => expect(ipc.transportPlay).toHaveBeenCalled());
  });
});

describe("Delete scoping between piano roll notes and arrangement regions (G1)", () => {
  it("Delete with notes selected deletes the notes, NOT the enclosing region", async () => {
    const { container } = await openProject();
    await openRegionInPianoRoll(container);

    // Select all notes in the piano roll, then Delete.
    fireEvent.keyDown(window, { key: "a", ctrlKey: true });
    fireEvent.keyDown(window, { key: "Delete" });

    await waitFor(() => expect(ipc.deleteNote).toHaveBeenCalled());
    // Let the region-delete promise chain (if wrongly triggered) land too.
    await act(async () => {});
    expect(ipc.deleteRegion).not.toHaveBeenCalled();
  });

  it("Delete with no note selection still deletes the selected region", async () => {
    const { container } = await openProject();
    await openRegionInPianoRoll(container);

    fireEvent.keyDown(window, { key: "Delete" });

    await waitFor(() =>
      expect(ipc.deleteRegion).toHaveBeenCalledWith("track-1", "region-1"),
    );
    expect(ipc.deleteNote).not.toHaveBeenCalled();
  });
});
