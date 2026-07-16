import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { LibraryPanel } from "./LibraryPanel";
import * as lib from "../api/library";

vi.mock("../api/library");
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const entry = (name: string, kind: string) => ({
  hash: name, name, kind, game: "Sonic 2", tags: ["lead"], favorite: false, rootLabel: "Seraph Pack",
});

describe("LibraryPanel", () => {
  beforeEach(() => {
    vi.mocked(lib.libraryGames).mockResolvedValue(["Sonic 2"]);
    vi.mocked(lib.libraryList).mockResolvedValue([entry("EHZ Lead", "fm"), entry("Env 3", "psg")] as never);
    vi.mocked(lib.libraryWarnings).mockResolvedValue([]);
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
});
