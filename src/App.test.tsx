import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, waitFor, act, screen } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as library from "./api/library";
import type { Song } from "./bindings";

vi.mock("./api/ipc");
vi.mock("./api/library");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue("/tmp/proj"),
  save: vi.fn(),
  ask: vi.fn().mockResolvedValue(true),
}));
vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    onCloseRequested: vi.fn().mockResolvedValue(() => {}),
    destroy: vi.fn(),
  }),
}));

const song: Song = {
  metadata: {
    name: "Test",
    tempo: 120,
    timeSignature: [4, 4],
    ticksPerBeat: 480,
    driverId: "flamedriver",
  },
  tracks: [],
  instruments: { fm: [], psg: [], dac: [] },
};

/** Render the App and open a project so song-edit shortcuts are live. */
async function renderOpenApp() {
  const utils = render(<App />);
  fireEvent.click(screen.getByText("Open"));
  await waitFor(() => expect(ipc.openProject).toHaveBeenCalled());
  await act(async () => {});
  return utils;
}

beforeEach(() => {
  vi.clearAllMocks();
  vi.mocked(ipc.openProject).mockResolvedValue(song);
  vi.mocked(ipc.listTracks).mockResolvedValue([]);
  vi.mocked(ipc.undo).mockResolvedValue([]);
  vi.mocked(ipc.redo).mockResolvedValue([]);
  vi.mocked(ipc.getUndoState).mockResolvedValue({
    canUndo: false,
    canRedo: false,
    dirty: false,
  });
  // The transport/playhead features (merged alongside undo) query playback
  // state on render — an unmocked call crashes the App under test.
  vi.mocked(ipc.getPlaybackState).mockResolvedValue({
    playing: false,
    tick: 0,
    loopStart: null,
    loopEnd: null,
    channelLevels: [],
  });
  vi.mocked(library.libraryGames).mockResolvedValue([]);
  vi.mocked(library.libraryList).mockResolvedValue([]);
  vi.mocked(library.libraryWarnings).mockResolvedValue([]);
});

describe("App undo/redo keyboard shortcuts", () => {
  it("Ctrl+Z calls undo and reloads the sequence", async () => {
    await renderOpenApp();
    fireEvent.keyDown(window, { key: "z", ctrlKey: true });
    await waitFor(() => expect(ipc.undo).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(ipc.reloadSequence).toHaveBeenCalled());
    expect(ipc.redo).not.toHaveBeenCalled();
  });

  it("Ctrl+Shift+Z calls redo", async () => {
    await renderOpenApp();
    fireEvent.keyDown(window, { key: "z", ctrlKey: true, shiftKey: true });
    await waitFor(() => expect(ipc.redo).toHaveBeenCalledTimes(1));
    expect(ipc.undo).not.toHaveBeenCalled();
  });

  it("Ctrl+Y calls redo", async () => {
    await renderOpenApp();
    fireEvent.keyDown(window, { key: "y", ctrlKey: true });
    await waitFor(() => expect(ipc.redo).toHaveBeenCalledTimes(1));
  });

  it("ignores the shortcut while typing in a form control", async () => {
    await renderOpenApp();
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();
    fireEvent.keyDown(input, { key: "z", ctrlKey: true });
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.undo).not.toHaveBeenCalled();
    input.remove();
  });

  it("does nothing with no project open", async () => {
    render(<App />);
    fireEvent.keyDown(window, { key: "z", ctrlKey: true });
    await new Promise((r) => setTimeout(r, 20));
    expect(ipc.undo).not.toHaveBeenCalled();
  });
});

describe("App dirty indicator", () => {
  it("marks the Save button when the project has unsaved changes", async () => {
    vi.mocked(ipc.getUndoState).mockResolvedValue({
      canUndo: true,
      canRedo: false,
      dirty: true,
    });
    await renderOpenApp();
    await waitFor(() => expect(screen.getByTitle("Unsaved changes")).toBeTruthy());
  });

  it("shows no dirty marker when clean", async () => {
    await renderOpenApp();
    await new Promise((r) => setTimeout(r, 20));
    expect(screen.queryByTitle("Unsaved changes")).toBeNull();
  });
});
