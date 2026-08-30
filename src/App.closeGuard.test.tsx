import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { render, screen, waitFor, act } from "@testing-library/react";
import { fireEvent } from "@testing-library/dom";
import App from "./App";
import * as ipc from "./api/ipc";
import * as library from "./api/library";
import type { Song } from "./bindings";

// F47: the window-close guard (App.tsx, `onCloseRequested`) is the last thing
// standing between the user and losing unsaved work, and until this file it was
// covered by nothing. Every assertion below was proven red by breaking the
// guard in App.tsx and watching this file fail; see the parcel's commit message
// for the exact failure text of each.
//
// The Tauri window module is mocked PER FILE, the way App.test.tsx already
// mocks it, rather than globally in src/test/setup.ts. A global mock would also
// silence the "close-confirm unavailable" lines the other App suites print, but
// it would make every test in the repo run the Tauri-present path, and no test
// anywhere would then exercise the browser-only path that a real `npm run dev`
// in a plain browser takes. Keeping the mock local keeps that choice visible in
// the file that made it.
const tauri = vi.hoisted(() => ({
  getCurrentWindow: vi.fn(),
  onCloseRequested: vi.fn(),
  destroy: vi.fn(),
  unlisten: vi.fn(),
  ask: vi.fn(),
}));

vi.mock("./api/ipc");
vi.mock("./api/library");
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn().mockResolvedValue("/tmp/proj"),
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
 * Render App with a project open, because `dirtyRef` only ever follows
 * `undoState`, and `undoState` is only polled while a project is open. A dirty
 * App that never opened a project is not reachable in the product either.
 */
async function renderWithProject(dirty: boolean) {
  vi.mocked(ipc.getUndoState).mockResolvedValue({
    canUndo: dirty,
    canRedo: false,
    dirty,
  });
  const utils = render(<App />);
  fireEvent.click(screen.getByText("Open"));
  await waitFor(() => expect(ipc.openProject).toHaveBeenCalled());
  if (dirty) {
    // The dirty marker is the observable proof that the poll landed and
    // dirtyRef is true; waiting on it instead of a timeout keeps the test
    // from asserting against a ref that has not caught up yet.
    await screen.findByTitle("Unsaved changes");
  } else {
    await act(async () => {});
    expect(screen.queryByTitle("Unsaved changes")).toBeNull();
  }
  return utils;
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
  vi.mocked(ipc.openProject).mockResolvedValue(song);
  vi.mocked(ipc.listTracks).mockResolvedValue([]);
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

describe("F47: the unsaved-changes guard on window close", () => {
  it("registers one close handler on the current window", async () => {
    await renderWithProject(false);
    expect(tauri.getCurrentWindow).toHaveBeenCalled();
    expect(tauri.onCloseRequested).toHaveBeenCalledTimes(1);
  });

  // CONTROL for the "guard always intercepts" break: a clean close must stay
  // untouched, so a fix that prevents every close is not a passing fix.
  it("lets a close through untouched when there are no unsaved changes", async () => {
    await renderWithProject(false);
    const event = closeEvent();
    await registeredHandler()(event);

    expect(event.preventDefault).not.toHaveBeenCalled();
    expect(tauri.ask).not.toHaveBeenCalled();
    expect(tauri.destroy).not.toHaveBeenCalled();
  });

  it("stops the close and asks the user when there are unsaved changes", async () => {
    await renderWithProject(true);
    const event = closeEvent();
    await registeredHandler()(event);

    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(tauri.ask).toHaveBeenCalledTimes(1);
    expect(tauri.ask).toHaveBeenCalledWith(
      "You have unsaved changes. Quit without saving?",
      { title: "Unsaved changes" },
    );
  });

  it("destroys the window when the user confirms the quit", async () => {
    tauri.ask.mockResolvedValue(true);
    await renderWithProject(true);
    await registeredHandler()(closeEvent());

    expect(tauri.destroy).toHaveBeenCalledTimes(1);
  });

  it("leaves the window alone when the user declines", async () => {
    tauri.ask.mockResolvedValue(false);
    await renderWithProject(true);
    const event = closeEvent();
    await registeredHandler()(event);

    // The close was already stopped; declining must not then let it through.
    expect(event.preventDefault).toHaveBeenCalledTimes(1);
    expect(tauri.destroy).not.toHaveBeenCalled();
  });

  it("unregisters the handler on unmount", async () => {
    const { unmount } = await renderWithProject(false);
    registeredHandler();
    expect(tauri.unlisten).not.toHaveBeenCalled();

    unmount();

    expect(tauri.unlisten).toHaveBeenCalledTimes(1);
  });

  it("tears down a registration that lands after unmount (the cancelled flag)", async () => {
    // Registration is async, so an unmount can beat it. Without the `cancelled`
    // flag the late handler would stay attached to a window whose App is gone.
    let land!: (unlisten: () => void) => void;
    tauri.onCloseRequested.mockReturnValue(
      new Promise<() => void>((resolve) => {
        land = resolve;
      }),
    );

    const { unmount } = render(<App />);
    await waitFor(() => expect(tauri.onCloseRequested).toHaveBeenCalledTimes(1));
    unmount();
    expect(tauri.unlisten).not.toHaveBeenCalled();

    await act(async () => {
      land(tauri.unlisten);
    });

    expect(tauri.unlisten).toHaveBeenCalledTimes(1);
  });

  it("survives a window API that is not there, and says so once", async () => {
    // Documents today's behaviour, it does not endorse it: the catch treats a
    // missing Tauri (a plain browser) and a denied window permission (a real
    // build) identically, and the app then runs with NO close guard at all,
    // with one console line as the only trace. See the parcel report, finding 1.
    // console.warn is spied on but deliberately still calls through: this line
    // stays visible in the run, it is just asserted now instead of accidental.
    const warn = vi.spyOn(console, "warn");
    tauri.getCurrentWindow.mockImplementation(() => {
      throw new TypeError("Cannot read properties of undefined (reading 'metadata')");
    });

    render(<App />);

    await waitFor(() =>
      expect(warn).toHaveBeenCalledWith(
        "close-confirm unavailable:",
        expect.any(TypeError),
      ),
    );
    // No handler was registered, and nothing in the UI says the guard is gone.
    expect(tauri.onCloseRequested).not.toHaveBeenCalled();
    expect(screen.getByText("Open")).toBeTruthy();
  });
});
