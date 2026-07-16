import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { LibraryPanel } from "./LibraryPanel";
import * as lib from "../api/library";
import type { LibraryListEntry } from "../api/library";

vi.mock("../api/library");
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const entry = (name: string, kind: LibraryListEntry["kind"]): LibraryListEntry => ({
  hash: name,
  name,
  kind,
  game: "Sonic 2",
  tags: ["lead"],
  favorite: false,
  rootLabel: "Seraph Pack",
});

describe("LibraryPanel", () => {
  beforeEach(() => {
    vi.mocked(lib.libraryGames).mockResolvedValue(["Sonic 2"]);
    vi.mocked(lib.libraryList).mockResolvedValue([entry("EHZ Lead", "fm"), entry("Env 3", "psg")]);
    vi.mocked(lib.libraryWarnings).mockResolvedValue([]);
    vi.mocked(lib.libraryAudition).mockResolvedValue(undefined);
    vi.mocked(lib.libraryStopAudition).mockResolvedValue(undefined);
  });

  it("renders entries and re-queries with kind filter", async () => {
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    await screen.findByText("EHZ Lead");
    fireEvent.click(screen.getByText("PSG"));
    await waitFor(() =>
      expect(vi.mocked(lib.libraryList)).toHaveBeenLastCalledWith(
        expect.objectContaining({ kind: "psg" })
      )
    );
  });

  it("passes search text to the backend filter", async () => {
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    await screen.findByText("EHZ Lead");
    fireEvent.change(screen.getByPlaceholderText(/Search/), { target: { value: "bass" } });
    await waitFor(() =>
      expect(vi.mocked(lib.libraryList)).toHaveBeenLastCalledWith(
        expect.objectContaining({ text: "bass" })
      )
    );
  });

  it("renders scan warnings in a dismissible strip", async () => {
    vi.mocked(lib.libraryWarnings).mockResolvedValue([
      "quarantined bad.json: invalid operator values",
    ]);
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    await screen.findByText("quarantined bad.json: invalid operator values");
    expect(screen.getByText(/1 library warning/)).toBeTruthy();
    fireEvent.click(screen.getByTitle("Dismiss"));
    await waitFor(() =>
      expect(screen.queryByText("quarantined bad.json: invalid operator values")).toBeNull()
    );
  });

  it("does not stop audition on hover-through; stops exactly once after a real audition", async () => {
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    const name = await screen.findByText("EHZ Lead");

    // Bare mouseLeave/mouseUp with no prior mouseDown (cursor dragged across
    // the list) must NOT fire the stop command.
    fireEvent.mouseLeave(name);
    fireEvent.mouseUp(name);
    expect(vi.mocked(lib.libraryStopAudition)).not.toHaveBeenCalled();

    // Real hold-to-audition: down starts, up stops once; a following
    // mouseLeave does not stop again.
    fireEvent.mouseDown(name);
    expect(vi.mocked(lib.libraryAudition)).toHaveBeenCalledWith("EHZ Lead", 60);
    fireEvent.mouseUp(name);
    await waitFor(() =>
      expect(vi.mocked(lib.libraryStopAudition)).toHaveBeenCalledTimes(1)
    );
    fireEvent.mouseLeave(name);
    expect(vi.mocked(lib.libraryStopAudition)).toHaveBeenCalledTimes(1);
  });
});
