import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as lib from "./api/library";
import { resetClipboardForTest } from "./utils/clipboard";
import {
  VIEW_STATE_KEY,
  VIEW_STATE_WRITE_DELAY_MS,
  getViewState,
  patchViewState,
} from "./utils/viewState";
import type { SongMetadata, Track } from "./types/model";

vi.mock("./api/ipc");
vi.mock("./api/library");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue("/tmp/projects"),
  save: vi.fn().mockResolvedValue(null),
  ask: vi.fn().mockResolvedValue(true),
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

const meta: SongMetadata = {
  name: "Test Song",
  tempo: 120,
  timeSignature: [4, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

/** The directory the mocked Open dialog answers with — the view-state key. */
const PROJECT_PATH = "/tmp/projects";
/** One bar; also the whole extent of the fixture project's only region. */
const ONE_BAR = meta.ticksPerBeat * meta.timeSignature[0];

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
        durationTicks: ONE_BAR,
        notes: [{ tick: 0, pitch: 60, velocity: 100, durationTicks: 240 }],
      },
    ],
    muted: false,
    solo: false,
    volume: 100,
    pan: "Center",
    pitchOffset: 0,
  };
}

/** Render App on the welcome screen and open the mocked project. */
async function openProject() {
  const utils = render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: "Open Project" }));
  await waitFor(() => expect(ipc.openProject).toHaveBeenCalled());
  // Project open: the arrangement shows the track header.
  await screen.findByText("Bass Lane");
  return utils;
}

/**
 * Wait out the write-through debounce. This is genuinely timed work (a
 * `setTimeout` in the writing effect), which is what `waitFor`-free direct
 * assertion cannot substitute for — unlike a chain of mocked promises.
 */
async function flushViewStateWrite() {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, VIEW_STATE_WRITE_DELAY_MS + 50));
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  localStorage.clear();
  resetClipboardForTest();
  vi.mocked(ipc.listDrivers).mockResolvedValue([
    { id: "flamedriver", name: "Flamedriver (S3K)" },
  ]);
  vi.mocked(ipc.createProject).mockResolvedValue(undefined);
  vi.mocked(ipc.getProjectInfo).mockResolvedValue(meta);
  vi.mocked(ipc.closeProject).mockResolvedValue(undefined);
  vi.mocked(ipc.openProject).mockResolvedValue({
    metadata: meta,
    tracks: [makeTrack()],
    instruments: { fm: [], psg: [], dac: [] },
  });
  vi.mocked(ipc.listTracks).mockResolvedValue([makeTrack()]);
  vi.mocked(ipc.transportSetLoop).mockResolvedValue(undefined);
  vi.mocked(ipc.transportClearLoop).mockResolvedValue(undefined);
  vi.mocked(ipc.getPlaybackState).mockResolvedValue({
    playing: false,
    tick: 0,
    loopStart: null,
    loopEnd: null,
    channelLevels: [],
  });
  vi.mocked(ipc.getUndoState).mockResolvedValue({
    canUndo: false,
    canRedo: false,
    dirty: false,
  });
  vi.mocked(lib.libraryList).mockResolvedValue([]);
  vi.mocked(lib.libraryGames).mockResolvedValue([]);
  vi.mocked(lib.libraryWarnings).mockResolvedValue([]);
});

describe("reopening a project restores where the user was sitting (F15)", () => {
  it("reopens the region that was open in the piano roll", async () => {
    patchViewState(PROJECT_PATH, { openRegion: { trackId: "track-1", regionId: "region-1" } });

    await openProject();

    expect(await screen.findByText("Piano Roll")).toBeInTheDocument();
  });

  it("re-arms the preview loop through the ordinary transport command", async () => {
    patchViewState(PROJECT_PATH, { loop: { start: 0, end: 960, enabled: true } });

    await openProject();

    expect(ipc.transportSetLoop).toHaveBeenCalledWith(0, 960);
  });

  it("remembers a loop range without arming it when it was disarmed", async () => {
    patchViewState(PROJECT_PATH, { loop: { start: 0, end: 960, enabled: false } });

    await openProject();

    expect(ipc.transportSetLoop).not.toHaveBeenCalled();
    // The range is still the one the L key re-arms: pressing L must reuse it
    // rather than falling back to one bar at the cursor.
    fireEvent.keyDown(window, { key: "l" });
    await act(async () => {});
    expect(ipc.transportSetLoop).toHaveBeenCalledWith(0, 960);
  });

  it("records the loop the user armed, under this project's path", async () => {
    await openProject();

    fireEvent.keyDown(window, { key: "l" });
    await act(async () => {});
    await flushViewStateWrite();

    expect(getViewState(PROJECT_PATH).loop).toEqual({ start: 0, end: ONE_BAR, enabled: true });
  });

  it("does not restore one project's view into another project", async () => {
    patchViewState("/tmp/somewhere-else", { loop: { start: 0, end: 960, enabled: true } });

    await openProject();

    expect(ipc.transportSetLoop).not.toHaveBeenCalled();
  });
});

