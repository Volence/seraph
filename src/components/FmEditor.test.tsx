import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { FmEditor } from "./FmEditor";
import * as ipc from "../api/ipc";
import * as lib from "../api/library";
import { whenReloadsSettled, resetLiveReloadForTests } from "../utils/liveReload";
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
    resetLiveReloadForTests();
    vi.mocked(ipc.listFmInstruments).mockResolvedValue([inst]);
    vi.mocked(ipc.updateFmInstrument).mockResolvedValue(undefined);
    vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
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

  // Audit F3: knobs used to call updateFmInstrument and nothing else, so a
  // running loop kept playing the pre-edit patch until stop/play. The
  // audibility of the reload itself is proved by the rendered-audio tests in
  // src-tauri/src/audio/live_edit_audibility.rs; this covers the wiring.
  describe("live audibility (F3)", () => {
    it("an operator edit reloads the running sequence", async () => {
      render(<FmEditor instrumentId="fm-1" />);

      const tlSlider = (await screen.findAllByRole("slider"))[0];
      fireEvent.change(tlSlider, { target: { value: "40" } });

      await waitFor(() => expect(ipc.updateFmInstrument).toHaveBeenCalledTimes(1));
      await whenReloadsSettled();
      expect(ipc.reloadSequence).toHaveBeenCalled();
    });

    it("an algorithm/feedback edit reloads the running sequence", async () => {
      render(<FmEditor instrumentId="fm-1" />);
      await screen.findAllByRole("slider");

      // The FB knob commits through the same updateInstrument path as the
      // algorithm diagram; drive it via its double-click numeric entry so the
      // test does not depend on canvas drag geometry (jsdom has no 2D context).
      const fbKnob = screen.getByText("FB").parentElement!;
      fireEvent.mouseDown(fbKnob.querySelector("canvas")!, { detail: 2 });
      const entry = screen.getByDisplayValue("3");
      fireEvent.change(entry, { target: { value: "6" } });
      fireEvent.keyDown(entry, { key: "Enter" });

      await waitFor(() => expect(ipc.updateFmInstrument).toHaveBeenCalledTimes(1));
      await whenReloadsSettled();
      expect(ipc.reloadSequence).toHaveBeenCalled();
    });

    it("a failed edit does not reload (the backend rejected the change)", async () => {
      vi.mocked(ipc.updateFmInstrument).mockRejectedValueOnce(new Error("nope"));
      render(<FmEditor instrumentId="fm-1" />);

      const tlSlider = (await screen.findAllByRole("slider"))[0];
      fireEvent.change(tlSlider, { target: { value: "40" } });

      await waitFor(() => expect(ipc.updateFmInstrument).toHaveBeenCalledTimes(1));
      await whenReloadsSettled();
      expect(ipc.reloadSequence).not.toHaveBeenCalled();
    });
  });
});
