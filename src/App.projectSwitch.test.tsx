import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as lib from "./api/library";
import {
  copyNotes,
  copyRegions,
  getNoteClipboard,
  getRegionClipboard,
  lastCopiedKind,
  resetClipboardForTest,
} from "./utils/clipboard";
import type { Note, SongMetadata, Track } from "./types/model";

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
        durationTicks: meta.ticksPerBeat * meta.timeSignature[0],
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

/**
 * Fill BOTH clipboard slots with payloads that name ids of the project that
 * is open right now: a note carrying a per-note voice id (add_note validates
 * an explicit instrumentId strictly) and a region naming a track/region id.
 * Neither means anything in a different project.
 */
function copyProjectScopedPayloads() {
  const notes: Note[] = [
    { tick: 0, pitch: 60, velocity: 100, durationTicks: 240, instrumentId: "fm-instrument-of-project-A" },
  ];
  copyNotes(notes, [0], "fm");
  copyRegions([
    {
      trackId: "track-1",
      regionId: "region-1",
      payload: { startTick: 0, durationTicks: 1920, notes },
    },
  ]);
}

/** Every clipboard slot is empty and Ctrl+V has nothing to arbitrate. */
function expectClipboardEmpty() {
  expect(getNoteClipboard()).toEqual([]);
  expect(getRegionClipboard()).toEqual([]);
  expect(lastCopiedKind()).toBeNull();
}

/** Renders App and drives the NewProjectDialog until a project is open. */
async function openProjectViaDialog() {
  const utils = render(<App />);
  fireEvent.click(await screen.findByRole("button", { name: "New Project" }));
  await createFromOpenNewProjectDialog();
  // Project open: the arrangement shows the track header.
  await screen.findByText("Bass Lane");
  return utils;
}

/** Fills in and submits an already-open NewProjectDialog. */
async function createFromOpenNewProjectDialog() {
  fireEvent.change(await screen.findByPlaceholderText("My Song"), {
    target: { value: "Test Song" },
  });
  fireEvent.change(screen.getByPlaceholderText("/path/to/projects"), {
    target: { value: "/tmp/projects" },
  });
  // Wait for the PRECONDITION, then assert the CONSEQUENCE without polling.
  //
  // The dialog loads its driver list in an effect, and handleCreate refuses
  // with "Select a driver" — never reaching ipc.createProject at all — while
  // driverId is still empty. Clicking Create first therefore sets up a wait
  // that can never succeed: the old
  //   `await waitFor(() => expect(ipc.createProject).toHaveBeenCalled())`
  // spent its entire 1000ms default budget and then reported "expected
  // vi.fn() to be called at least once", which reads as the product failing
  // to create a project rather than as this test having jumped the gun. That
  // was THE flake here (the failing run's own 1342ms duration is the tell),
  // and it only ever bit under a loaded full-suite run, where React defers
  // the dialog's passive effect past the point this helper clicks.
  //
  // Drained, not polled — the same reasoning the sibling cases in
  // ArrangementView.test.tsx already carry. listDrivers is a mocked promise
  // with no timers, so one async act() runs the effect and settles it, with
  // no timeout budget for a loaded box to blow through.
  await act(async () => {});
  expect(screen.getByRole("option", { name: "Flamedriver (S3K)" })).toBeInTheDocument();
  fireEvent.click(screen.getByText("Create"));
  // handleCreate calls ipc.createProject synchronously, before its first
  // await, so there is nothing to poll for: this either holds right now or
  // names a real defect immediately instead of a minute later.
  expect(ipc.createProject).toHaveBeenCalled();
}

beforeEach(() => {
  vi.clearAllMocks();
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
  vi.mocked(ipc.importSong).mockResolvedValue({
    projectDir: "/tmp/projects/imported",
    metadata: meta,
    trackCount: 1,
    instrumentCount: 0,
    warnings: [],
  });
  vi.mocked(ipc.listTracks).mockResolvedValue([makeTrack()]);
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

// The clipboard is module state that deliberately survives region and view
// switches — but every id on it is scoped to ONE project. Carried across a
// project switch, a paste either fails a backend validation or resolves an id
// that now belongs to something else.
describe("the module clipboard does not survive a project switch", () => {
  it("New Project clears it", async () => {
    await openProjectViaDialog();
    copyProjectScopedPayloads();
    expect(lastCopiedKind()).toBe("regions");

    fireEvent.click(screen.getByRole("button", { name: "New" }));
    await createFromOpenNewProjectDialog();
    await act(async () => {});

    expectClipboardEmpty();
  });

  it("Open Project clears it", async () => {
    await openProjectViaDialog();
    copyProjectScopedPayloads();

    fireEvent.click(screen.getByRole("button", { name: "Open" }));
    // Drained, not polled: handleOpenProject is a chain of mocked promises
    // (confirmDiscard, the dialog import, open, closeProject, openProject)
    // with no timers, so one async act() settles it. Polling for the call
    // instead would report a chain that broke as a call that never happened.
    await act(async () => {});
    expect(ipc.openProject).toHaveBeenCalled();

    expectClipboardEmpty();
  });

  it("Import clears it", async () => {
    await openProjectViaDialog();
    copyProjectScopedPayloads();

    // "Import" is an ambiguous label: the TopBar's project import and the
    // LibraryPanel's instrument-file import share it. TopBar renders first.
    fireEvent.click(screen.getAllByRole("button", { name: "Import" })[0]);
    await screen.findByText("Import Song");
    // Two Browse buttons in the flamedriver form (source, DAC dir) plus the
    // project-location one; the mocked dialog answers all of them.
    // One at a time: each handler does its own `await import(
    // "@tauri-apps/plugin-dialog")`, and firing them in the same tick races
    // the module mock (the losers reach the real plugin and reject).
    for (const btn of screen.getAllByText("Browse")) {
      fireEvent.click(btn);
      await act(async () => {});
    }
    // The loop above already drained each Browse, so this precondition holds
    // now or not at all — handleImport refuses with "Select a source file"
    // while it is unset, which would surface as importSong "never called".
    expect(screen.getByPlaceholderText("Select .asm file")).toHaveValue("/tmp/projects");
    // Two "Import" buttons exist now (TopBar's and the dialog's submit);
    // the dialog's is the later one in the tree.
    const importButtons = screen.getAllByRole("button", { name: "Import" });
    fireEvent.click(importButtons[importButtons.length - 1]);
    // Drained, not polled: handleImport awaits closeProject before importSong,
    // all mocked promises with no timers.
    await act(async () => {});
    expect(ipc.importSong).toHaveBeenCalled();

    expectClipboardEmpty();
  });
});