// A project can be edited by another session, another machine, or by hand
// between two sittings. Restoration must take what still validates and drop
// the rest in silence — never throw on the way into a render.
describe("restoration survives a project edited elsewhere", () => {
  it("opens the project normally when the remembered region's track is gone", async () => {
    patchViewState(PROJECT_PATH, { openRegion: { trackId: "deleted-track", regionId: "region-1" } });

    await openProject();

    expect(screen.queryByText("Piano Roll")).not.toBeInTheDocument();
    // The project itself opened: the arrangement is live.
    expect(screen.getByText("Bass Lane")).toBeInTheDocument();
  });

  it("opens the project normally when the remembered region itself is gone", async () => {
    patchViewState(PROJECT_PATH, { openRegion: { trackId: "track-1", regionId: "deleted-region" } });

    await openProject();

    expect(screen.queryByText("Piano Roll")).not.toBeInTheDocument();
    expect(screen.getByText("Bass Lane")).toBeInTheDocument();
  });

  it("refuses to re-arm a loop whose content was deleted", async () => {
    // The fixture project ends at ONE_BAR; this range starts past everything
    // left in it, so re-arming it would loop silence.
    patchViewState(PROJECT_PATH, {
      loop: { start: ONE_BAR * 4, end: ONE_BAR * 5, enabled: true },
    });

    await openProject();

    expect(ipc.transportSetLoop).not.toHaveBeenCalled();
    expect(screen.getByText("Bass Lane")).toBeInTheDocument();
  });

  it("opens the project when the stored record is corrupt beyond parsing", async () => {
    localStorage.setItem(VIEW_STATE_KEY, "{not json[");
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});

    await openProject();

    expect(screen.getByText("Bass Lane")).toBeInTheDocument();
    expect(ipc.transportSetLoop).not.toHaveBeenCalled();
    // Corruption is absorbed by the STORE, not by the restore's outer catch:
    // the outer catch is the last line of defence, and a test that cannot
    // tell the two apart would go green on a store that throws every read.
    expect(
      warn.mock.calls.filter((c) => String(c[0]).startsWith("view-state restore skipped")),
    ).toEqual([]);
    warn.mockRestore();
  });

  it("opens the project when listing its tracks fails during restore", async () => {
    patchViewState(PROJECT_PATH, {
      openRegion: { trackId: "track-1", regionId: "region-1" },
      loop: { start: 0, end: 960, enabled: true },
    });
    // First call is the restore's; the arrangement's own polling then works.
    vi.mocked(ipc.listTracks).mockRejectedValueOnce(new Error("backend gone"));

    await openProject();

    expect(screen.getByText("Bass Lane")).toBeInTheDocument();
    expect(screen.queryByText("Piano Roll")).not.toBeInTheDocument();
    expect(ipc.transportSetLoop).not.toHaveBeenCalled();
  });
});

describe("a new project does not inherit the view filed under its directory", () => {
  it("forgets the previous occupant's remembered region and loop", async () => {
    // NewProjectDialog builds `${location}/${sanitized name}`.
    const newPath = "/tmp/projects/Test_Song";
    patchViewState(newPath, {
      openRegion: { trackId: "track-1", regionId: "region-1" },
      loop: { start: 0, end: 960, enabled: true },
    });

    render(<App />);
    fireEvent.click(await screen.findByRole("button", { name: "New Project" }));
    fireEvent.change(await screen.findByPlaceholderText("My Song"), {
      target: { value: "Test Song" },
    });
    fireEvent.change(screen.getByPlaceholderText("/path/to/projects"), {
      target: { value: "/tmp/projects" },
    });
    fireEvent.click(screen.getByText("Create"));
    await waitFor(() => expect(ipc.createProject).toHaveBeenCalled());
    await act(async () => {});

    expect(screen.queryByText("Piano Roll")).not.toBeInTheDocument();
    expect(ipc.transportSetLoop).not.toHaveBeenCalled();
    expect(getViewState(newPath).openRegion ?? null).toBeNull();
  });
});
