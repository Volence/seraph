import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as lib from "./api/library";
import type { SongMetadata, Track } from "./types/model";

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
  vi.mocked(lib.libraryList).mockResolvedValue([]);
  vi.mocked(lib.libraryGames).mockResolvedValue([]);
  vi.mocked(lib.libraryWarnings).mockResolvedValue([]);
});

afterEach(() => {
  vi.restoreAllMocks();
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
