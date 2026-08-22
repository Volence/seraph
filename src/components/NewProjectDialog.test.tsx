import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { NewProjectDialog } from "./NewProjectDialog";
import * as ipc from "../api/ipc";
import { getRecentLocations, rememberLocation } from "../utils/recentLocations";

vi.mock("../api/ipc");
const openMock = vi.fn();
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: (...args: unknown[]) => openMock(...args) }));

const meta = {
  name: "My Song",
  tempo: 120,
  timeSignature: [4, 4] as [number, number],
  ticksPerBeat: 96,
  driverId: "flamedriver",
};

describe("NewProjectDialog recent locations", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.clear();
    openMock.mockReset();
    vi.mocked(ipc.listDrivers).mockResolvedValue([{ id: "flamedriver", name: "Flamedriver" }]);
    vi.mocked(ipc.createProject).mockResolvedValue(undefined);
    vi.mocked(ipc.getProjectInfo).mockResolvedValue(meta);
  });

  function locationInput(): HTMLInputElement {
    return screen.getByPlaceholderText("/path/to/projects") as HTMLInputElement;
  }

  it("prefills Location with the most recent remembered location", () => {
    rememberLocation("/home/me/older");
    rememberLocation("/home/me/songs");
    render(<NewProjectDialog onClose={() => {}} onCreated={() => {}} />);
    expect(locationInput().value).toBe("/home/me/songs");
  });

  it("leaves Location empty when nothing is remembered", () => {
    render(<NewProjectDialog onClose={() => {}} onCreated={() => {}} />);
    expect(locationInput().value).toBe("");
  });

  it("shows remembered locations as suggestions on focus and fills on click", () => {
    rememberLocation("/home/me/older");
    rememberLocation("/home/me/songs");
    render(<NewProjectDialog onClose={() => {}} onCreated={() => {}} />);
    fireEvent.focus(locationInput());
    const older = screen.getByRole("option", { name: "/home/me/older" });
    expect(screen.getByRole("option", { name: "/home/me/songs" })).toBeInTheDocument();
    fireEvent.mouseDown(older);
    expect(locationInput().value).toBe("/home/me/older");
  });

  it("remembers the location after a successful create", async () => {
    const onCreated = vi.fn();
    render(<NewProjectDialog onClose={() => {}} onCreated={onCreated} />);
    // Wait for the driver list so driverId is set before Create validates it.
    await screen.findByText("Flamedriver");
    fireEvent.change(screen.getByPlaceholderText("My Song"), { target: { value: "Tune" } });
    fireEvent.change(locationInput(), { target: { value: "/home/me/songs" } });
    fireEvent.click(screen.getByText("Create"));
    await waitFor(() => expect(onCreated).toHaveBeenCalled());
    expect(getRecentLocations()).toEqual(["/home/me/songs"]);
  });

  it("starts the Browse dialog from the most recent location", async () => {
    rememberLocation("/home/me/songs");
    openMock.mockResolvedValue(null);
    render(<NewProjectDialog onClose={() => {}} onCreated={() => {}} />);
    fireEvent.click(screen.getByText("Browse"));
    await waitFor(() =>
      expect(openMock).toHaveBeenCalledWith(
        expect.objectContaining({ directory: true, defaultPath: "/home/me/songs" })
      )
    );
  });
});
