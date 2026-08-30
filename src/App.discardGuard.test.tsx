import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as library from "./api/library";
import type { Song } from "./bindings";

// F49: `confirmDiscard` in App.tsx guards the SAME unsaved work as the
// window-close guard (F47, App.closeGuard.test.tsx), on the other path — the
// New and Open buttons. Until this file its dirty branch was exercised by
// nothing: the suite's only `dirty: true` is App.test.tsx's dirty-indicator
// test, which does open a project but never clicks New or Open afterwards, so
// only `if (!dirtyRef.current) return true` ever ran. `ask` was mocked in
// App.test.tsx and App.projectSwitch.test.tsx purely to keep the effect quiet
// and asserted in neither.
//
// The point of the guard is that the ACTION IS ABANDONED, not that a promise
// resolved false, so every case below asserts on the caller's own observable
// consequence: the New Project dialog appearing or not, the directory picker
// and `open_project` being reached or not. Each assertion was proven red by
// breaking the guard in App.tsx; the commit message carries the exact failure
// text of each break and which cases stayed green as controls.
//
// The Tauri window module is mocked PER FILE (inert), matching App.test.tsx
// and App.closeGuard.test.tsx: this file is about the discard guard, and a
// real `getCurrentWindow()` here would print a stack per test. See the header
// of App.closeGuard.test.tsx for why the mock is not global.
const tauri = vi.hoisted(() => ({
  ask: vi.fn(),
  open: vi.fn(),
}));

vi.mock("./api/ipc");
vi.mock("./api/library");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: tauri.open,
  save: vi.fn(),
  ask: tauri.ask,
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

/**
 * The toolbar button with this label.
 *
 * Scoped to the top bar because the LibraryPanel — which only renders while a
 * project is open, i.e. in exactly the dirty cases this file cares about —
 * carries its own "Import" button, and a bare getByText("Import") matched both.
 */
function topBarButton(label: string): HTMLElement {
  const matches = screen
    .getAllByRole("button", { name: label })
    .filter((b) => b.closest("[class*='topBar']") !== null);
  expect(matches, `expected exactly one top-bar "${label}" button`).toHaveLength(1);
  return matches[0];
}

/**
 * Render App with a project open, dirty or clean.
 *
 * `dirtyRef` only ever follows `undoState`, and `undoState` is only polled
 * while a project is open — a dirty App that never opened a project is not
 * reachable in the product either. Same helper shape as
 * App.closeGuard.test.tsx, deliberately: both guards read the same ref.
 *
 * The first Open click happens while still clean, so in a healthy tree it
 * passes the guard's early-out without asking; the call log is cleared before
 * returning so each test measures only its own action. See the note below.
 */
async function renderWithProject(dirty: boolean) {
  vi.mocked(ipc.getUndoState).mockResolvedValue({
    canUndo: dirty,
    canRedo: false,
    dirty,
  });
  const utils = render(<App />);
  fireEvent.click(topBarButton("Open"));
  await waitFor(() => expect(ipc.openProject).toHaveBeenCalledTimes(1));
  if (dirty) {
    // The dirty marker is the observable proof that the poll landed and
    // dirtyRef is true; waiting on it instead of a timeout keeps the test
    // from asserting against a ref that has not caught up yet.
    await screen.findByTitle("Unsaved changes");
  } else {
    await act(async () => {});
    expect(screen.queryByTitle("Unsaved changes")).toBeNull();
  }
  // Forget the setup click so every count below is about the ONE action the
  // test then performs. Only the call log is cleared, not the resolved value.
  //
  // This is why the helper does not also assert `ask` was never called here:
  // that tripwire fired for all seven cases under a single break (removing
  // the early-out), and a break that reds an entire file teaches nothing
  // about which behaviour it broke. With the counts scoped per action, that
  // break reds exactly the three no-unsaved-changes cases and leaves the rest
  // green — which is what a control is for.
  tauri.ask.mockClear();
  return utils;
}

/** The New Project dialog is on screen (its own heading, not the toolbar). */
function newProjectDialogOpen() {
  return screen.queryByRole("heading", { name: "New Project" }) !== null;
}

/** Settle the NewProjectDialog's `listDrivers` effect before the test ends. */
async function settleDialogEffects() {
  await act(async () => {});
}

