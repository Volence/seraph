import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as lib from "./api/library";
import type { SongMetadata, Track } from "./types/model";
import { resetTransportMemory } from "./utils/transportMemory";

// Transport-focused App tests: the TimelineCanvas is stubbed to capture its
// props so the seek cursor (seekTick) is observable without canvas layout.
const captured = vi.hoisted(() => ({ props: [] as { seekTick: number }[] }));
vi.mock("./components/TimelineCanvas", () => ({
  TimelineCanvas: (props: { seekTick: number }) => {
    captured.props.push(props);
    return null;
  },
}));

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
  timeSignature: [3, 4],
  ticksPerBeat: 480,
  driverId: "flamedriver",
};

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

// The transport's real position, as get_playback_state reports it (its tick
// field is real even though other fields are hardcoded backend-side).
let transportTick = 0;

async function openProject() {
  const utils = render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: "New Project" }));
  fireEvent.change(await screen.findByPlaceholderText("My Song"), {
    target: { value: "Test Song" },
  });
  fireEvent.click(screen.getByText("Browse"));
  await waitFor(() =>
    expect(screen.getByPlaceholderText("/path/to/projects")).toHaveValue("/tmp/projects"),
  );
  fireEvent.click(screen.getByText("Create"));
  await waitFor(() => expect(ipc.createProject).toHaveBeenCalled());
  await screen.findByText("Bass Lane");
  await act(async () => {});
  return utils;
}

function lastSeekTick(): number {
  expect(captured.props.length).toBeGreaterThan(0);
  return captured.props[captured.props.length - 1].seekTick;
}

beforeEach(() => {
  vi.clearAllMocks();
  captured.props.length = 0;
  transportTick = 0;
  resetTransportMemory();
  vi.mocked(ipc.listDrivers).mockResolvedValue([
    { id: "flamedriver", name: "Flamedriver (S3K)" },
  ]);
  vi.mocked(ipc.createProject).mockResolvedValue(undefined);
  vi.mocked(ipc.getProjectInfo).mockResolvedValue(meta);
  vi.mocked(ipc.listTracks).mockResolvedValue([track]);
  vi.mocked(ipc.getPlaybackState).mockImplementation(async () => ({
    playing: false,
    tick: transportTick,
    loopStart: null,
    loopEnd: null,
    channelLevels: [],
  }));
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

afterEach(() => {
  vi.restoreAllMocks();
});

describe("Space stop semantics (G37 owner ruling)", () => {
  // Space = pause in place; Space again while stopped within
  // STOP_DOUBLE_TAP_MS returns the playhead to where the last playback
  // started (without playing); otherwise Space plays from current position.

  let nowValue = 0;
  function setNow(t: number) {
    nowValue = t;
  }

  async function playFromTick(startTick: number) {
    transportTick = startTick;
    fireEvent.keyDown(window, { key: " " }); // play (captures start tick)
    await waitFor(() => expect(ipc.transportPlay).toHaveBeenCalled());
    await act(async () => {});
  }

  beforeEach(() => {
    vi.spyOn(performance, "now").mockImplementation(() => nowValue);
    setNow(0);
  });

  it("double-tap Space after stop returns the playhead to the play start without playing", async () => {
    const { unmount } = await openProject();

    await playFromTick(960);
    transportTick = 2000; // playback advanced
    setNow(10_000);
    fireEvent.keyDown(window, { key: " " }); // stop (pauses in place)
    await waitFor(() => expect(ipc.transportStop).toHaveBeenCalled());
    await waitFor(() => expect(lastSeekTick()).toBe(2000)); // G29 sync landed

    setNow(10_200); // within the 400ms window
    fireEvent.keyDown(window, { key: " " });
    await waitFor(() => expect(ipc.transportSeek).toHaveBeenCalledWith(960));
    await waitFor(() => expect(lastSeekTick()).toBe(960));
    expect(ipc.transportPlay).toHaveBeenCalledTimes(1); // did NOT restart

    // The window is consumed: the next Space plays from the current position.
    setNow(10_300);
    transportTick = 960;
    fireEvent.keyDown(window, { key: " " });
    await waitFor(() => expect(ipc.transportPlay).toHaveBeenCalledTimes(2));
    unmount();
  });

  it("Space beyond the double-tap window plays from the current position", async () => {
    const { unmount } = await openProject();

    await playFromTick(960);
    transportTick = 2000;
    setNow(10_000);
    fireEvent.keyDown(window, { key: " " }); // stop
    await waitFor(() => expect(ipc.transportStop).toHaveBeenCalled());

    setNow(10_000 + 401); // just past the window
    fireEvent.keyDown(window, { key: " " });
    await waitFor(() => expect(ipc.transportPlay).toHaveBeenCalledTimes(2));
    expect(ipc.transportSeek).not.toHaveBeenCalledWith(960);
    unmount();
  });
});

describe("seek cursor sync on stop (G29)", () => {
  it("after Stop, the seek cursor moves to the transport's real tick", async () => {
    await openProject();

    fireEvent.keyDown(window, { key: " " }); // play
    await act(async () => {});

    // Playback advanced the real transport to tick 777 before the stop.
    transportTick = 777;
    fireEvent.keyDown(window, { key: " " }); // stop pauses in place
    await waitFor(() => expect(lastSeekTick()).toBe(777));
  });

  it("Home moves the cursor and the transport to 0", async () => {
    await openProject();

    fireEvent.keyDown(window, { key: " " }); // play
    await act(async () => {});
    transportTick = 777;
    fireEvent.keyDown(window, { key: " " }); // stop
    await waitFor(() => expect(lastSeekTick()).toBe(777));

    fireEvent.keyDown(window, { key: "Home" });
    await waitFor(() => expect(lastSeekTick()).toBe(0));
    expect(ipc.transportSeek).toHaveBeenCalledWith(0);
  });
});
