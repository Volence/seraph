import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { LibraryPanel } from "./LibraryPanel";
import * as lib from "../api/library";
import type { LibraryEntryDetail, LibraryListEntry } from "../api/library";
import type { FmInstrument, FmOperator } from "../bindings";

vi.mock("../api/library");
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

const entry = (hash: string, name: string, kind: LibraryListEntry["kind"]): LibraryListEntry => ({
  hash,
  name,
  kind,
  game: "Sonic 2",
  tags: ["lead"],
  favorite: false,
  rootLabel: "Seraph Pack",
});

const fmOp: FmOperator = {
  detune: 1, multiple: 2, rateScale: 0, attackRate: 31, ampMod: false,
  d1r: 5, d2r: 3, sustainLevel: 2, releaseRate: 7, totalLevel: 20,
};

const fmDetail = (name: string): LibraryEntryDetail => ({
  name,
  game: "Sonic 2",
  tags: ["lead"],
  instrument: {
    type: "fm",
    instrument: {
      id: "lib", name, algorithm: 4, feedback: 3,
      operators: [fmOp, fmOp, fmOp, fmOp] as FmInstrument["operators"],
      metadata: { category: "", author: "", tags: [] },
    },
  },
});

describe("LibraryPanel", () => {
  beforeEach(() => {
    vi.clearAllMocks(); // call counts must not leak across tests
    vi.mocked(lib.libraryGames).mockResolvedValue(["Sonic 2"]);
    vi.mocked(lib.libraryList).mockResolvedValue([entry("sha256:aaaa", "EHZ Lead", "fm"), entry("sha256:bbbb", "Env 3", "psg")]);
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
    expect(vi.mocked(lib.libraryAudition)).toHaveBeenCalledWith("sha256:aaaa", 60);
    fireEvent.mouseUp(name);
    await waitFor(() =>
      expect(vi.mocked(lib.libraryStopAudition)).toHaveBeenCalledTimes(1)
    );
    fireEvent.mouseLeave(name);
    expect(vi.mocked(lib.libraryStopAudition)).toHaveBeenCalledTimes(1);
  });

  it("selecting an entry fetches and renders the detail card; PianoKeys auditions at the played note", async () => {
    vi.mocked(lib.libraryGetEntry).mockResolvedValue(fmDetail("EHZ Lead"));
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    const name = await screen.findByText("EHZ Lead");

    fireEvent.click(name);
    await waitFor(() =>
      expect(vi.mocked(lib.libraryGetEntry)).toHaveBeenCalledWith("sha256:aaaa")
    );
    await screen.findByText("ALG 4 · FB 3");
    expect(screen.getByText("MUL")).toBeTruthy();

    // PianoKeys renders plain divs; vitest maps CSS-module classes to
    // stable names containing the original identifier, so match by
    // substring. Second white key at the default C4 octave = semitone 2 =
    // MIDI 62.
    const keys = document.querySelectorAll('[class*="whiteKey"]');
    expect(keys.length).toBe(7);
    fireEvent.mouseDown(keys[1]);
    expect(vi.mocked(lib.libraryAudition)).toHaveBeenCalledWith("sha256:aaaa", 62);
    fireEvent.mouseUp(window);
    await waitFor(() => expect(vi.mocked(lib.libraryStopAudition)).toHaveBeenCalledTimes(1));
  });

  it("rows are draggable and set the library-entry drag payload", async () => {
    render(<LibraryPanel onInstrumentAdded={() => {}} />);
    const name = await screen.findByText("EHZ Lead");
    const row = name.closest("[draggable]");
    expect(row).not.toBeNull();

    const setData = vi.fn();
    fireEvent.dragStart(row!, { dataTransfer: { setData, effectAllowed: "" } });
    expect(setData).toHaveBeenCalledWith(
      "application/x-seraph-library-entry",
      JSON.stringify({ hash: "sha256:aaaa", kind: "fm" }),
    );
    // Kind-suffixed type for dragover compatibility cues.
    expect(setData).toHaveBeenCalledWith("application/x-seraph-library-entry-fm", "fm");
  });
});
