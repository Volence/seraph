import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { FmEditor } from "./FmEditor";
import * as ipc from "../api/ipc";
import * as lib from "../api/library";
import type { FmInstrument, FmOperator } from "../types/model";

vi.mock("../api/ipc");
vi.mock("../api/library");

const op: FmOperator = {
  detune: 0, multiple: 1, rateScale: 0, attackRate: 31, ampMod: false,
  d1r: 0, d2r: 0, sustainLevel: 0, releaseRate: 15, totalLevel: 0,
};

const inst: FmInstrument = {
  id: "fm-1",
  name: "Lead",
  algorithm: 4,
  feedback: 3,
  operators: [op, op, op, op],
  metadata: { category: "", author: "", tags: [] },
};

describe("FmEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(ipc.listFmInstruments).mockResolvedValue([inst]);
    vi.mocked(lib.librarySaveFromProject).mockResolvedValue("sha256:abcd");
  });

  it("Save to library saves this instrument and fires the refresh callback", async () => {
    const onSaved = vi.fn();
    render(<FmEditor instrumentId="fm-1" onSavedToLibrary={onSaved} />);

    fireEvent.click(await screen.findByText("Save to library"));

    await waitFor(() =>
      expect(vi.mocked(lib.librarySaveFromProject)).toHaveBeenCalledWith("fm", "fm-1", null, [])
    );
    await waitFor(() => expect(onSaved).toHaveBeenCalledTimes(1));
  });
});
