import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as library from "./api/library";
import type { Song } from "./bindings";

// F48: BOTH unsaved-changes guards could fail in complete silence.
//
// The close-request effect's `try` covers only REGISTRATION -- the dynamic
// import, `getCurrentWindow()`, and the `await` on `onCloseRequested`. The
// handler body runs later, outside it. So when the user actually closed the
// window and `ask()` or `win.destroy()` rejected, nothing happened visibly, no
// console line was emitted at all, and -- because `event.preventDefault()` had
// already fired -- the window was left unclosable. `confirmDiscard` had the
// mirror-image problem with no try/catch at all: an unavailable dialog rejected,
// the caller rejected with it, and the New/Open button silently did nothing.
//
// This file covers ONLY that the failure is now reported. It deliberately does
// NOT pin what should happen to the close itself: whether a failed dialog should
// let the window close (risking unsaved work) or keep it open (trapping the
// user) is a product decision, parked for the owner. The cases below assert
// today's outcome where they touch it, and say so, so that whichever way the
// owner rules, the test that has to change is obvious.
//
// This is NOT reachable through a missing permission: src-tauri/capabilities/
// default.json does grant `core:window:allow-destroy`. It is a latent
// robustness gap reachable by other rejection causes.
const tauri = vi.hoisted(() => ({
  getCurrentWindow: vi.fn(),
  onCloseRequested: vi.fn(),
  destroy: vi.fn(),
  unlisten: vi.fn(),
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
  getCurrentWindow: tauri.getCurrentWindow,
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

/** The one argument the guard uses off Tauri's CloseRequestedEvent. */
function closeEvent() {
  return { preventDefault: vi.fn() };
}

/** The callback App handed to `win.onCloseRequested`. */
function registeredHandler() {
  expect(tauri.onCloseRequested).toHaveBeenCalledTimes(1);
  return tauri.onCloseRequested.mock.calls[0][0] as (
    event: ReturnType<typeof closeEvent>,
  ) => Promise<void>;
}

/**
 * Fire a close request the way Tauri does: invoke the handler and DROP the
 * promise it returns. That is the whole shape of this bug -- a rejection there
 * had nowhere to go and became an unhandled rejection nobody saw. Awaiting it
 * into the test would rethrow it here and quietly test something the product
 * never does.
 */
async function fireCloseRequest(event: ReturnType<typeof closeEvent>) {
  const settled = Promise.resolve(registeredHandler()(event)).catch(() => {});
  await act(async () => {
    await settled;
  });
}

/** The toolbar button with this label (see App.discardGuard.test.tsx). */
function topBarButton(label: string): HTMLElement {
  const matches = screen
    .getAllByRole("button", { name: label })
    .filter((b) => b.closest("[class*='topBar']") !== null);
  expect(matches, `expected exactly one top-bar "${label}" button`).toHaveLength(1);
  return matches[0];
}

/**
 * Render App with a project open and dirty, so both guards are live. Same
 * reasoning as App.closeGuard.test.tsx: dirtyRef only follows undoState, and
 * undoState is only polled while a project is open.
 */
async function renderDirty() {
  vi.mocked(ipc.getUndoState).mockResolvedValue({
    canUndo: true,
    canRedo: false,
    dirty: true,
  });
  const utils = render(<App />);
  fireEvent.click(topBarButton("Open"));
  await waitFor(() => expect(ipc.openProject).toHaveBeenCalledTimes(1));
  await screen.findByTitle("Unsaved changes");
  tauri.ask.mockClear();
  return utils;
}

/**
 * Spy on console.error and let it CALL THROUGH. The whole point of this parcel
 * is that these failures become visible; silencing them in the very tests that
 * assert them would be a strange way to prove it. The lines these cases print
 * during a run are expected output, not noise.
 */
function watchConsoleError() {
  return vi.spyOn(console, "error");
}

beforeEach(() => {
  vi.clearAllMocks();
  tauri.getCurrentWindow.mockReturnValue({
    onCloseRequested: tauri.onCloseRequested,
    destroy: tauri.destroy,
  });
  tauri.onCloseRequested.mockResolvedValue(tauri.unlisten);
  tauri.ask.mockResolvedValue(true);
  tauri.destroy.mockResolvedValue(undefined);
  tauri.open.mockResolvedValue("/tmp/proj");
  vi.mocked(ipc.openProject).mockResolvedValue(song);
  vi.mocked(ipc.closeProject).mockResolvedValue(undefined);
  vi.mocked(ipc.listTracks).mockResolvedValue([]);
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

describe("F48: a failure inside the close handler is reported", () => {
  it("reports a rejected quit dialog instead of failing silently", async () => {
    const error = watchConsoleError();
    const boom = new Error("dialog plugin unavailable");
    tauri.ask.mockRejectedValue(boom);
    await renderDirty();

    const event = closeEvent();
    await fireCloseRequest(event);

    expect(error).toHaveBeenCalledWith("close-confirm failed:", boom);
    // Today's outcome, recorded, not endorsed: the close was already stopped
    // and nothing lets it through, so the window stays open. Whether that is
    // right -- versus closing and risking the unsaved work -- is the owner's
    // decision, parked by this parcel. If it is ruled the other way, this is
    // the line to change.
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(tauri.destroy).not.toHaveBeenCalled();
  });

  it("reports a rejected window.destroy instead of failing silently", async () => {
    const error = watchConsoleError();
    const boom = new Error("destroy rejected");
    tauri.destroy.mockRejectedValue(boom);
    await renderDirty();

    await fireCloseRequest(closeEvent());

    // The user said quit, the destroy failed, and the window is still here.
    // Without this report that state is indistinguishable from a frozen app.
    expect(tauri.ask).toHaveBeenCalledTimes(1);
    expect(tauri.destroy).toHaveBeenCalledTimes(1);
    expect(error).toHaveBeenCalledWith("close-confirm failed:", boom);
  });

  // CONTROL. A working close must stay silent and must still destroy the
  // window: a catch that swallows the happy path, or reports on every close,
  // is not a passing fix. Green before this parcel and after it.
  it("reports nothing and still destroys the window on a working close", async () => {
    const error = watchConsoleError();
    await renderDirty();

    await fireCloseRequest(closeEvent());

    expect(tauri.destroy).toHaveBeenCalledTimes(1);
    expect(error).not.toHaveBeenCalled();
  });
});

describe("F48: a failure inside confirmDiscard is reported", () => {
  it("reports a rejected discard dialog on New Project", async () => {
    const error = watchConsoleError();
    const boom = new Error("dialog plugin unavailable");
    await renderDirty();
    tauri.ask.mockRejectedValue(boom);

    fireEvent.click(topBarButton("New"));

    await waitFor(() =>
      expect(error).toHaveBeenCalledWith("discard-confirm failed:", boom),
    );
    // Today's outcome, recorded: the action is abandoned, which is what
    // happened before this parcel too (the rejection propagated and the
    // button did nothing) -- only now it says so. The alternative, treating
    // an unavailable dialog as consent to discard, is the owner's call.
    expect(screen.queryByRole("heading", { name: "New Project" })).toBeNull();
  });

  it("reports a rejected discard dialog on Open Project", async () => {
    const error = watchConsoleError();
    const boom = new Error("dialog plugin unavailable");
    await renderDirty();
    tauri.ask.mockRejectedValue(boom);

    fireEvent.click(topBarButton("Open"));

    await waitFor(() =>
      expect(error).toHaveBeenCalledWith("discard-confirm failed:", boom),
    );
    // Abandoned before the directory picker, as it was before this parcel.
    expect(tauri.open).toHaveBeenCalledTimes(1);
    expect(ipc.openProject).toHaveBeenCalledTimes(1);
  });

  // CONTROL, the confirmDiscard half: a dialog that answers normally must
  // report nothing and must still let the action through.
  it("reports nothing when the discard dialog answers normally", async () => {
    const error = watchConsoleError();
    await renderDirty();

    fireEvent.click(topBarButton("New"));

    await waitFor(() =>
      expect(screen.queryByRole("heading", { name: "New Project" })).not.toBeNull(),
    );
    await act(async () => {});
    expect(error).not.toHaveBeenCalled();
  });
});
