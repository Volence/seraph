import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { PsgEditor } from "./PsgEditor";
import * as ipc from "../api/ipc";
import * as lib from "../api/library";
import { whenReloadsSettled, resetLiveReloadForTests } from "../utils/liveReload";
import type { PsgInstrument } from "../types/model";

vi.mock("../api/ipc");
vi.mock("../api/library");

const inst: PsgInstrument = {
  id: "psg-1",
  name: "Blip",
  volumeSequence: [15, 12, 8],
  loopPoint: null,
  silenceOnEnd: false,
  noiseMode: null,
  metadata: { category: "", author: "", tags: [] },
};

describe("PsgEditor", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    resetLiveReloadForTests();
    vi.mocked(ipc.listPsgInstruments).mockResolvedValue([inst]);
    vi.mocked(ipc.updatePsgInstrument).mockResolvedValue(undefined);
    vi.mocked(ipc.reloadSequence).mockResolvedValue(undefined);
    vi.mocked(lib.librarySaveFromProject).mockResolvedValue("sha256:abcd");
  });

  // Audit F3 named FmEditor; PsgEditor had the same hole — `update` called
  // updatePsgInstrument and nothing else, so an envelope edit was inaudible
  // until stop/play. The audibility of the reload is proved by the
  // rendered-audio tests in src-tauri/src/audio/live_edit_audibility.rs.
  describe("live audibility (F3)", () => {
    it("an envelope-length edit reloads the running sequence", async () => {
      render(<PsgEditor instrumentId="psg-1" />);

      fireEvent.click(await screen.findByText("+ Tick"));

      await waitFor(() =>
        expect(ipc.updatePsgInstrument).toHaveBeenCalledWith(
          "psg-1",
          expect.objectContaining({ volumeSequence: [15, 12, 8, 0] }),
        ),
      );
      await whenReloadsSettled();
      expect(ipc.reloadSequence).toHaveBeenCalled();
    });

    it("a noise-mode edit reloads the running sequence", async () => {
      render(<PsgEditor instrumentId="psg-1" />);

      fireEvent.click(await screen.findByText("White"));

      await waitFor(() => expect(ipc.updatePsgInstrument).toHaveBeenCalledTimes(1));
      await whenReloadsSettled();
      expect(ipc.reloadSequence).toHaveBeenCalled();
    });

    it("a failed edit does not reload (the backend rejected the change)", async () => {
      vi.mocked(ipc.updatePsgInstrument).mockRejectedValueOnce(new Error("nope"));
      render(<PsgEditor instrumentId="psg-1" />);

      fireEvent.click(await screen.findByText("+ Tick"));

      await waitFor(() => expect(ipc.updatePsgInstrument).toHaveBeenCalledTimes(1));
      await whenReloadsSettled();
      expect(ipc.reloadSequence).not.toHaveBeenCalled();
    });
  });
});