beforeEach(() => {
  vi.clearAllMocks();
  tauri.ask.mockResolvedValue(true);
  tauri.open.mockResolvedValue("/tmp/proj");
  vi.mocked(ipc.openProject).mockResolvedValue(song);
  vi.mocked(ipc.closeProject).mockResolvedValue(undefined);
  vi.mocked(ipc.listTracks).mockResolvedValue([]);
  // The NewProjectDialog loads this in a mount effect and refuses to create
  // without a driver; one entry is enough for the dialog to settle.
  vi.mocked(ipc.listDrivers).mockResolvedValue([
    { id: "flamedriver", name: "Flamedriver" },
  ]);
  vi.mocked(ipc.getUndoState).mockResolvedValue({
    canUndo: false,
    canRedo: false,
    dirty: false,
  });
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

afterEach(() => {
  vi.restoreAllMocks();
});

describe("F49: the unsaved-changes guard on New Project", () => {
  it("asks before discarding, and creates once the user agrees", async () => {
    await renderWithProject(true);
    fireEvent.click(topBarButton("New"));

    await waitFor(() => expect(tauri.ask).toHaveBeenCalledTimes(1));
    expect(tauri.ask).toHaveBeenCalledWith(
      "You have unsaved changes. Create a new project and discard them?",
      { title: "Unsaved changes" },
    );
    await waitFor(() => expect(newProjectDialogOpen()).toBe(true));
    await settleDialogEffects();
  });

  it("abandons New Project when the user declines", async () => {
    tauri.ask.mockResolvedValue(false);
    await renderWithProject(true);
    fireEvent.click(topBarButton("New"));

    await waitFor(() => expect(tauri.ask).toHaveBeenCalledTimes(1));
    // The action itself is what must be abandoned, not merely the promise.
    await act(async () => {});
    expect(newProjectDialogOpen()).toBe(false);
  });

  // CONTROL for every break that removes or neuters the ask: a clean New must
  // still open the dialog with no prompt at all, so "always ask" and "never
  // proceed" are not passing fixes.
  it("opens New Project with no prompt when there are no unsaved changes", async () => {
    await renderWithProject(false);
    fireEvent.click(topBarButton("New"));

    await waitFor(() => expect(newProjectDialogOpen()).toBe(true));
    expect(tauri.ask).not.toHaveBeenCalled();
    await settleDialogEffects();
  });
});

describe("F49: the unsaved-changes guard on Open Project", () => {
  it("asks before discarding, and opens once the user agrees", async () => {
    await renderWithProject(true);
    fireEvent.click(topBarButton("Open"));

    await waitFor(() => expect(tauri.ask).toHaveBeenCalledTimes(1));
    expect(tauri.ask).toHaveBeenCalledWith(
      "You have unsaved changes. Open another project and discard them?",
      { title: "Unsaved changes" },
    );
    // Past the guard: the directory picker runs and a second project loads.
    await waitFor(() => expect(ipc.openProject).toHaveBeenCalledTimes(2));
    expect(tauri.open).toHaveBeenCalledTimes(2);
  });

  it("abandons Open Project when the user declines", async () => {
    tauri.ask.mockResolvedValue(false);
    await renderWithProject(true);
    fireEvent.click(topBarButton("Open"));

    await waitFor(() => expect(tauri.ask).toHaveBeenCalledTimes(1));
    await act(async () => {});
    // Not even the directory picker: the user is never asked where to open
    // a project they just said they did not want to open.
    expect(tauri.open).toHaveBeenCalledTimes(1);
    expect(ipc.openProject).toHaveBeenCalledTimes(1);
    // The open project was not closed out from under the unsaved work.
    expect(ipc.closeProject).not.toHaveBeenCalled();
  });

  // CONTROL, mirror of the New Project one: a clean Open must reach the
  // picker with no prompt.
  it("opens with no prompt when there are no unsaved changes", async () => {
    await renderWithProject(false);
    fireEvent.click(topBarButton("Open"));

    await waitFor(() => expect(ipc.openProject).toHaveBeenCalledTimes(2));
    expect(tauri.ask).not.toHaveBeenCalled();
  });
});

describe("F49: Import is not behind the discard guard", () => {
  // Documents today's behaviour, it does not endorse it. `onImport` in
  // App.tsx is a bare `() => setShowImportDialog(true)` — it never calls
  // confirmDiscard — yet a completed import replaces the open project exactly
  // as New and Open do (see handleImported's own comment: "same boundary as
  // open/new"). So the third door onto the same unsaved work has no guard on
  // it. Whether it should is the owner's call, not this parcel's; if the
  // guard is added, this case is the one to rewrite. See the parcel report.
  //
  // It is also THE control of this file: it is the one case that touches no
  // part of confirmDiscard, so it stayed green under every break applied to
  // the guard. A break that takes this case with it broke something else.
  it("opens the Import dialog with unsaved changes and no prompt", async () => {
    await renderWithProject(true);
    fireEvent.click(topBarButton("Import"));

    await screen.findByRole("heading", { name: "Import Song" });
    expect(tauri.ask).not.toHaveBeenCalled();
  });
});
